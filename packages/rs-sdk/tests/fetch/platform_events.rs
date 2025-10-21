use super::{common::setup_logs, config::Config};
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::platform::v0::platform_subscription_request::{
    PlatformSubscriptionRequestV0, Version as RequestVersion,
};
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
            client_subscription_id: "test-subscription".to_string(),
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

    // Ensure the stream stays open (no immediate error) by expecting a timeout when waiting for the first message.
    let wait_result = timeout(Duration::from_millis(250), stream.message()).await;
    assert!(
        wait_result.is_err(),
        "expected to time out waiting for initial platform event"
    );
}
