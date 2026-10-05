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
use std::hash::BuildHasher;
use std::net::{IpAddr, SocketAddr};
#[cfg(unix)]
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
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
/// complete the SOCKS5 negotiation, method selection and any
/// authentication (Dash Core's default `-timeout`). Running out of it is a
/// [ProxyError]; the rest of the connect budget covers the CONNECT reply.
const PROXY_ANSWER_TIMEOUT: Duration = Duration::from_secs(5);

/// Connect budget through a proxy when the settings give none: a request's
/// own deadline only starts once the connection is up, so a proxy that
/// accepts the TCP connection and never answers would otherwise hang the
/// request. Dash Core waits as long for each SOCKS5 reply.
pub(crate) const PROXY_CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// Stand-in for a connect budget too large to add to the current instant.
const UNLIMITED: Duration = Duration::from_secs(100 * 365 * 24 * 60 * 60);

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
/// timeout once the proxy has completed the SOCKS5 negotiation (the CONNECT
/// reply is pending): a proxy that holds its reply while it builds the route (Tor does while bootstrapping) cannot
/// be told apart from a slow node, and Dash Core counts it against the peer
/// too. A proxy that does not accept the TCP connection, or does not finish
/// the SOCKS5 negotiation (method selection and any authentication), within
/// five seconds or the connect budget, whichever is shorter, is the proxy's
/// failure: that is a proxy that is stuck (a suspended Tor), not a route
/// being built.
#[derive(Debug, thiserror::Error)]
#[error("SOCKS5 proxy {proxy}: {source}")]
pub struct ProxyError {
    proxy: String,
    #[source]
    source: BoxError,
}

/// A process-local keyed hash of a proxy password, so that two fixed
/// passwords never share a pooled connection (Tor isolates streams by
/// username and password) while the pool key never holds the password or a
/// digest of it that could be checked offline.
fn password_identity(password: &str) -> u64 {
    static KEY: OnceLock<std::collections::hash_map::RandomState> = OnceLock::new();
    KEY.get_or_init(Default::default).hash_one(password)
}

impl Socks5Proxy {
    /// The part of the connection pool key this proxy contributes. Never
    /// includes the password, only a process-local keyed hash of it.
    pub(crate) fn connection_key(&self) -> String {
        let auth = match &self.auth {
            Socks5Auth::None => "none".to_string(),
            Socks5Auth::Password { username, password } => {
                format!("password {username} {:016x}", password_identity(password))
            }
            Socks5Auth::RandomPerConnection => "random".to_string(),
        };
        format!("{} {auth}", self.endpoint)
    }

    /// Opens a tunnel to `uri`'s host through the proxy by `deadline`: the
    /// proxy gets [PROXY_ANSWER_TIMEOUT] (or less, if the deadline is
    /// closer) to accept and negotiate, and running out of it is the proxy's
    /// failure; the CONNECT reply gets what is left, and running out of that
    /// is the destination's.
    async fn connect(
        self,
        uri: Uri,
        started: tokio::time::Instant,
        deadline: tokio::time::Instant,
    ) -> Result<Box<dyn ProxiedStream>, BoxError> {
        let target = target(&uri)?;
        let negotiated_by = deadline.min(started + PROXY_ANSWER_TIMEOUT);
        let proxy_error = |source: BoxError| ProxyError {
            proxy: self.endpoint.to_string(),
            source,
        };
        let not_accepted = || proxy_error("the proxy did not accept the connection in time".into());
        match &self.endpoint {
            ProxyEndpoint::Tcp(address) => {
                let socket = tokio::time::timeout_at(negotiated_by, TcpStream::connect(address))
                    .await
                    .map_err(|_| not_accepted())?
                    .map_err(|e| proxy_error(e.into()))?;
                socket
                    .set_nodelay(true)
                    .map_err(|e| proxy_error(e.into()))?;
                let stream = self
                    .handshake_timed(socket, target, negotiated_by, deadline, proxy_error)
                    .await?;
                Ok(Box::new(stream))
            }
            #[cfg(unix)]
            ProxyEndpoint::Unix(path) => {
                let socket = tokio::time::timeout_at(negotiated_by, UnixStream::connect(path))
                    .await
                    .map_err(|_| not_accepted())?
                    .map_err(|e| proxy_error(e.into()))?;
                let stream = self
                    .handshake_timed(socket, target, negotiated_by, deadline, proxy_error)
                    .await?;
                Ok(Box::new(stream))
            }
        }
    }

