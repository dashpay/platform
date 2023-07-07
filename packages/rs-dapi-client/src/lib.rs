//! This crate provides [DapiClient] --- client for a decentralized API for Dash.

#![deny(missing_docs)]

mod platform;
mod settings;
mod transport;

use backon::{ExponentialBuilder, Retryable};
use transport::{grpc::PlatformGrpcClient, TransportRequest};

pub use settings::Settings;

/// DAPI request.
pub trait DapiRequest {
    /// Response type for the request.
    type DapiResponse;

    /// Settings that will override [DapiClient]'s ones each time the request is executed.
    const SETTINGS_OVERRIDES: Settings;

    /// Error that may happen during conversion from transport-specific response to the
    /// DAPI response.
    type Error;

    /// 1 to 1 mapping from the DAPI request to a type that represents a way for the data
    /// to be fetched.
    type TransportRequest: TransportRequest;

    /// Get the transport layer request.
    fn to_transport_request(&self) -> Self::TransportRequest;

    /// Attempts to build DAPI response specific to this DAPI request from transport layer data.
    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error>;
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

/// DAPI request error type that wraps either transport or domain (DAPI response parsing) errors.
#[derive(Debug, thiserror::Error)]
pub enum DapiError<TE, PE> {
    /// The error happened on transport layer
    #[error("transport error: {0}")]
    Transport(TE),
    /// Successful transport execution, but was unable to make a conversion from
    /// transport request to DAPI request.
    #[error("response parse error: {0}")]
    ParseResponse(PE),
}

impl DapiClient {
    /// Execute the [Request] handling.
    pub async fn execute<'c, R>(
        &'c mut self,
        request: R,
        settings: Settings,
    ) -> Result<
        R::DapiResponse,
        DapiError<<R::TransportRequest as TransportRequest>::Error, R::Error>,
    >
    // require the existence of transport client from this DAPI client required for the request:
    where
        DapiClient:
            GetTransport<<<R as DapiRequest>::TransportRequest as TransportRequest>::Client>,
        R: DapiRequest,
    {
        // Join settings of different sources to get final version of the settings for this execution:
        let applied_settings = self
            .settings
            .override_by(R::SETTINGS_OVERRIDES)
            .override_by(settings)
            .finalize();

        // Setup retry policy:
        let retry_settings = ExponentialBuilder::default().with_max_times(applied_settings.retries);

        // Setup DAPI request execution routine future. It's a closure that will be called more than
        // once to build new future on each retry:
        let routine = || {
            // Get a transport client requried by the DAPI request from this DAPI client:
            let mut transport_client = GetTransport::<
                <R::TransportRequest as TransportRequest>::Client,
            >::get_transport(self);

            let transport_request = request.to_transport_request();

            async move {
                // On a lower layer DAPI requests should be fulfilled as a transport request first:
                let transport_response = transport_request
                    .execute(&mut transport_client, &applied_settings)
                    .await
                    .map_err(|e| DapiError::<_, <R as DapiRequest>::Error>::Transport(e))?;

                // Next try to build a proper DAPI response if possible:
                let dapi_response =
                    R::try_from_transport_response(transport_response).map_err(|e| {
                        DapiError::<
                            <<R as DapiRequest>::TransportRequest as TransportRequest>::Error,
                            _,
                        >::ParseResponse(e)
                    })?;

                Ok::<_, DapiError<_, _>>(dapi_response)
            }
        };

        // Start the routine with retry policy applied:
        routine.retry(&retry_settings).await
    }
}

/// A mean for [DapiClient] to get a transport required for a request.
/// This is done as a separate trait because currently we cannot share
/// common request execution logic with specific transport logic unless
/// we introduce any trait bound (and we cannot say "there exists method
/// with a type parameter applied" -- but a trait can do).
pub trait GetTransport<T> {
    /// Get suitable transport.
    fn get_transport(&mut self) -> T;
}

impl GetTransport<PlatformGrpcClient> for DapiClient {
    fn get_transport(&mut self) -> PlatformGrpcClient {
        todo!()
    }
}
