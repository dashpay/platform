//! SOCKS5 proxy (RFC 1928) for DAPI connections.
//!
//! A [DapiClient](crate::DapiClient) configured with a [Socks5Proxy] opens
//! every connection through it: the evonode's host goes to the proxy
//! unresolved (an IP literal as such, anything else as a domain name the
//! proxy resolves), so there is never a local DNS lookup for an endpoint,
//! and TLS still runs end to end with the evonode, validated against the
//! endpoint host. There is no fallback to a direct connection.

use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
#[cfg(unix)]
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use dapi_grpc::tonic::Status;
use futures::future::{select, Either};
use hyper_util::rt::TokioIo;
use rand::RngCore;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;
use tokio_socks::tcp::Socks5Stream;
use tokio_socks::{Error as SocksError, TargetAddr};

use super::tonic_channel::unbracketed;
use crate::Uri;

type BoxError = Box<dyn StdError + Send + Sync>;

/// How long the proxy itself may take to accept the connection and to
/// answer the SOCKS5 greeting (Dash Core's default `-timeout`). Running out
/// of it is a [ProxyError]; the channel's connect timeout covers the rest.
const PROXY_ANSWER_TIMEOUT: Duration = Duration::from_secs(5);

/// A SOCKS5 proxy every DAPI connection is tunnelled through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Socks5Proxy {
    /// Where the proxy listens.
    pub endpoint: ProxyEndpoint,
    /// How the client authenticates to the proxy.
    pub auth: Socks5Auth,
}

/// Where a [Socks5Proxy] listens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyEndpoint {
    /// A TCP address. Numeric: resolving the proxy's own name is up to the
    /// caller.
    Tcp(SocketAddr),
    /// A Unix domain socket.
    #[cfg(unix)]
    Unix(PathBuf),
}

/// How a client authenticates to a [Socks5Proxy].
///
/// With credentials, the client offers both "no authentication" and
/// username/password (RFC 1929) and accepts whichever the proxy picks.
#[derive(Clone, PartialEq, Eq)]
pub enum Socks5Auth {
    /// No credentials.
    None,
    /// Fixed username and password, 1 to 255 bytes each.
    Password {
        /// Username.
        username: String,
        /// Password.
        password: String,
    },
    /// Fresh random credentials for every new connection. Tor isolates
    /// streams by SOCKS credentials by default (`IsolateSOCKSAuth`), so each
    /// connection gets its own circuit.
    RandomPerConnection,
}

/// A request failed because the proxy failed, not the evonode: the proxy
/// is unreachable, broke the SOCKS5 protocol, refused the credentials, or
/// answered with a general failure or an unsupported command. It is carried
/// in the source chain of the transport error; see
/// [TransportError::is_proxy_failure](super::TransportError::is_proxy_failure).
///
/// Such a failure is not retried and does not ban the evonode: every
/// endpoint shares the proxy. That includes a general failure, which Tor
/// also sends when a single circuit fails; the request fails at once rather
/// than being retried on another node without a ban, and the next request
/// (with [Socks5Auth::RandomPerConnection], on a fresh circuit) tries again.
///
/// A reply about the destination (unreachable, refused, TTL expired, not
/// allowed by the proxy's rules, address type not supported) produces an
/// ordinary, retryable connection error instead, as does the connect
/// timeout once the proxy has accepted the connection: a proxy that holds
/// its reply while it builds the route (Tor does while bootstrapping) cannot
/// be told apart from a slow node, and Dash Core counts it against the peer
/// too. A proxy that does not accept the TCP connection, or does not answer
/// the SOCKS5 greeting, within five seconds is the proxy's failure: that is
/// a proxy that is stuck (a suspended Tor), not a route being built.
#[derive(Debug, thiserror::Error)]
#[error("SOCKS5 proxy {proxy}: {source}")]
pub struct ProxyError {
    proxy: String,
    #[source]
    source: BoxError,
}

impl Socks5Proxy {
    /// The part of the connection pool key this proxy contributes. Never
    /// includes the password.
    pub(crate) fn connection_key(&self) -> String {
        let auth = match &self.auth {
            Socks5Auth::None => "none".to_string(),
            Socks5Auth::Password { username, .. } => format!("password {username}"),
            Socks5Auth::RandomPerConnection => "random".to_string(),
        };
        format!("{} {auth}", self.endpoint)
    }

