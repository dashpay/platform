//! This crate provides [DapiClient] --- client for a decentralized API for Dash.

#![deny(missing_docs)]

mod platform;
mod settings;

use dapi_grpc::platform::v0::platform_client::PlatformClient;
use futures::future::BoxFuture;

use settings::AppliedSettings;
pub use settings::Settings;
use tonic::transport::Channel;

/// DAPI request.
pub trait DapiRequest {
    /// Response type for the request.
    type DapiResponse;

    /// Settings that will override [DapiClient]'s ones each time the request is executed.
    const SETTINGS_OVERRIDES: Settings;

    /// Error type for the request.
    type Error;

    /// Transport that is used to execute the request.
    type Transport;

    /// Converts the DAPI request to it's transport-specific [Future].
    fn prepare<'c>(
        &self,
        client: &'c mut Self::Transport,
        settings: AppliedSettings,
    ) -> BoxFuture<'c, Result<Self::DapiResponse, Self::Error>>;
}

/// DAPI error variants when using gRPC transport.
#[derive(Debug, thiserror::Error)]
pub enum GrpcRequestError {
    /// gRPC transport error
    #[error("gRPC transport error")]
    Fetch(#[from] tonic::Status),
    /// Proto message is correct, however it's optional fields failed our expectations.
    #[error("gRPC response is insufficient")]
    IncompleteResponse,
}

/// Access point to DAPI.
#[derive(Debug)]
pub struct DapiClient {
    settings: Settings,
}

impl DapiClient {
    /// Execute the [Request] handling
    pub async fn execute<'c, R: DapiRequest>(
        &'c mut self,
        request: R,
        settings: Settings,
    ) -> Result<R::DapiResponse, R::Error>
    where
        DapiClient: GetTransport<<R as DapiRequest>::Transport>,
    {
        let applied_settings = self
            .settings
            .override_by(R::SETTINGS_OVERRIDES)
            .override_by(settings)
            .finalize();

        let mut result = request
            .prepare(
                GetTransport::<R::Transport>::get_transport(self),
                applied_settings,
            )
            .await;

        // TODO: exp/fib ?
        for _ in 0..applied_settings.retries {
            if result.is_ok() {
                break;
            }

            result = request
                .prepare(
                    GetTransport::<R::Transport>::get_transport(self),
                    applied_settings,
                )
                .await;
        }

        result
    }
}

/// A mean for [DapiClient] to get a transport required for a request.
pub trait GetTransport<T> {
    /// Get suitable transport.
    fn get_transport(&mut self) -> &mut T;
}

impl GetTransport<PlatformClient<Channel>> for DapiClient {
    fn get_transport(&mut self) -> &mut PlatformClient<Channel> {
        todo!()
    }
}
