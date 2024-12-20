use std::time::Duration;

use super::{CanRetry, TransportClient, TransportError, TransportRequest};
use crate::connection_pool::{ConnectionPool, PoolPrefix};
use crate::{request_settings::AppliedRequestSettings, RequestSettings, Uri};
use dapi_grpc::core::v0::core_client::CoreClient;
use dapi_grpc::core::v0::{self as core_proto};
use dapi_grpc::platform::v0::{self as platform_proto, platform_client::PlatformClient};
use dapi_grpc::tonic::transport::{Certificate, Channel, ClientTlsConfig, Uri};
use dapi_grpc::tonic::{IntoRequest, Streaming};
use futures::{future::BoxFuture, FutureExt, TryFutureExt};

/// Platform Client using gRPC transport.
pub type PlatformGrpcClient = PlatformClient<Channel>;
/// Core Client using gRPC transport.
pub type CoreGrpcClient = CoreClient<Channel>;

/// backon::Sleeper
#[derive(Default, Clone, Debug)]
pub(crate) struct Sleeper(backon::TokioSleeper);

impl backon::Sleeper for Sleeper {
    type Sleep = backon::TokioSleeper::Sleep;
    fn sleep(&self, dur: Duration) -> Self::Sleep {
        self.0.sleep(dur)
    }
}

fn create_channel(
    uri: Uri,
    settings: Option<&AppliedRequestSettings>,
) -> Result<Channel, TransportError> {
    let host = uri.host().expect("Failed to get host from URI").to_string();

    let mut builder = Channel::builder(uri);
    let mut tls_config = ClientTlsConfig::new()
        .with_native_roots()
        .with_webpki_roots()
        .assume_http2(true);

    if let Some(settings) = settings {
        if let Some(timeout) = settings.connect_timeout {
            builder = builder.connect_timeout(timeout);
        }

        if let Some(pem) = settings.ca_certificate.as_ref() {
            let cert = Certificate::from_pem(pem);
            tls_config = tls_config.ca_certificate(cert).domain_name(host);
        };
    }

    builder = builder
        .tls_config(tls_config)
        .expect("Failed to set TLS config");

    Ok(builder.connect_lazy())
}
