#[cfg(all(feature = "network-testing", not(feature = "offline-testing")))]
/// Tests that require connectivity to the server
mod online {
    use crate::fetch::{common::setup_logs, config::Config};
    use dapi_grpc::platform::v0::{
        wait_for_state_transition_result_request::WaitForStateTransitionResultRequestV0,
        WaitForStateTransitionResultRequest,
    };
    use rs_dapi_client::{DapiRequest, RequestSettings};
    use rs_sdk::Sdk;
    use std::time::Duration;

    /// Send streaming request to the server and time out after 1 second (because we don't expect to receive anything)
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[cfg(all(feature = "network-testing", not(feature = "offline-testing")))]
    async fn test_wait_timeout() {
        setup_logs();

        const TIMEOUT: std::time::Duration = std::time::Duration::from_millis(400);

        let cfg = Config::new();
        let sdk = cfg.setup_api("test_wait_timeout").await;
        let sdk_ref: &Sdk = sdk.as_ref();

        let request: WaitForStateTransitionResultRequest = WaitForStateTransitionResultRequestV0 {
            prove: false,
            state_transition_hash: [0u8; 32].to_vec(),
        }
        .into();

        let settings = RequestSettings {
            timeout: Some(TIMEOUT),
            ..Default::default()
        };

        // we add few millis to duration to give chance to the server to time out before we kill request
        let response = tokio::time::timeout(
            TIMEOUT + Duration::from_millis(100),
            request.execute(sdk_ref, settings),
        )
        .await
        .expect("expected request timeout, got tokio timeout");

        assert!(response.is_err(), "expected timeout, got {:?}", response);
        tracing::info!(response = ?response, "received timeout");
        // assert!(response.version.is_some());
    }
}
