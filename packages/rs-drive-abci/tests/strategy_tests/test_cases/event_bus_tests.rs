#[cfg(test)]
mod tests {
    use drive_abci::config::PlatformConfig;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;

    ///  Verify the in-process EventBus subscription delivers a published event
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn event_bus_subscribe_all_and_receive() {
        use dapi_grpc::platform::v0::platform_event_v0;
        use dapi_grpc::platform::v0::platform_filter_v0::Kind as FilterKind;
        use dapi_grpc::platform::v0::{PlatformEventV0, PlatformFilterV0};
        use drive_abci::abci::app::FullAbciApplication;
        use drive_abci::query::PlatformFilterAdapter;

        let config = PlatformConfig::default();
        let platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        // Create ABCI app and subscribe to all events
        let abci_application = FullAbciApplication::new(&platform.platform);
        let filter = PlatformFilterV0 {
            kind: Some(FilterKind::All(Default::default())),
        };
        let handle = abci_application
            .event_bus
            .add_subscription(PlatformFilterAdapter::new(filter))
            .await;

        // Publish a simple BlockCommitted event
        let meta = platform_event_v0::BlockMetadata {
            height: 1,
            time_ms: 123,
            block_id_hash: vec![0u8; 32],
        };
        let evt = PlatformEventV0 {
            event: Some(platform_event_v0::Event::BlockCommitted(
                platform_event_v0::BlockCommitted {
                    meta: Some(meta),
                    tx_count: 0,
                },
            )),
        };
        abci_application.event_bus.notify_sync(evt.clone());

        // Await delivery
        let received = tokio::time::timeout(std::time::Duration::from_secs(1), handle.recv())
            .await
            .expect("timed out waiting for event");

        assert_eq!(received, Some(evt));
    }
}
