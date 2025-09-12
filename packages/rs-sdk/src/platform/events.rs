use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::platform::v0::PlatformFilterV0;
use rs_dapi_client::transport::{create_channel, PlatformGrpcClient};
use rs_dapi_client::{DapiRequestExecutor, Uri};
use rs_dash_notify::GrpcPlatformEventsProducer;
use rs_dash_notify::{EventMux, PlatformEventsSubscriptionHandle};
use std::sync::Arc;
use tokio::task::yield_now;
use tracing::Event;

impl crate::Sdk {
    pub(crate) async fn get_event_mux(&self) -> Result<EventMux, crate::Error> {
        use once_cell::sync::OnceCell;
        static MUX: OnceCell<EventMux> = OnceCell::new();

        if let Some(mux) = MUX.get() {
            return Ok(mux.clone());
        }

        let mux = EventMux::new();

        // Build a gRPC client to a live address
        let address = self
            .address_list()
            .get_live_address()
            .ok_or_else(|| crate::Error::SubscriptionError("no live DAPI address".to_string()))?;
        let uri: Uri = address.uri().clone();

        tracing::debug!(address = ?uri, "creating gRPC client for platform events");
        let channel = create_channel(uri, None)
            .map_err(|e| crate::Error::SubscriptionError(format!("channel: {e}")))?;
        let client: PlatformGrpcClient = PlatformClient::new(channel);

        // Spawn the producer bridge
        let worker_mux = mux.clone();
        tracing::debug!("spawning platform events producer task");
        self.spawn(async move {
            let inner_mux = worker_mux.clone();
            tracing::debug!("starting platform events producer task GrpcPlatformEventsProducer");
            if let Err(e) = GrpcPlatformEventsProducer::run(inner_mux, client).await {
                tracing::error!("platform events producer terminated: {}", e);
            }
        })
        .await;

        let _ = MUX.set(mux.clone());

        Ok(mux)
    }

    /// Subscribe to Platform events and receive a raw EventBus handle. The
    /// upstream subscription is removed automatically (RAII) when the last
    /// clone of the handle is dropped.
    pub async fn subscribe_platform_events(
        &self,
        filter: PlatformFilterV0,
    ) -> Result<(String, PlatformEventsSubscriptionHandle), crate::Error> {
        // Initialize global mux with a single upstream producer on first use
        let mux = self.get_event_mux().await?;

        let (id, handle) = mux
            .subscribe(filter)
            .await
            .map_err(|e| crate::Error::SubscriptionError(format!("subscribe: {}", e)))?;
        Ok((id, handle))
    }
}
