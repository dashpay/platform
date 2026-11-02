use super::proxy::{Socks5Connector, PROXY_CONNECT_TIMEOUT};
use super::TransportError;
use crate::{request_settings::AppliedRequestSettings, Uri};
use dapi_grpc::core::v0::core_client::CoreClient;
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::tonic::transport::{Certificate, Channel, ClientTlsConfig};
use std::error::Error as _;
use std::sync::Arc;
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
    let connect_timeout = settings.and_then(AppliedRequestSettings::effective_connect_timeout);
    if let Some(pem) = settings.and_then(|settings| settings.ca_certificate.as_ref()) {
        tls_config = tls_config.ca_certificate(Certificate::from_pem(pem));
    }

    // Ping only while a request is in flight: a connection whose network path
    // died mid-response is then closed within interval + timeout, failing its
    // streams instead of leaving them waiting for data that never comes.
    // Idle pooled connections are not pinged.
    builder = builder
        .http2_keep_alive_interval(HTTP2_KEEP_ALIVE_INTERVAL)
        .keep_alive_timeout(HTTP2_KEEP_ALIVE_TIMEOUT)
        .keep_alive_while_idle(false);

    builder = builder.tls_config(tls_config).map_err(invalid_tls_config)?;

    Ok(match proxy {
        None => {
            if let Some(timeout) = connect_timeout {
                builder = builder.connect_timeout(timeout);
            }
            builder.connect_lazy()
        }
        Some(proxy) => {
            // One absolute deadline for the whole connection: tonic applies
            // it around the connector, so it covers the SOCKS5 tunnel and the
            // TLS handshake after it. The connector times its own phases
            // against the same budget from a start that is never later than
            // tonic's, so it still tells a stuck proxy from a slow
            // destination before this deadline fires.
            let connect_timeout = connect_timeout.unwrap_or(PROXY_CONNECT_TIMEOUT);
            builder
                .connect_timeout(connect_timeout)
                .connect_with_connector_lazy(Socks5Connector {
                    proxy,
                    connect_timeout,
                })
        }
    })
}

/// A TLS configuration tonic rejects, as an `InvalidArgument` status that
/// keeps the cause: tonic's own message names only the error kind, the cause
/// (an invalid server name, a malformed certificate) is in its source chain.
fn invalid_tls_config(error: dapi_grpc::tonic::transport::Error) -> TransportError {
    let mut message = format!("invalid TLS configuration: {error}");
    let mut cause = error.source();
    while let Some(current) = cause {
        message.push_str(&format!(": {current}"));
        cause = current.source();
    }
    let mut status = dapi_grpc::tonic::Status::invalid_argument(message);
    status.set_source(Arc::new(error));
    TransportError::Grpc(status)
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

    #[test]
    fn should_keep_the_cause_of_an_invalid_tls_configuration() {
        // Not a valid TLS server name.
        let uri: Uri = "https://foo..bar:1443".parse().expect("uri");
        let Err(TransportError::Grpc(status)) = create_channel(uri, None) else {
            panic!("an invalid server name must be rejected");
        };
        assert_eq!(status.code(), dapi_grpc::tonic::Code::InvalidArgument);
        assert!(
            status.message().contains("invalid dns name"),
            "{}",
            status.message()
        );
        assert!(
            status.source().is_some(),
            "the tonic error stays in the source chain"
        );
    }
}
