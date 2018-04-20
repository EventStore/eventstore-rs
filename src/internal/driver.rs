use core::option::Option;
use std::io::{ Error, ErrorKind };
use std::time::{ Duration, Instant };

use futures::{ Future, Stream, Sink };
use futures::sync::mpsc::Sender;
use tokio_core::reactor::Handle;
use tokio_timer::Timer;
use uuid::Uuid;

use discovery::{ Endpoint, Discovery };
use internal::command::Cmd;
use internal::connection::Connection;
use internal::messaging::Msg;
use internal::operations::Exchange;
use internal::package::Pkg;
use internal::registry::Registry;
use types::{ Credentials, Settings };

#[derive(Copy, Clone)]
enum HeartbeatStatus {
    Init,
    Delay(u32, Instant),
    Timeout(u32, Instant),
}

enum Heartbeat {
    Valid,
    Failure,
}

struct HealthTracker {
    pkg_num: u32,
    state: HeartbeatStatus,
    heartbeat_delay: Duration,
    heartbeat_timeout: Duration,
}

impl HealthTracker {
    fn new(setts: &Settings) -> HealthTracker {
        HealthTracker {
            pkg_num: 0,
            state: HeartbeatStatus::Init,
            heartbeat_delay: setts.heartbeat_delay,
            heartbeat_timeout: setts.heartbeat_timeout,
        }
    }

    fn incr_pkg_num(&mut self) {
        self.pkg_num += 1;
    }

    fn reset(&mut self) {
        self.state = HeartbeatStatus::Init;
    }

    fn manage_heartbeat(&mut self, conn: &Connection) -> Heartbeat {
        match self.state {
            HeartbeatStatus::Init => {
                self.state = HeartbeatStatus::Delay(
                        self.pkg_num, Instant::now());

                Heartbeat::Valid
            },

            HeartbeatStatus::Delay(num, start) => {

                if self.pkg_num != num {
                    self.state = HeartbeatStatus::Delay(
                        self.pkg_num, Instant::now());
                } else {
                    if start.elapsed() >= self.heartbeat_delay {
                        self.state = HeartbeatStatus::Timeout(
                            self.pkg_num, Instant::now());

                        conn.enqueue(Pkg::heartbeat_request());
                    }
                }

                Heartbeat::Valid
            },

            HeartbeatStatus::Timeout(num, start) => {

                if self.pkg_num != num {
                    self.state = HeartbeatStatus::Delay(
                        self.pkg_num, Instant::now());

                    Heartbeat::Valid
                } else {
                    if start.elapsed() >= self.heartbeat_timeout {
                        println!("Closing connection [{}] due to HEARTBEAT TIMEOUT at pkgNum {}.", conn.id, self.pkg_num);

                        Heartbeat::Failure
                    } else {
                        Heartbeat::Valid
                    }
                }
            },
        }
    }
}

#[derive(PartialEq, Eq)]
enum ConnectionState {
    Init,
    Connecting,
    Connected,
    Closed,
}

impl ConnectionState {
    fn is_connected(&self) -> bool {
        *self == ConnectionState::Connected
    }
}

struct InitReq {
    correlation: Uuid,
    started: Instant,
}

impl InitReq {
    fn new(id: Uuid) -> InitReq {
        InitReq {
            correlation: id,
            started: Instant::now(),
        }
    }
}

#[derive(PartialEq, Eq)]
enum Phase {
    Reconnecting,
    EndpointDiscovery,
    Establishing,
    Authentication,
    Identification,
}

struct Attempt {
    started: Instant,
    tries: u32,
}

