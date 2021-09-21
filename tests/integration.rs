#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;

mod images;

use eventstore::{
    Acl, Client, ClientSettings, EventData, PersistentSubscriptionOptions,
    PersistentSubscriptionSettings, ProjectionClient, ReadResult, Single, StreamAclBuilder,
    StreamMetadataBuilder, StreamMetadataResult,
};
use futures::channel::oneshot;
use futures::stream::TryStreamExt;
use std::collections::HashMap;
use std::error::Error;
use testcontainers::clients::Cli;
use testcontainers::{Docker, RunArgs};

fn fresh_stream_id(prefix: &str) -> String {
    let uuid = uuid::Uuid::new_v4();

    format!("{}-{}", prefix, uuid)
}

fn generate_events<Type: AsRef<str>>(event_type: Type, cnt: usize) -> Vec<EventData> {
    let mut events = Vec::with_capacity(cnt);

    for idx in 1..cnt + 1 {
        let payload = json!({
            "event_index": idx,
        });

        let data = EventData::json(event_type.as_ref(), payload).unwrap();
        events.push(data);
    }

    events
}

async fn test_write_events(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("write_events");
    let events = generate_events("es6-write-events-test".to_string(), 3);

    let result = client
        .append_to_stream(stream_id, &Default::default(), events)
        .await?;

    debug!("Write response: {:?}", result);

    Ok(())
}

// We read all stream events by batch.
async fn test_read_all_stream_events(client: &Client) -> Result<(), Box<dyn Error>> {
    // Eventstore should always have "some" events in $all, since eventstore itself uses streams, ouroboros style.
    client.read_all(&Default::default(), Single).await?;

    Ok(())
}

// We read stream events by batch. We also test if we can properly read a
// stream thoroughly.
async fn test_read_stream_events(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("read_stream_events");
    let events = generate_events("es6-read-stream-events-test".to_string(), 10);

    let _ = client
        .append_to_stream(stream_id.clone(), &Default::default(), events)
        .await?;

    let mut pos = 0usize;
    let mut idx = 0i64;

    let result = client
        .read_stream(stream_id, &Default::default(), 10)
        .await?;

    if let eventstore::ReadResult::Ok(mut stream) = result {
        while let Some(event) = stream.try_next().await? {
            let event = event.get_original_event();
            let obj: HashMap<String, i64> = event.as_json().unwrap();
            let value = obj.get("event_index").unwrap();

            idx = *value;
            pos += 1;
        }
    }

    assert_eq!(pos, 10);
    assert_eq!(idx, 10);

    Ok(())
}

async fn test_metadata(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("metadata");
    let events = generate_events("metadata-test".to_string(), 5);

    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events)
        .await?;

    let expected = StreamMetadataBuilder::new()
        .max_age(std::time::Duration::from_secs(2))
        .acl(Acl::Stream(
            StreamAclBuilder::new().add_read_roles("admin").build(),
        ))
        .build();

    let _ = client
        .set_stream_metadata(stream_id.as_str(), &Default::default(), expected.clone())
        .await?;

    let actual = client
        .get_stream_metadata(stream_id.as_str(), &Default::default())
        .await?;

    assert!(actual.is_success());

    if let StreamMetadataResult::Success(actual) = actual {
        assert_eq!(&expected, actual.metadata());
    }

    Ok(())
}

async fn test_metadata_not_exist(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("metadata_not_exist");
    let events = generate_events("metadata-test-not-exist".to_string(), 5);

    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events)
        .await?;

    let actual = client
        .get_stream_metadata(stream_id.as_str(), &Default::default())
        .await?;

    assert!(actual.is_not_found());

    Ok(())
}

// We check to see the client can handle the correct GRPC proto response when
// a stream does not exist
async fn test_read_stream_events_non_existent(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("read_stream_events");

    let result = client
        .read_stream(stream_id.as_str(), &Default::default(), Single)
        .await?;

    if let eventstore::ReadResult::StreamNotFound(stream) = result {
        assert_eq!(stream, stream_id);
        return Ok(());
    }

    panic!("We expected to have a stream not found result");
}

// We write an event into a stream then soft delete that stream.
async fn test_delete_stream(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("delete");
    let events = generate_events("delete-test".to_string(), 1);

    let _ = client
        .append_to_stream(stream_id.clone(), &Default::default(), events)
        .await?;

    let result = client
        .delete_stream(stream_id.as_str(), &Default::default())
        .await?;

    debug!("Delete stream [{}] result: {:?}", stream_id, result);

    Ok(())
}

