use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::platform::v0::platform_subscription_request::{
    PlatformSubscriptionRequestV0, Version as RequestVersion,
};
use dapi_grpc::platform::v0::{
    PlatformFilterV0, PlatformSubscriptionRequest, PlatformSubscriptionResponse,
};
use dapi_grpc::tonic::{Request, Streaming};
use rs_dapi_client::transport::{create_channel, PlatformGrpcClient};
use rs_dapi_client::{RequestSettings, Uri};
use std::fmt::{Debug, Display};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// ID generator with most siginificant bits based on local process info.
static NEXT_SUBSCRIPTION_ID: LazyLock<AtomicU64> = LazyLock::new(|| {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let pid = std::process::id() as u64;

    // 48..63 bits: lower 16 bits of pid
    // 32..47 bits: lower 16 bits ofprocess start time in seconds
    // 0..31 bits: for a counter
    AtomicU64::new(((pid & 0xFFFF) << 48) | (secs & 0xFFFF) << 32)
});

#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct EventSubscriptionId(pub u64);
impl EventSubscriptionId {
    pub fn new() -> Self {
        Self(NEXT_SUBSCRIPTION_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Display for EventSubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Write in format: timepid:counter
        write!(
            f,
            "{:04X}_{:04X}:{}",
            (self.0 >> 48) & 0xffff,
            (self.0 >> 32) & 0xffff,
            self.0 & 0xffff_ffff
        )
    }
}

impl Debug for EventSubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl crate::Sdk {
    /// Subscribe to Platform events using the gRPC streaming API.
    ///
    /// Returns the generated client subscription id alongside the streaming response.
    pub async fn subscribe_platform_events(
        &self,
        filter: PlatformFilterV0,
    ) -> Result<(EventSubscriptionId, Streaming<PlatformSubscriptionResponse>), crate::Error> {
        let subscription_id = EventSubscriptionId::new();

        let address = self
            .address_list()
            .get_live_address()
            .ok_or_else(|| crate::Error::SubscriptionError("no live DAPI address".to_string()))?;
        let uri: Uri = address.uri().clone();

        tracing::debug!(
            address = ?uri,
            %subscription_id,
            "creating gRPC client for platform events subscription"
        );
        let settings = self
            .dapi_client_settings
            .override_by(RequestSettings {
                connect_timeout: Some(Duration::from_secs(5)),
                timeout: Some(Duration::from_secs(3600)),
                ..Default::default()
            })
            .finalize();
        let channel = create_channel(uri, Some(&settings))
            .map_err(|e| crate::Error::SubscriptionError(format!("channel: {e}")))?;
        let mut client: PlatformGrpcClient = PlatformClient::new(channel);

        let request = PlatformSubscriptionRequest {
            version: Some(RequestVersion::V0(PlatformSubscriptionRequestV0 {
                client_subscription_id: subscription_id.to_string(),
                filter: Some(filter),
            })),
        };

        let response = client
            .subscribe_platform_events(Request::new(request))
            .await
            .map_err(|e| crate::Error::SubscriptionError(format!("subscribe: {}", e)))?
            .into_inner();

        Ok((subscription_id, response))
    }
}
