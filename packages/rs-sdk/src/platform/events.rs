use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::platform::v0::platform_subscription_request::{
    PlatformSubscriptionRequestV0, Version as RequestVersion,
};
use dapi_grpc::platform::v0::platform_subscription_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    PlatformFilterV0, PlatformSubscriptionRequest, PlatformSubscriptionResponse,
};
use dapi_grpc::tonic::{Request, Streaming};
use rs_dapi_client::transport::{create_channel, PlatformGrpcClient};
use rs_dapi_client::{RequestSettings, Uri};
use std::time::Duration;
pub type EventSubscriptionId = String;
impl crate::Sdk {
    /// Subscribe to Platform events using the gRPC streaming API.
    ///
    /// Returns the server-assigned subscription id alongside the streaming response.
    pub async fn subscribe_platform_events(
        &self,
        filter: PlatformFilterV0,
    ) -> Result<Streaming<PlatformSubscriptionResponse>, crate::Error> {
        let address = self
            .address_list()
            .get_live_address()
            .ok_or_else(|| crate::Error::SubscriptionError("no live DAPI address".to_string()))?;
        let uri: Uri = address.uri().clone();

        tracing::debug!(
            address = ?uri,
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

        // Keepalive should be less than the timeout to avoid unintentional disconnects.
        let keepalive = (settings.timeout - Duration::from_secs(5))
            .as_secs()
            .clamp(25, 300) as u32;
        let request = PlatformSubscriptionRequest {
            version: Some(RequestVersion::V0(PlatformSubscriptionRequestV0 {
                filter: Some(filter),
                keepalive,
            })),
        };

        let response = client
            .subscribe_platform_events(Request::new(request))
            .await
            .map_err(|e| crate::Error::SubscriptionError(format!("subscribe: {}", e)))?
            .into_inner();

        Ok(response)
    }
}

/// Trait for managing subscriptions.
pub trait Subscription {
    /// Get the subscription id associated with this response.
    ///
    /// Returns an empty string if no subscription id is available.
    fn subscription_id(&self) -> EventSubscriptionId;
}

impl Subscription for PlatformSubscriptionResponse {
    fn subscription_id(&self) -> EventSubscriptionId {
        match &self.version {
            Some(ResponseVersion::V0(v0)) => v0.client_subscription_id.clone(),
            _ => "".to_string(),
        }
    }
}