// We write an event into a stream then hard delete that stream.
async fn test_tombstone_stream(client: &Client) -> Result<(), Box<dyn Error>> {
    let _ = pretty_env_logger::try_init();
    let stream_id = fresh_stream_id("tombstone");
    let events = generate_events("tombstone-test".to_string(), 1);

    let _ = client
        .append_to_stream(stream_id.clone(), &Default::default(), events)
        .await?;

    let result = client
        .tombstone_stream(stream_id.as_str(), &Default::default())
        .await?;

    debug!("Tombstone stream [{}] result: {:?}", stream_id, result);

    let result = client
        .read_stream(stream_id.as_str(), &Default::default(), 1)
        .await?;

    if let ReadResult::StreamDeleted(stream_name) = result {
        assert_eq!(stream_id, stream_name);
        Ok(())
    } else {
        panic!("Expected stream deleted error");
    }
}

// We write events into a stream. Then, we issue a catchup subscription. After,
// we write another batch of events into the same stream. The goal is to make
// sure we receive events written prior and after our subscription request.
// To assess we received all the events we expected, we test our subscription
// internal state value.
async fn test_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("catchup");
    let events_before = generate_events("catchup-test-before".to_string(), 3);
    let events_after = generate_events("catchup-test-after".to_string(), 3);

    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events_before)
        .await?;

    let mut sub = client
        .subscribe_to_stream(stream_id.as_str(), &Default::default())
        .await?;

    let (tx, recv) = oneshot::channel();

    tokio::spawn(async move {
        let mut count = 0usize;
        let max = 6usize;

        while let Some(event) = sub.try_next().await? {
            if let eventstore::SubEvent::EventAppeared(_) = event {
                count += 1;

                if count == max {
                    break;
                }
            }
        }

        tx.send(count).unwrap();
        Ok(()) as eventstore::Result<()>
    });

    let _ = client
        .append_to_stream(stream_id, &Default::default(), events_after)
        .await?;

    let test_count = recv.await?;

    assert_eq!(
        test_count, 6,
        "We are testing proper state after catchup subscription: got {} expected {}.",
        test_count, 6
    );

    Ok(())
}

async fn test_create_persistent_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("create_persistent_sub");

    client
        .create_persistent_subscription(stream_id, "a_group_name", &Default::default())
        .await?;

    Ok(())
}

// We test we can successfully update a persistent subscription.
async fn test_update_persistent_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("update_persistent_sub");

    client
        .create_persistent_subscription(stream_id.as_str(), "a_group_name", &Default::default())
        .await?;

    let mut setts = PersistentSubscriptionSettings::default();

    setts.max_retry_count = 1000;

    let options = PersistentSubscriptionOptions::default().settings(setts);
    client
        .update_persistent_subscription(stream_id, "a_group_name", &options)
        .await?;

    Ok(())
}

// We test we can successfully delete a persistent subscription.
async fn test_delete_persistent_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("delete_persistent_sub");
    client
        .create_persistent_subscription(stream_id.as_str(), "a_group_name", &Default::default())
        .await?;

    client
        .delete_persistent_subscription(stream_id, "a_group_name", &Default::default())
        .await?;

    Ok(())
}

async fn test_persistent_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("persistent_subscription");
    let events = generate_events("es6-persistent-subscription-test".to_string(), 5);

    client
        .create_persistent_subscription(stream_id.as_str(), "a_group_name", &Default::default())
        .await?;

    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events)
        .await?;

    let (mut read, mut write) = client
        .connect_persistent_subscription(stream_id.as_str(), "a_group_name", &Default::default())
        .await?;

    let max = 10usize;

    let handle = tokio::spawn(async move {
        let mut count = 0usize;
        while let Some(event) = read.try_next().await.unwrap() {
            if let eventstore::SubEvent::EventAppeared(event) = event {
                write.ack_event(event).await.unwrap();

                count += 1;

                if count == max {
                    break;
                }
            }
        }

        count
    });

    let events = generate_events("es6-persistent-subscription-test".to_string(), 5);
    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events)
        .await?;

    let count = handle.await?;

    assert_eq!(
        count, 10,
        "We are testing proper state after persistent subscription: got {} expected {}",
        count, 10
    );

    Ok(())
}