impl Attempt {
    fn new() -> Attempt {
        Attempt {
            started: Instant::now(),
            tries: 0,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Report {
    Continue,
    Quit,
}

pub struct Driver {
    handle: Handle,
    registry: Registry,
    candidate: Option<Connection>,
    tracker: HealthTracker,
    attempt_opt: Option<Attempt>,
    state: ConnectionState,
    phase: Phase,
    last_endpoint: Option<Endpoint>,
    discovery: Box<Discovery>,
    connection_name: Option<String>,
    default_user: Option<Credentials>,
    operation_timeout: Duration,
    init_req_opt: Option<InitReq>,
    reconnect_delay: Duration,
    max_reconnect: u32,
    sender: Sender<Msg>,
    operation_check_period: Duration,
    last_operation_check: Instant,
}

impl Driver {
    pub fn new(setts: Settings, disc: Box<Discovery>, sender: Sender<Msg>, handle: Handle) -> Driver {
        Driver {
            handle: handle,
            registry: Registry::new(&setts),
            candidate: None,
            tracker: HealthTracker::new(&setts),
            attempt_opt: None,
            state: ConnectionState::Init,
            phase: Phase::Reconnecting,
            last_endpoint: None,
            discovery: disc,
            connection_name: setts.connection_name,
            default_user: setts.default_user,
            operation_timeout: setts.operation_timeout,
            init_req_opt: None,
            reconnect_delay: Duration::from_secs(3),
            max_reconnect: setts.connection_retry.to_u32(),
            sender: sender,
            operation_check_period: setts.operation_check_period,
            last_operation_check: Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.attempt_opt = Some(Attempt::new());
        self.state       = ConnectionState::Connecting;
        self.phase       = Phase::Reconnecting;

        let tick_period = Duration::from_millis(200);
        let tick        = Timer::default().interval(tick_period).map_err(|_| ());

        let tick = tick.fold(self.sender.clone(), |sender, _| {
            sender.send(Msg::Tick).map_err(|_| ())
        });

        self.handle.spawn(tick.then(|_| Ok(())));

        self.discover();
    }

    fn discover(&mut self) {
        if self.state == ConnectionState::Connecting && self.phase == Phase::Reconnecting {
            let endpoint = self.discovery.discover(self.last_endpoint.as_ref());

            self.phase = Phase::EndpointDiscovery;

            // TODO - Properly handle endpoint discovery asynchronously.
            self.handle.spawn(
                self.sender.clone().send(Msg::Establish(endpoint)).then(|_| Ok(())));

            self.tracker.reset();
        }
    }

    pub fn on_establish(&mut self, endpoint: Endpoint) {
        if self.state == ConnectionState::Connecting && self.phase == Phase::EndpointDiscovery {
            self.phase         = Phase::Establishing;
            self.candidate     = Some(Connection::new(self.sender.clone(), endpoint.addr, self.handle.clone()));
            self.last_endpoint = Some(endpoint);
        }
    }

    fn authenticate(&mut self, creds: Credentials) {
        if self.state == ConnectionState::Connecting && self.phase == Phase::Establishing {
            let pkg = Pkg::authenticate(creds);

            self.init_req_opt = Some(InitReq::new(pkg.correlation));
            self.phase        = Phase::Authentication;

            if let Some(conn) = self.candidate.as_ref() {
                conn.enqueue(pkg);
            }
        }
    }

    fn identify_client(&mut self) {
        if self.state == ConnectionState::Connecting && (self.phase == Phase::Authentication || self.phase == Phase::Establishing) {
            let pkg = Pkg::identify_client(&self.connection_name);

            self.init_req_opt = Some(InitReq::new(pkg.correlation));
            self.phase        = Phase::Identification;

            if let Some(conn) = self.candidate.as_ref() {
                conn.enqueue(pkg);
            }
        }
    }

    pub fn on_established(&mut self, id: Uuid) {
        if self.state == ConnectionState::Connecting && self.phase == Phase::Establishing {
            let same_connection =
                match self.candidate {
                    Some(ref conn) => conn.id == id,
                    None           => false,
                };

            if same_connection {
                println!("Connection established: {}.", id);
                self.tracker.reset();

                match self.default_user.clone() {
                    Some(creds) => self.authenticate(creds),
                    None        => self.identify_client(),
                }
            }
        }
    }

    fn is_same_connection(&self, conn_id: &Uuid) -> bool {
        match self.candidate {
            Some(ref conn) => conn.id == *conn_id,
            None           => false,
        }
    }

    pub fn on_connection_closed(&mut self, conn_id: Uuid, error: Error) {
        if self.is_same_connection(&conn_id) {
            println!("CloseConnection: {}.", error);
            self.tcp_connection_close(&conn_id, error);
        }
    }

    fn tcp_connection_close(&mut self, conn_id: &Uuid, err: Error) {
        println!("Connection [{}] error. Cause: {}.", conn_id, err);

        match self.state {
            ConnectionState::Connected => {
                self.attempt_opt = Some(Attempt::new());
                self.state       = ConnectionState::Connecting;
                self.phase       = Phase::Reconnecting;
            },

            ConnectionState::Connecting => {
                self.state = ConnectionState::Connecting;
                self.phase = Phase::Reconnecting;
            },

            _ => (),
        }
    }

    pub fn on_package_arrived(&mut self, pkg: Pkg) {
        self.tracker.incr_pkg_num();

        if pkg.cmd == Cmd::ClientIdentified && self.state == ConnectionState::Connecting && self.phase == Phase::Identification {
            if let Some(req) = self.init_req_opt.take() {
                if req.correlation == pkg.correlation {
                    if let Some(ref conn) = self.candidate {
                        println!("Connection identified: {}.", conn.id);
                    }

                    self.attempt_opt          = None;
                    self.last_operation_check = Instant::now();
                    self.state                = ConnectionState::Connected;
                }
            }
        } else if (pkg.cmd == Cmd::Authenticated || pkg.cmd == Cmd::NotAuthenticated) && self.state == ConnectionState::Connecting && self.phase == Phase::Authentication {
            if let Some(req) = self.init_req_opt.take(){
                if req.correlation == pkg.correlation {
                    if pkg.cmd == Cmd::NotAuthenticated {
                        println!("Not authenticated.");
                    }

                    self.identify_client();
                }
            }
        } else {
            if self.state == ConnectionState::Connected {
                match pkg.cmd {
                    Cmd::HeartbeatRequest => {
                        let mut resp = pkg.copy_headers_only();

                        resp.cmd = Cmd::HeartbeatResponse;

                        if let Some(ref conn) = self.candidate {
                            conn.enqueue(resp);
                        }
                    },

                    Cmd::HeartbeatResponse => (),

                    _ => {
                        // It will be always 'Some' when receiving a package.
                        if let Some(ref conn) = self.candidate {
                            self.registry.handle(pkg, conn);
                        }
                    },
                }
            }
        }
    }

    pub fn on_new_op(&mut self, operation: Exchange) {
        let conn_opt = {
            if self.state.is_connected() {
                // Will be always 'Some' when connected.
                self.candidate.as_ref()
            } else {
                None
            }
        };

        self.registry.register(operation, conn_opt);
    }

    fn has_init_req_timeout(&self) -> bool {
        if let Some(ref req) = self.init_req_opt {
            req.started.elapsed() >= self.operation_timeout
        } else {
            false
        }
    }

    fn conn_has_timeout(&self) -> bool {
        if let Some(att) = self.attempt_opt.as_ref() {
            att.started.elapsed() >= self.reconnect_delay
        } else {
            false
        }
    }

    fn start_new_attempt(&mut self) -> bool {
        if let Some(att) = self.attempt_opt.as_mut() {
            att.tries   += 1;
            att.started = Instant::now();

            att.tries <= self.max_reconnect
        } else {
            false
        }
    }

    pub fn close_connection(&mut self) {
        self.state = ConnectionState::Closed;
    }

    fn manage_heartbeat(&mut self) {
        let has_timeout =
                if let Some(ref conn) = self.candidate {
                    match self.tracker.manage_heartbeat(conn) {
                        Heartbeat::Valid   => false,
                        Heartbeat::Failure => true,
                    }
                } else {
                    false
                };

        if has_timeout {
            if let Some(conn) = self.candidate.take() {
                self.tcp_connection_close(&conn.id, heartbeat_timeout_error());
            }
        }
    }

    pub fn on_tick(&mut self) -> Report {

        if self.state == ConnectionState::Init || self.state == ConnectionState::Closed {
            return Report::Continue;
        }

        if self.state == ConnectionState::Connecting {
            if self.phase == Phase::Reconnecting {
                if self.conn_has_timeout() {
                    if self.start_new_attempt() {
                        self.discover();
                    } else {
                        return Report::Quit;
                    }
                }
            } else if self.phase == Phase::Authentication {
                if self.has_init_req_timeout() {
                    println!("Authentication has timeout.");

                    self.identify_client();
                }

                self.manage_heartbeat();
            } else if self.phase == Phase::Identification {
                if self.has_init_req_timeout() {
                    return Report::Quit;
                } else {
                    self.manage_heartbeat();
                }
            }
        } else {
            // Connected state
            if let Some(ref conn) = self.candidate {
                if self.last_operation_check.elapsed() >= self.operation_check_period {
                    self.registry.check_and_retry(conn);

                    self.last_operation_check = Instant::now();
                }
            }

            self.manage_heartbeat();
        }

        Report::Continue
    }

    pub fn on_send_pkg(&mut self, pkg: Pkg) {
        if self.state == ConnectionState::Connected {
            if let Some(ref conn) = self.candidate {
                conn.enqueue(pkg);
            }
        }
    }
}

fn heartbeat_timeout_error() -> Error {
    Error::new(ErrorKind::Other, "Heartbeat timeout error.")
}