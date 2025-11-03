#![cfg(all(
    feature = "network-testing",
    feature = "subscriptions",
    not(feature = "offline-testing")
))]

use super::{common::setup_logs, config::Config};
use dapi_grpc::platform::v0::platform_subscription_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::PlatformFilterV0;
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_platform_events_subscribe_stream_opens() {
    setup_logs();

    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("document_read").await;

    let filter = PlatformFilterV0 {
        kind: Some(dapi_grpc::platform::v0::platform_filter_v0::Kind::All(true)),
    };
    let mut subscription = sdk
        .subscribe_platform_events(filter)
        .await
        .expect("subscribe should succeed");

    let handshake = timeout(Duration::from_secs(1), subscription.message())
        .await
        .expect("handshake should arrive promptly")
        .expect("handshake message")
        .expect("handshake payload");

    if let Some(ResponseVersion::V0(ref v0)) = handshake.version {
        assert!(
            !v0.client_subscription_id.is_empty(),
            "handshake must include subscription id"
        );

        tracing::debug!(
            ?handshake,
            "Received handshake with subscription id: {}",
            v0.client_subscription_id
        );

        assert!(
            v0.event.is_none(),
            "handshake should not include an event payload"
        );
    } else {
        panic!("unexpected handshake response version");
    }

    // Ensure the stream stays open (no immediate event) by expecting a timeout when waiting for the next message.
    let wait_result = timeout(Duration::from_millis(250), subscription.message()).await;
    assert!(
        wait_result.is_err(),
        "expected to time out waiting for initial platform event"
    );
}