async fn test_batch_append(client: &Client) -> Result<(), Box<dyn Error>> {
    let batch_client = client.batch_append(&Default::default()).await?;

    for _ in 0..3 {
        let stream_id = fresh_stream_id("batch-append");
        let events = generate_events("batch-append-type", 3);
        let _ = batch_client
            .append_to_stream(
                stream_id.as_str(),
                eventstore::ExpectedRevision::Any,
                events,
            )
            .await?;
        let options = eventstore::ReadStreamOptions::default()
            .forwards()
            .position(eventstore::StreamPosition::Start);
        let mut stream = client
            .read_stream(stream_id.as_str(), &options, eventstore::All)
            .await?
            .unwrap();

        let mut cpt = 0usize;

        while let Some(_) = stream.try_next().await? {
            cpt += 1;
        }

        assert_eq!(cpt, 3, "We expecting 3 events out of those streams");
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_error_on_failure_to_discover_single_node() -> Result<(), Box<dyn Error>> {
    let _ = pretty_env_logger::try_init();

    let settings = format!("esdb://noserver:{}", 2_113).parse()?;
    let client = Client::new(settings).await?;
    let stream_id = fresh_stream_id("wont-be-created");
    let events = generate_events("wont-be-written".to_string(), 5);

    let result = client
        .append_to_stream(stream_id, &Default::default(), events)
        .await;

    if let Err(eventstore::Error::GrpcConnectionError(_)) = result {
        Ok(())
    } else {
        panic!("Expected gRPC connection error");
    }
}

type VolumeName = String;

fn create_unique_volume() -> Result<VolumeName, Box<dyn std::error::Error>> {
    let dir_name = uuid::Uuid::new_v4();
    let dir_name = format!("dir-{}", dir_name);

    std::process::Command::new("docker")
        .arg("volume")
        .arg("create")
        .arg(format!("--name {}", dir_name))
        .output()?;

    Ok(dir_name)
}

async fn wait_node_is_alive(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match tokio::time::timeout(
            std::time::Duration::from_secs(1),
            reqwest::get(format!("http://localhost:{}/health/live", port)),
        )
        .await
        {
            Err(_) => error!("Healthcheck timed out! retrying..."),

            Ok(resp) => match resp {
                Err(e) => error!("Node localhost:{} is not up yet: {}", port, e),

                Ok(resp) => {
                    if resp.status().is_success() {
                        break;
                    }

                    error!(
                        "Healthcheck response was not successful: {}, retrying...",
                        resp.status()
                    );
                }
            },
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn cluster() -> Result<(), Box<dyn std::error::Error>> {
    let _ = pretty_env_logger::try_init();
    let settings = "esdb://admin:changeit@localhost:2111,localhost:2112,localhost:2113?tlsVerifyCert=false&nodePreference=leader"
        .parse::<ClientSettings>()?;

    let client = Client::new(settings).await?;

    all_around_tests(client).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn single_node() -> Result<(), Box<dyn std::error::Error>> {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();
    let image = images::ESDB::default().insecure_mode();
    let container = docker.run_with_args(image, RunArgs::default());

    wait_node_is_alive(container.get_host_port(2_113).unwrap()).await?;

    let settings = format!(
        "esdb://localhost:{}?tls=false",
        container.get_host_port(2_113).unwrap(),
    )
    .parse::<ClientSettings>()?;

    let client = Client::new(settings).await?;

    all_around_tests(client).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_auto_resub_on_connection_drop() -> Result<(), Box<dyn std::error::Error>> {
    let volume = create_unique_volume()?;
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();
    let image = images::ESDB::default()
        .insecure_mode()
        .attach_volume_to_db_directory(volume);
    let container = docker.run_with_args(
        image.clone(),
        RunArgs::default().with_mapped_port((3_113, 2_113)),
    );

    wait_node_is_alive(3_113).await?;

    let settings = format!("esdb://localhost:{}?tls=false", 3_113).parse::<ClientSettings>()?;

    let client = Client::new(settings).await?;
    let stream_name = fresh_stream_id("auto-reconnect");
    let retry = eventstore::RetryOptions::default().retry_forever();
    let options = eventstore::SubscribeToStreamOptions::default().retry_options(retry);
    let mut stream = client
        .subscribe_to_stream(stream_name.as_str(), &options)
        .await?;
    let max = 6usize;
    let (tx, recv) = oneshot::channel();

    tokio::spawn(async move {
        let mut count = 0usize;

        while let Some(_) = stream.try_next().await? {
            count += 1;

            if count == max {
                break;
            }
        }

        tx.send(count).unwrap();
        Ok(()) as eventstore::Result<()>
    });

    let events = generate_events("reconnect".to_string(), 3);

    let _ = client
        .append_to_stream(stream_name.as_str(), &Default::default(), events)
        .await?;

    container.stop();
    let _container = docker.run_with_args(
        image.clone(),
        RunArgs::default().with_mapped_port((3_113, 2_113)),
    );

    wait_node_is_alive(3_113).await?;

    let events = generate_events("reconnect".to_string(), 3);

    let _ = client
        .append_to_stream(stream_name.as_str(), &Default::default(), events)
        .await?;
    let test_count = recv.await?;

    assert_eq!(
        test_count, 6,
        "We are testing proper state after subscription upon reconnection: got {} expected {}.",
        test_count, 6
    );

    Ok(())
}

async fn all_around_tests(client: Client) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Before test_write_events…");
    test_write_events(&client).await?;
    debug!("Complete");
    debug!("Before test_all_read_stream_events…");
    test_read_all_stream_events(&client).await?;
    debug!("Complete");
    debug!("Before test_read_stream_events…");
    test_read_stream_events(&client).await?;
    debug!("Complete");
    debug!("Before test_read_stream_events_non_existent");
    test_read_stream_events_non_existent(&client).await?;
    debug!("Complete");
    debug!("Before test test_metadata");
    test_metadata(&client).await?;
    debug!("Complete");
    debug!("Before test test_metadata_not_exist");
    test_metadata_not_exist(&client).await?;
    debug!("Complete");
    debug!("Before test_delete_stream…");
    test_delete_stream(&client).await?;
    debug!("Complete");
    debug!("Before test_tombstone_stream…");
    test_tombstone_stream(&client).await?;
    debug!("Complete");
    debug!("Before test_subscription…");
    test_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_create_persistent_subscription…");
    test_create_persistent_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_update_persistent_subscription…");
    test_update_persistent_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_delete_persistent_subscription…");
    test_delete_persistent_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_persistent_subscription…");
    test_persistent_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_batch_append");
    test_batch_append(&client).await?;
    debug!("Complete");

    Ok(())
}

static PROJECTION_FILE: &'static str = include_str!("fixtures/projection.js");
static PROJECTION_UPDATED_FILE: &'static str = include_str!("fixtures/projection-updated.js");

async fn wait_until_projection_status_cc(
    client: &ProjectionClient,
    name: &str,
    last_status: &mut String,
    status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let result = client.get_status(name, None).await?;

        if let Some(stats) = result {
            if stats.status.contains(status) {
                break;
            }

            *last_status = stats.status.clone();
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Ok(())
}

async fn wait_until_projection_status_is(
    client: &ProjectionClient,
    name: &str,
    status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut last_status = "".to_string();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(FIVE_MINS_IN_SECS),
        wait_until_projection_status_cc(client, name, &mut last_status, status),
    )
    .await;

    if result.is_err() {
        error!(
            "Projection {} doesn't reach the expected status. Got {}, Expected {}",
            name, last_status, status
        );
    }

    result?
}

async fn wait_until_state_ready<A>(
    client: &ProjectionClient,
    name: &str,
) -> Result<A, Box<dyn std::error::Error>>
where
    A: serde::de::DeserializeOwned + Send,
{
    tokio::time::timeout(
        std::time::Duration::from_secs(FIVE_MINS_IN_SECS),
        async move {
            loop {
                let result: serde_json::Result<A> =
                    client.get_state(name, &Default::default()).await?;

                if let Ok(a) = result {
                    return Ok::<A, Box<dyn std::error::Error>>(a);
                }
            }
        },
    )
    .await?
}

async fn wait_until_result_ready<A>(
    client: &ProjectionClient,
    name: &str,
) -> Result<A, Box<dyn std::error::Error>>
where
    A: serde::de::DeserializeOwned + Send,
{
    tokio::time::timeout(
        std::time::Duration::from_secs(FIVE_MINS_IN_SECS),
        async move {
            loop {
                let result: serde_json::Result<A> =
                    client.get_result(name, &Default::default()).await?;

                if let Ok(a) = result {
                    return Ok::<A, Box<dyn std::error::Error>>(a);
                }
            }
        },
    )
    .await?
}

const FIVE_MINS_IN_SECS: u64 = 5 * 60;

async fn create_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await
}

// TODO - A projection must be stopped to be able to delete it. But Stop projection gRPC call doesn't exist yet.
async fn delete_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;

    debug!("delete_projection: create_projection succeeded: {}", name);

    client.abort(name.as_str(), None).await?;

    wait_until_projection_status_is(client, name.as_str(), "Aborted").await?;

    debug!("delete_projection: reading newly-created projection statistic succeeded");

    let cloned_name = name.clone();
    // There is a race-condition in the projection manager: https://github.com/EventStore/EventStore/issues/2938
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), async move {
        loop {
            let result = client
                .delete(cloned_name.as_str(), &Default::default())
                .await;

            if result.is_ok() {
                break;
            }

            warn!("projection deletion failed with: {:?}. Retrying...", result);
            let _ = tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    })
    .await;

    if result.is_err() {
        warn!("projection deletion didn't complete under test timeout. Not a big deal considering https://github.com/EventStore/EventStore/issues/2938");
    }

    Ok(())
}
async fn update_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();

    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;

    client
        .update(
            name.as_str(),
            PROJECTION_UPDATED_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    let stats = client.get_status(name.as_str(), None).await?;

    assert!(stats.is_some());

    let stats = stats.unwrap();

    assert_eq!(stats.name, name);
    assert_eq!(stats.version, 1);

    Ok(())
}

async fn enable_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;

    client.enable(name.as_str(), None).await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;

    Ok(())
}

async fn disable_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;
    client.enable(name.as_str(), None).await?;
    wait_until_projection_status_is(client, name.as_str(), "Running").await?;
    client.disable(name.as_str(), None).await?;
    wait_until_projection_status_is(client, name.as_str(), "Stopped").await?;

    Ok(())
}

