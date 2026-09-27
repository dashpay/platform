use super::proxy::Socks5Connector;
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

/// Connect budget through a proxy when the settings give none: tonic applies
/// it around the SOCKS5 handshake and TLS, while a request's own deadline
/// only starts once the connection is up, so a proxy that accepts the TCP
/// connection and never answers would otherwise hang the request. Dash Core
/// waits as long for each SOCKS5 reply.
const PROXY_CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// backon::Sleeper
// #[derive(Default, Clone, Debug)]
pub type TokioBackonSleeper = backon::TokioSleeper;

/// HTTP/2 PING interval while a request is in flight on a connection.
const HTTP2_KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);
/// How long to wait for a PING acknowledgement before closing the connection.
const HTTP2_KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(10);

/// Create channel (connection) for gRPC transport.
///
/// With a proxy in `settings`, every connection of the channel is tunnelled
/// through it.
pub fn create_channel(
    uri: Uri,
    settings: Option<&AppliedRequestSettings>,
) -> Result<Channel, TransportError> {
    let host = unbracketed(uri.host().expect("Failed to get host from URI")).to_string();

    let mut builder = Channel::builder(uri);

    // Start with webpki roots (bundled Mozilla certificates) which work on all platforms
    // Try to add native roots only on platforms where they're available (not iOS)
    let mut tls_config = ClientTlsConfig::new()
        .with_webpki_roots()
        .assume_http2(true)
        // Without an explicit name tonic takes `Uri::host`, which keeps the
        // brackets of an IPv6 literal and is not a valid TLS server name.
        .domain_name(host);

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

    let proxy = settings.and_then(|settings| settings.proxy.clone());
    if let Some(settings) = settings {
        let connect_timeout = settings
            .connect_timeout
            .or(proxy.is_some().then_some(PROXY_CONNECT_TIMEOUT));
        if let Some(timeout) = connect_timeout {
            builder = builder.connect_timeout(timeout);
        }

        if let Some(pem) = settings.ca_certificate.as_ref() {
            let cert = Certificate::from_pem(pem);
            tls_config = tls_config.ca_certificate(cert);
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

    builder = builder.tls_config(tls_config).map_err(|e| {
        TransportError::Grpc(dapi_grpc::tonic::Status::invalid_argument(format!(
            "invalid TLS configuration: {e}"
        )))
    })?;

    Ok(match proxy {
        None => builder.connect_lazy(),
        Some(proxy) => builder.connect_with_connector_lazy(Socks5Connector(proxy)),
    })
}

/// The host of a URI without the brackets an IPv6 literal carries in it
/// (`[2001:db8::1]` becomes `2001:db8::1`); any other host is unchanged.
pub(crate) fn unbracketed(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{ProxyEndpoint, Socks5Auth, Socks5Proxy};
    use crate::RequestSettings;

    #[test]
    fn should_strip_only_ipv6_brackets() {
        assert_eq!(unbracketed("[2001:db8::1]"), "2001:db8::1");
        assert_eq!(unbracketed("127.0.0.1"), "127.0.0.1");
        assert_eq!(unbracketed("evo.example"), "evo.example");
    }

    /// `create_channel` used to panic for every IPv6 endpoint: the bracketed
    /// host is not a valid TLS server name.
    #[tokio::test]
    async fn should_create_a_channel_for_an_ipv6_endpoint() {
        let uri: Uri = "https://[2001:db8::1]:443".parse().expect("uri");
        create_channel(uri.clone(), None).expect("default roots");

        let settings = RequestSettings::default()
            .finalize()
            .with_ca_certificate(Some(Certificate::from_pem("fake-pem-data")));
        create_channel(uri.clone(), Some(&settings)).expect("explicit CA");

        let settings = settings.with_proxy(Some(Socks5Proxy {
            endpoint: ProxyEndpoint::Tcp("127.0.0.1:9050".parse().expect("address")),
            auth: Socks5Auth::None,
        }));
        create_channel(uri, Some(&settings)).expect("through a proxy");
    }
}
