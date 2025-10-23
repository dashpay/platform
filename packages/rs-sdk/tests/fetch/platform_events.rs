use super::{common::setup_logs, config::Config};
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::platform::v0::platform_subscription_request::{
    PlatformSubscriptionRequestV0, Version as RequestVersion,
};
use dapi_grpc::platform::v0::platform_subscription_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{PlatformFilterV0, PlatformSubscriptionRequest};
use dapi_grpc::tonic::Request;
use rs_dapi_client::transport::create_channel;
use rs_dapi_client::{RequestSettings, Uri};
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg(all(feature = "network-testing", not(feature = "offline-testing")))]
async fn test_platform_events_subscribe_stream_opens() {
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
    let mut client = PlatformClient::new(channel);

    let request = PlatformSubscriptionRequest {
        version: Some(RequestVersion::V0(PlatformSubscriptionRequestV0 {
            filter: Some(PlatformFilterV0 {
                kind: Some(dapi_grpc::platform::v0::platform_filter_v0::Kind::All(true)),
            }),
        })),
    };

    let mut stream = client
        .subscribe_platform_events(Request::new(request))
        .await
        .expect("subscribe")
        .into_inner();

    let handshake = timeout(Duration::from_secs(1), stream.message())
        .await
        .expect("handshake should arrive promptly")
        .expect("handshake message")
        .expect("handshake payload");

    if let Some(ResponseVersion::V0(v0)) = handshake.version {
        assert!(
            !v0.client_subscription_id.is_empty(),
            "handshake must include subscription id"
        );
        assert!(
            v0.client_subscription_id
                .parse::<u64>()
                .map(|id| id > 0)
                .unwrap_or(false),
            "handshake subscription id must be a positive integer"
        );
        assert!(
            v0.event.is_none(),
            "handshake should not include an event payload"
        );
    } else {
        panic!("unexpected handshake response version");
    }

    // Ensure the stream stays open (no immediate event) by expecting a timeout when waiting for the next message.
    let wait_result = timeout(Duration::from_millis(250), stream.message()).await;
    assert!(
        wait_result.is_err(),
        "expected to time out waiting for initial platform event"
    );
}
