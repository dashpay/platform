use super::{common::setup_logs, config::Config};
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::platform::v0::platform_events_command::platform_events_command_v0::Command as Cmd;
use dapi_grpc::platform::v0::platform_events_command::Version as CmdVersion;
use dapi_grpc::platform::v0::platform_events_response::platform_events_response_v0::Response as Resp;
use dapi_grpc::platform::v0::platform_events_response::Version as RespVersion;
use dapi_grpc::platform::v0::{AddSubscriptionV0, PingV0, PlatformEventsCommand, PlatformFilterV0};
use dash_event_bus::{EventMux, GrpcPlatformEventsProducer};
use rs_dapi_client::transport::create_channel;
use rs_dapi_client::{RequestSettings, Uri};
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg(all(feature = "network-testing", not(feature = "offline-testing")))]
async fn test_platform_events_ping() {
    setup_logs();

    // Build gRPC client from test config
    let cfg = Config::new();
    let address = cfg
        .address_list()
        .get_live_address()
        .expect("at least one platform address configured")
        .clone();
    let uri: Uri = address.uri().clone();
    let settings = RequestSettings {
        timeout: Some(Duration::from_secs(30)),
        ..Default::default()
    }
    .finalize();
    let channel = create_channel(uri, Some(&settings)).expect("create channel");
    let client = PlatformClient::new(channel);

    // Wire EventMux with a gRPC producer
    let mux = EventMux::new();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let mux_worker = mux.clone();
    tokio::spawn(async move {
        let _ = GrpcPlatformEventsProducer::run(mux_worker, client, ready_tx).await;
    });
    // Wait until producer is ready
    timeout(Duration::from_secs(5), ready_rx)
        .await
        .expect("producer ready timeout")
        .expect("producer start");

    // Create a raw subscriber on the mux to send commands and receive responses
    let sub = mux.add_subscriber().await;
    let cmd_tx = sub.cmd_tx;
    let mut resp_rx = sub.resp_rx;

    // Choose a numeric ID for our subscription and ping
    let id_num: u64 = 4242;
    let id_str = id_num.to_string();

    // Send Add with our chosen client_subscription_id
    let add_cmd = PlatformEventsCommand {
        version: Some(CmdVersion::V0(
            dapi_grpc::platform::v0::platform_events_command::PlatformEventsCommandV0 {
                command: Some(Cmd::Add(AddSubscriptionV0 {
                    client_subscription_id: id_str.clone(),
                    filter: Some(PlatformFilterV0::default()),
                })),
            },
        )),
    };
    cmd_tx.send(Ok(add_cmd)).expect("send add");

    // Expect Add ack
    let add_ack = timeout(Duration::from_secs(3), resp_rx.recv())
        .await
        .expect("timeout waiting add ack")
        .expect("subscriber closed")
        .expect("ack error");
    match add_ack.version.and_then(|v| match v {
        RespVersion::V0(v0) => v0.response,
    }) {
        Some(Resp::Ack(a)) => {
            assert_eq!(a.client_subscription_id, id_str);
            assert_eq!(a.op, "add");
        }
        other => panic!("expected add ack, got: {:?}", other.map(|_| ())),
    }

    // Send Ping with matching nonce so that ack routes to our subscription
    let ping_cmd = PlatformEventsCommand {
        version: Some(CmdVersion::V0(
            dapi_grpc::platform::v0::platform_events_command::PlatformEventsCommandV0 {
                command: Some(Cmd::Ping(PingV0 { nonce: id_num })),
            },
        )),
    };
    cmd_tx.send(Ok(ping_cmd)).expect("send ping");

    // Expect Ping ack routed through Mux to our subscriber
    let ping_ack = timeout(Duration::from_secs(3), resp_rx.recv())
        .await
        .expect("timeout waiting ping ack")
        .expect("subscriber closed")
        .expect("ack error");
    match ping_ack.version.and_then(|v| match v {
        RespVersion::V0(v0) => v0.response,
    }) {
        Some(Resp::Ack(a)) => {
            assert_eq!(a.client_subscription_id, id_str);
            assert_eq!(a.op, "ping");
        }
        other => panic!("expected ping ack, got: {:?}", other.map(|_| ())),
    }
}
