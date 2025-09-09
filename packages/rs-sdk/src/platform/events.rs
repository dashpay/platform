use std::sync::Arc;
use dapi_grpc::platform::v0::PlatformFilterV0;
use rs_dash_notify::platform_mux::{PlatformEventsMux, PlatformEventsSubscriptionHandle, PlatformMuxSettings};

impl crate::Sdk {
    /// Subscribe to Platform events and receive a raw EventBus handle. The
    /// upstream subscription is removed automatically (RAII) when the last
    /// clone of the handle is dropped.
    pub async fn subscribe_platform_events(
        &self,
        filter: PlatformFilterV0,
    ) -> Result<(String, PlatformEventsSubscriptionHandle), crate::Error> {
        use once_cell::sync::OnceCell;
        static MUX: OnceCell<Arc<PlatformEventsMux>> = OnceCell::new();
        let mux = if let Some(m) = MUX.get() { m.clone() } else {
            let settings = PlatformMuxSettings { upstream_conn_count: 2 };
            let m = PlatformEventsMux::new(self.address_list().clone(), settings)
                .map_err(|e| crate::Error::DapiClientError(format!("mux init: {}", e)))?;
            let m = Arc::new(m);
            let _ = MUX.set(m.clone());
            m
        };
        let (id, handle) = mux
            .subscribe(filter)
            .await
            .map_err(|e| crate::Error::DapiClientError(format!("subscribe: {}", e)))?;
        Ok((id, handle))
    }
}