    /// Opens a tunnel to `uri`'s host through the proxy.
    async fn connect(self, uri: Uri) -> Result<Box<dyn ProxiedStream>, BoxError> {
        let target = target(&uri)?;
        let proxy_error = |source: BoxError| ProxyError {
            proxy: self.endpoint.to_string(),
            source,
        };
        match &self.endpoint {
            ProxyEndpoint::Tcp(address) => {
                let socket =
                    tokio::time::timeout(PROXY_ANSWER_TIMEOUT, TcpStream::connect(address))
                        .await
                        .map_err(|e| proxy_error(e.into()))?
                        .map_err(|e| proxy_error(e.into()))?;
                socket
                    .set_nodelay(true)
                    .map_err(|e| proxy_error(e.into()))?;
                let stream = self.handshake_answered(socket, target, proxy_error).await?;
                Ok(Box::new(stream))
            }
            #[cfg(unix)]
            ProxyEndpoint::Unix(path) => {
                let socket = UnixStream::connect(path)
                    .await
                    .map_err(|e| proxy_error(e.into()))?;
                let stream = self.handshake_answered(socket, target, proxy_error).await?;
                Ok(Box::new(stream))
            }
        }
    }

    /// [handshake](Self::handshake), failing as the proxy's when it has not
    /// answered the greeting within [PROXY_ANSWER_TIMEOUT]. Once it has, the
    /// CONNECT reply is waited for under the channel's connect timeout.
    async fn handshake_answered<S: AsyncRead + AsyncWrite + Send + Unpin + 'static>(
        &self,
        socket: S,
        target: TargetAddr<'static>,
        proxy_error: impl Fn(BoxError) -> ProxyError,
    ) -> Result<S, BoxError> {
        let answered = Arc::new(AtomicBool::new(false));
        let socket = Answered {
            inner: socket,
            answered: answered.clone(),
        };
        let handshake = Box::pin(self.handshake(socket, target, &proxy_error));
        match select(
            handshake,
            Box::pin(tokio::time::sleep(PROXY_ANSWER_TIMEOUT)),
        )
        .await
        {
            Either::Left((result, _)) => result,
            Either::Right(((), handshake)) if answered.load(Ordering::Relaxed) => handshake.await,
            Either::Right(_) => Err(proxy_error("no answer to the SOCKS5 greeting".into()).into()),
        }
        .map(|stream| stream.inner)
    }

    async fn handshake<S: AsyncRead + AsyncWrite + Send + Unpin + 'static>(
        &self,
        socket: S,
        target: TargetAddr<'static>,
        proxy_error: impl Fn(BoxError) -> ProxyError,
    ) -> Result<S, BoxError> {
        let result = match &self.auth {
            Socks5Auth::None => Socks5Stream::connect_with_socket(socket, target.clone()).await,
            Socks5Auth::Password { username, password } => {
                Socks5Stream::connect_with_password_and_socket(
                    socket,
                    target.clone(),
                    username,
                    password,
                )
                .await
            }
            Socks5Auth::RandomPerConnection => {
                let (username, password) = (random_credential(), random_credential());
                Socks5Stream::connect_with_password_and_socket(
                    socket,
                    target.clone(),
                    &username,
                    &password,
                )
                .await
            }
        };
        match result {
            Ok(stream) => Ok(stream.into_inner()),
            // Replies about the destination: the evonode, or the route to it,
            // failed. Tor reports its exit and entry policy refusals as "not
            // allowed by ruleset".
            Err(
                e @ (SocksError::HostUnreachable
                | SocksError::ConnectionRefused
                | SocksError::NetworkUnreachable
                | SocksError::TtlExpired
                | SocksError::ConnectionNotAllowedByRuleset
                | SocksError::AddressTypeNotSupported),
            ) => Err(e.into()),
            // Everything else is the proxy's, including a general failure,
            // which is how Tor answers while it cannot build circuits, and an
            // invalid address, which can only come from the proxy's reply
            // since the target was checked before connecting.
            Err(e) => Err(proxy_error(e.into()).into()),
        }
    }
}

/// The CONNECT target for `uri`: an IP literal as an address, any other
/// host as a domain name for the proxy to resolve. An unusable host is the
/// endpoint's fault, not the proxy's.
fn target(uri: &Uri) -> Result<TargetAddr<'static>, BoxError> {
    let host = unbracketed(uri.host().ok_or("the endpoint URI has no host")?);
    let default_port = match uri.scheme_str() {
        Some("http") => 80,
        _ => 443,
    };
    let port = uri.port_u16().unwrap_or(default_port);
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(TargetAddr::Ip(SocketAddr::new(ip, port)));
    }
    if host.len() > 255 {
        return Err(format!("the endpoint host is longer than 255 bytes: {host}").into());
    }
    Ok(TargetAddr::Domain(host.to_string().into(), port))
}

