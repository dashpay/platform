use super::TransportError;
use crate::{request_settings::AppliedRequestSettings, Uri};
use dapi_grpc::core::v0::core_client::CoreClient;
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::tonic::transport::{Certificate, Channel, ClientTlsConfig};
use std::time::Duration;

/// Platform Client using gRPC transport.
pub type PlatformGrpcClient = PlatformClient<Channel>;
/// Core Client using gRPC transport.
pub type CoreGrpcClient = CoreClient<Channel>;

/// backon::Sleeper
// #[derive(Default, Clone, Debug)]
pub type TokioBackonSleeper = backon::TokioSleeper;

/// HTTP/2 PING interval while a request is in flight on a connection.
const HTTP2_KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);
/// How long to wait for a PING acknowledgement before closing the connection.
const HTTP2_KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(10);

/// Create channel (connection) for gRPC transport.
pub fn create_channel(
    uri: Uri,
    settings: Option<&AppliedRequestSettings>,
) -> Result<Channel, TransportError> {
    let host = uri.host().expect("Failed to get host from URI").to_string();

    let mut builder = Channel::builder(uri);

    // Start with webpki roots (bundled Mozilla certificates) which work on all platforms
    // Try to add native roots only on platforms where they're available (not iOS)
    let mut tls_config = ClientTlsConfig::new()
        .with_webpki_roots()
        .assume_http2(true);

    // Try to add native roots - this may fail on iOS/Android, which is fine since we have webpki roots
    #[cfg(not(any(
        target_os = "ios",
        target_os = "tvos",
        target_os = "watchos",
        target_os = "android"
    )))]
    {
        tls_config = tls_config.with_native_roots();
    }

    if let Some(settings) = settings {
        if let Some(timeout) = settings.connect_timeout {
            builder = builder.connect_timeout(timeout);
        }

        if let Some(pem) = settings.ca_certificate.as_ref() {
            let cert = Certificate::from_pem(pem);
            tls_config = tls_config.ca_certificate(cert).domain_name(host);
        };
    }

    // Ping only while a request is in flight: a connection whose network path
    // died mid-response is then closed within interval + timeout, failing its
    // streams instead of leaving them waiting for data that never comes.
    // Idle pooled connections are not pinged.
    builder = builder
        .http2_keep_alive_interval(HTTP2_KEEP_ALIVE_INTERVAL)
        .keep_alive_timeout(HTTP2_KEEP_ALIVE_TIMEOUT)
        .keep_alive_while_idle(false);

    builder = builder
        .tls_config(tls_config)
        .expect("Failed to set TLS config");

    Ok(builder.connect_lazy())
}
