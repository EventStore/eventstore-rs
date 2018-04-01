#[derive(Copy, Clone, Debug)]
pub enum Cmd {
    HeartbeatRequest,
    HeartbeatResponse,
    IdentifyClient,
    ClientIdentified,
    Authenticate,
    Authenticated,
    NotAuthenticated,
    WriteEvents,
    WriteEventsCompleted,
    ReadEvent,
    ReadEventCompleted,
    TransactionStart,
    TransactionStartCompleted,
    TransactionWrite,
    TransactionWriteCompleted,
    TransactionCommit,
    TransactionCommitCompleted,
    Unknown(u8),
}

impl PartialEq for Cmd {
    fn eq(&self, other: &Cmd) -> bool {
        self.to_u8() == other.to_u8()
    }
}

impl Eq for Cmd {}

impl Cmd {
    pub fn to_u8(&self) -> u8 {
        match *self {
            Cmd::HeartbeatRequest           => 0x01,
            Cmd::HeartbeatResponse          => 0x02,
            Cmd::IdentifyClient             => 0xF5,
            Cmd::ClientIdentified           => 0xF6,
            Cmd::Authenticate               => 0xF2,
            Cmd::Authenticated              => 0xF3,
            Cmd::NotAuthenticated           => 0xF4,
            Cmd::WriteEvents                => 0x82,
            Cmd::WriteEventsCompleted       => 0x83,
            Cmd::ReadEvent                  => 0xB0,
            Cmd::ReadEventCompleted         => 0xB1,
            Cmd::TransactionStart           => 0x84,
            Cmd::TransactionStartCompleted  => 0x85,
            Cmd::TransactionWrite           => 0x86,
            Cmd::TransactionWriteCompleted  => 0x87,
            Cmd::TransactionCommit          => 0x88,
            Cmd::TransactionCommitCompleted => 0x89,
            Cmd::Unknown(cmd)               => cmd,
        }
    }

    pub fn from_u8(cmd: u8) -> Cmd {
        match cmd {
            0x01 => Cmd::HeartbeatRequest,
            0x02 => Cmd::HeartbeatResponse,
            0xF5 => Cmd::IdentifyClient,
            0xF6 => Cmd::ClientIdentified,
            0xF2 => Cmd::Authenticate,
            0xF3 => Cmd::Authenticated,
            0xF4 => Cmd::NotAuthenticated,
            0x82 => Cmd::WriteEvents,
            0x83 => Cmd::WriteEventsCompleted,
            0xB0 => Cmd::ReadEvent,
            0xB1 => Cmd::ReadEventCompleted,
            0x84 => Cmd::TransactionStart,
            0x85 => Cmd::TransactionStartCompleted,
            0x86 => Cmd::TransactionWrite,
            0x87 => Cmd::TransactionWriteCompleted,
            0x88 => Cmd::TransactionCommit,
            0x89 => Cmd::TransactionCommitCompleted,
            _    => Cmd::Unknown(cmd),
        }
    }
}