    /// [handshake](Self::handshake), failing as the proxy's when the
    /// negotiation (method selection and any authentication) is not complete
    /// by `negotiated_by`, and as the destination's when the CONNECT reply,
    /// which only then is asked for, is not in by `deadline`.
    async fn handshake_timed<S: AsyncRead + AsyncWrite + Send + Unpin + 'static>(
        &self,
        socket: S,
        target: TargetAddr<'static>,
        negotiated_by: tokio::time::Instant,
        deadline: tokio::time::Instant,
        proxy_error: impl Fn(BoxError) -> ProxyError,
    ) -> Result<S, BoxError> {
        let negotiated = Arc::new(AtomicBool::new(false));
        let socket = Negotiation::new(socket, negotiated.clone());
        let handshake = Box::pin(self.handshake(socket, target, &proxy_error));
        let stalled = || -> BoxError {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "no SOCKS5 CONNECT reply within the connect timeout",
            )
            .into()
        };
        match select(handshake, Box::pin(tokio::time::sleep_until(negotiated_by))).await {
            Either::Left((result, _)) => result,
            Either::Right(((), handshake)) if negotiated.load(Ordering::Relaxed) => {
                tokio::time::timeout_at(deadline, handshake)
                    .await
                    .map_err(|_| stalled())?
            }
            Either::Right(_) => {
                Err(proxy_error("no complete SOCKS5 negotiation in time".into()).into())
            }
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

/// A stream to the proxy that notes when the SOCKS5 negotiation is
/// complete: the proxy's method selection (RFC 1928 §3, two bytes) and, when
/// it chose username/password, its authentication status (RFC 1929, two
/// more). Only then does the client send CONNECT, so a stall before that is
/// the proxy's, not the destination's.
struct Negotiation<S> {
    inner: S,
    negotiated: Arc<AtomicBool>,
    /// Bytes the proxy has sent so far, up to the end of the negotiation.
    received: usize,
    /// Bytes the negotiation takes: 2, or 4 once username/password is chosen.
    expected: usize,
}

impl<S> Negotiation<S> {
    fn new(inner: S, negotiated: Arc<AtomicBool>) -> Self {
        Self {
            inner,
            negotiated,
            received: 0,
            expected: 2,
        }
    }

    fn note(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if self.received >= self.expected {
                break;
            }
            // The second byte of the method selection is the method chosen.
            if self.received == 1 && byte == 0x02 {
                self.expected = 4;
            }
            self.received += 1;
        }
        if self.received >= self.expected {
            self.negotiated.store(true, Ordering::Relaxed);
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Negotiation<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let poll = Pin::new(&mut self.inner).poll_read(cx, buf);
        if buf.filled().len() > before && self.received < self.expected {
            self.note(&buf.filled()[before..]);
        }
        poll
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Negotiation<S> {
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

/// The tonic connector that tunnels every connection through the proxy
/// within the connect budget (see [Socks5Proxy::connect]).
///
/// The channel's own connect timeout, set to the same budget, bounds the
/// tunnel and the TLS handshake after it together. Its clock starts when
/// the connection future is first polled, after [call](tower_service::Service::call)
/// returns, and it polls the connection before checking its own deadline, so
/// the connector's deadlines, fixed in `call`, always fire first: a proxy
/// that stalls the negotiation is still reported as a [ProxyError].
#[derive(Clone)]
pub(crate) struct Socks5Connector {
    pub(crate) proxy: Socks5Proxy,
    pub(crate) connect_timeout: Duration,
}

impl tower_service::Service<Uri> for Socks5Connector {
    type Response = TokioIo<Box<dyn ProxiedStream>>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let proxy = self.proxy.clone();
        let started = tokio::time::Instant::now();
        // A budget too large to add (an "unlimited" Duration::MAX) is
        // capped rather than overflowing.
        let deadline = started
            .checked_add(self.connect_timeout)
            .unwrap_or_else(|| started + UNLIMITED);
        Box::pin(async move {
            proxy
                .connect(uri, started, deadline)
                .await
                .map(TokioIo::new)
        })
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
        assert_eq!(with("one"), with("one"));
        assert_ne!(with("one"), with("two"));
        assert!(!with("hunter2").contains("hunter2"));
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