/// 16 random bytes, hex encoded.
fn random_credential() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Whether `status` failed because of the proxy (a [ProxyError] is in its
/// source chain).
pub(crate) fn is_proxy_failure(status: &Status) -> bool {
    let mut error: Option<&(dyn StdError + 'static)> = Some(status);
    while let Some(current) = error {
        if current.is::<ProxyError>() {
            return true;
        }
        error = current.source();
    }
    false
}

impl fmt::Display for ProxyEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyEndpoint::Tcp(address) => write!(f, "{address}"),
            #[cfg(unix)]
            ProxyEndpoint::Unix(path) => write!(f, "unix:{}", path.display()),
        }
    }
}

// Request settings, and with them the proxy, are logged at debug level.
impl fmt::Debug for Socks5Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Socks5Auth::None => write!(f, "None"),
            Socks5Auth::Password { username, .. } => f
                .debug_struct("Password")
                .field("username", username)
                .field("password", &"<redacted>")
                .finish(),
            Socks5Auth::RandomPerConnection => write!(f, "RandomPerConnection"),
        }
    }
}

/// A stream to the proxy that notes when the proxy first sends anything.
struct Answered<S> {
    inner: S,
    answered: Arc<AtomicBool>,
}

impl<S: AsyncRead + Unpin> AsyncRead for Answered<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let poll = Pin::new(&mut self.inner).poll_read(cx, buf);
        if buf.filled().len() > before {
            self.answered.store(true, Ordering::Relaxed);
        }
        poll
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Answered<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }
}

/// A connected tunnel, TCP or Unix underneath.
pub(crate) trait ProxiedStream: AsyncRead + AsyncWrite + Send + Unpin {}
impl<S: AsyncRead + AsyncWrite + Send + Unpin> ProxiedStream for S {}

/// The tonic connector that tunnels every connection through the proxy.
#[derive(Clone)]
pub(crate) struct Socks5Connector(pub(crate) Socks5Proxy);

impl tower_service::Service<Uri> for Socks5Connector {
    type Response = TokioIo<Box<dyn ProxiedStream>>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let proxy = self.0.clone();
        Box::pin(async move { proxy.connect(uri).await.map(TokioIo::new) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proxy(auth: Socks5Auth) -> Socks5Proxy {
        Socks5Proxy {
            endpoint: ProxyEndpoint::Tcp("127.0.0.1:9050".parse().expect("address")),
            auth,
        }
    }

    #[test]
    fn should_redact_the_password_in_debug_output() {
        let debug = format!(
            "{:?}",
            proxy(Socks5Auth::Password {
                username: "alice".to_string(),
                password: "hunter2".to_string(),
            })
        );
        assert!(debug.contains("alice"), "{debug}");
        assert!(!debug.contains("hunter2"), "{debug}");
    }

    #[test]
    fn should_leave_the_password_out_of_the_connection_key() {
        let with = |password: &str| {
            proxy(Socks5Auth::Password {
                username: "alice".to_string(),
                password: password.to_string(),
            })
            .connection_key()
        };
        assert_eq!(with("one"), with("two"));
        assert!(!with("one").contains("one"));
        assert_ne!(
            proxy(Socks5Auth::None).connection_key(),
            proxy(Socks5Auth::RandomPerConnection).connection_key()
        );
    }

    #[test]
    fn should_draw_fresh_hex_credentials() {
        let (first, second) = (random_credential(), random_credential());
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn should_send_ip_literals_as_addresses_and_names_as_domains() {
        let target = |uri: &str| target(&uri.parse().expect("uri"));
        assert_eq!(
            target("https://[2001:db8::1]:1443").expect("ipv6"),
            TargetAddr::Ip("[2001:db8::1]:1443".parse().expect("address"))
        );
        assert_eq!(
            target("https://10.0.0.1").expect("default port"),
            TargetAddr::Ip("10.0.0.1:443".parse().expect("address"))
        );
        assert_eq!(
            target("https://evo.example:1443").expect("name"),
            TargetAddr::Domain("evo.example".into(), 1443)
        );
        let overlong = format!("https://{}.example", "a".repeat(250));
        assert!(target(&overlong).is_err());
    }

    #[test]
    fn should_find_a_proxy_failure_in_the_source_chain() {
        let error = ProxyError {
            proxy: "127.0.0.1:9050".to_string(),
            source: "refused".into(),
        };
        let status = Status::from_error(Box::new(error));
        assert!(is_proxy_failure(&status));
        assert!(is_proxy_failure(&status.clone()), "clones keep the source");
        assert!(!is_proxy_failure(&Status::unavailable("node down")));
    }
}