async fn reset_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;
    client.enable(name.as_str(), None).await?;
    client.reset(name.as_str(), None).await?;

    Ok(())
}

async fn projection_state(
    stream_client: &Client,
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    use serde::Deserialize;

    let events = generate_events("testing", 10);
    let stream_name = gen_name.next().unwrap();

    let write_result = stream_client
        .append_to_stream(stream_name, &Default::default(), events)
        .await?;

    assert!(write_result.is_ok());

    // This is the state of the projection, see tests/fixtures/projection.js.
    #[derive(Deserialize, Debug)]
    struct State {
        foo: Foo,
    }

    #[derive(Deserialize, Debug)]
    struct Foo {
        baz: Baz,
    }

    #[derive(Deserialize, Debug)]
    struct Baz {
        count: f64,
    }

    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;
    client.enable(name.as_str(), None).await?;

    let state = wait_until_state_ready::<State>(client, name.as_str()).await?;

    debug!("{:?}", state);

    Ok(())
}

async fn projection_result(
    stream_client: &Client,
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    use serde::Deserialize;

    let events = generate_events("testing", 10);
    let stream_name = gen_name.next().unwrap();

    let write_result = stream_client
        .append_to_stream(stream_name, &Default::default(), events)
        .await?;

    assert!(write_result.is_ok());

    // This is the state of the projection, see tests/fixtures/projection.js.
    #[derive(Deserialize, Debug)]
    struct State {
        foo: Foo,
    }

    #[derive(Deserialize, Debug)]
    struct Foo {
        baz: Baz,
    }

    #[derive(Deserialize, Debug)]
    struct Baz {
        count: f64,
    }

    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;
    client.enable(name.as_str(), None).await?;

    let result = wait_until_result_ready::<State>(client, name.as_str()).await?;

    debug!("{:?}", result);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn projection_tests() -> Result<(), Box<dyn std::error::Error>> {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();
    let image = images::ESDB::default().insecure_mode().enable_projections();
    let container = docker.run_with_args(image, RunArgs::default());

    wait_node_is_alive(container.get_host_port(2_113).unwrap()).await?;

    debug!("Docker container is ready and alive!");

    let settings = format!(
        "esdb://localhost:{}?tls=false",
        container.get_host_port(2_113).unwrap(),
    )
    .parse::<ClientSettings>()?;

    let client = ProjectionClient::new(settings.clone()).await?;
    let stream_client = Client::new(settings).await?;
    let mut name_gen = names::Generator::default();

    create_projection(&client, &mut name_gen).await?;
    debug!("create_projection passed");
    delete_projection(&client, &mut name_gen).await?;
    debug!("delete_projection passed");
    update_projection(&client, &mut name_gen).await?;
    debug!("update_projection passed");
    enable_projection(&client, &mut name_gen).await?;
    debug!("enable_projection passed");
    disable_projection(&client, &mut name_gen).await?;
    debug!("disable_projection passed");
    reset_projection(&client, &mut name_gen).await?;
    debug!("reset_projection passed");
    projection_state(&stream_client, &client, &mut name_gen).await?;
    debug!("projection_state passed");
    projection_result(&stream_client, &client, &mut name_gen).await?;
    debug!("projection_result passed");
    Ok(())
}
