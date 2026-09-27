//! End-to-end SOCKS5 proxy suite on loopback: a real `DapiClient` sends a
//! gRPC request through a minimal SOCKS5 server to a TLS gRPC server. The
//! gRPC server serves no methods, so a request that made it all the way
//! (TCP, SOCKS5, TLS, HTTP/2, gRPC) comes back `Unimplemented`.
//!
//! The certificates in `tests/data/proxy/` are described in its README.
#![cfg(not(target_arch = "wasm32"))]

use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dapi_grpc::platform::v0::{get_status_request, GetStatusRequest};
use dapi_grpc::tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};
use dapi_grpc::tonic::{Code, Status};
use rs_dapi_client::transport::{ProxyEndpoint, Socks5Auth, Socks5Proxy, TransportError};
use rs_dapi_client::{
    Address, AddressList, CanRetry, DapiClient, DapiClientError, DapiRequestExecutor,
    ExecutionError, RequestSettings,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tonic::service::Routes;
use tonic::transport::server::TcpIncoming;

const CA: &str = include_str!("data/proxy/ca.pem");
const SERVER_CERT: &str = include_str!("data/proxy/server.pem");
const SERVER_KEY: &str = include_str!("data/proxy/server.key");

/// SOCKS5 reply codes (RFC 1928 §6).
const SUCCEEDED: u8 = 0x00;
const GENERAL_FAILURE: u8 = 0x01;
const NOT_ALLOWED_BY_RULESET: u8 = 0x02;
const CONNECTION_REFUSED: u8 = 0x05;

/// What the proxy saw for one connection.
#[derive(Debug, Clone)]
struct Connect {
    /// Authentication methods the client offered.
    methods: Vec<u8>,
    /// RFC 1929 username, if the proxy chose username/password.
    username: Option<String>,
    /// Address type of the CONNECT target.
    atyp: u8,
    target: String,
}

type Log = Arc<Mutex<Vec<Connect>>>;

#[derive(Clone, Copy)]
struct ProxyBehaviour {
    /// Accept "no authentication" only, like a proxy without credentials.
    no_auth_only: bool,
    /// Reply code for the CONNECT; anything but `SUCCEEDED` closes.
    reply: u8,
}

impl Default for ProxyBehaviour {
    fn default() -> Self {
        Self {
            no_auth_only: false,
            reply: SUCCEEDED,
        }
    }
}

/// Starts the TLS gRPC server ("the evonode").
async fn start_node() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind node");
    let address = listener.local_addr().expect("node address");
    let tls = ServerTlsConfig::new().identity(Identity::from_pem(SERVER_CERT, SERVER_KEY));
    let server = Server::builder()
        .tls_config(tls)
        .expect("server TLS")
        .add_routes(Routes::default());
    tokio::spawn(server.serve_with_incoming(TcpIncoming::from(listener)));
    address
}

/// Starts a SOCKS5 proxy on loopback TCP that splices every accepted
/// CONNECT to `upstream`, whatever its target.
async fn start_proxy(upstream: SocketAddr, behaviour: ProxyBehaviour) -> (SocketAddr, Log) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let address = listener.local_addr().expect("proxy address");
    let log = Log::default();
    let connections = log.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_socks(
                stream,
                upstream,
                behaviour,
                connections.clone(),
            ));
        }
    });
    (address, log)
}

async fn serve_socks<S: AsyncRead + AsyncWrite + Unpin>(
    mut client: S,
    upstream: SocketAddr,
    behaviour: ProxyBehaviour,
    log: Log,
) -> io::Result<()> {
    let mut greeting = [0u8; 2];
    client.read_exact(&mut greeting).await?;
    let mut methods = vec![0u8; greeting[1] as usize];
    client.read_exact(&mut methods).await?;

    let mut username = None;
    if methods.contains(&0x02) && !behaviour.no_auth_only {
        client.write_all(&[0x05, 0x02]).await?;
        let mut header = [0u8; 2];
        client.read_exact(&mut header).await?;
        let mut name = vec![0u8; header[1] as usize];
        client.read_exact(&mut name).await?;
        let mut password_len = [0u8; 1];
        client.read_exact(&mut password_len).await?;
        let mut password = vec![0u8; password_len[0] as usize];
        client.read_exact(&mut password).await?;
        username = Some(String::from_utf8_lossy(&name).into_owned());
        client.write_all(&[0x01, 0x00]).await?;
    } else if methods.contains(&0x00) {
        client.write_all(&[0x05, 0x00]).await?;
    } else {
        return client.write_all(&[0x05, 0xff]).await;
    }

    let mut request = [0u8; 4];
    client.read_exact(&mut request).await?;
    let atyp = request[3];
    let host = match atyp {
        0x01 => {
            let mut ip = [0u8; 4];
            client.read_exact(&mut ip).await?;
            Ipv4Addr::from(ip).to_string()
        }
        0x03 => {
            let mut len = [0u8; 1];
            client.read_exact(&mut len).await?;
            let mut name = vec![0u8; len[0] as usize];
            client.read_exact(&mut name).await?;
            String::from_utf8_lossy(&name).into_owned()
        }
        0x04 => {
            let mut ip = [0u8; 16];
            client.read_exact(&mut ip).await?;
            format!("[{}]", Ipv6Addr::from(ip))
        }
        other => return Err(io::Error::other(format!("address type {other}"))),
    };
    let port = client.read_u16().await?;
    log.lock().expect("log").push(Connect {
        methods,
        username,
        atyp,
        target: format!("{host}:{port}"),
    });

    client
        .write_all(&[0x05, behaviour.reply, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await?;
    if behaviour.reply == SUCCEEDED {
        let mut node = TcpStream::connect(upstream).await?;
        tokio::io::copy_bidirectional(&mut client, &mut node).await?;
    }
    Ok(())
}

fn tcp_proxy(address: SocketAddr, auth: Socks5Auth) -> Socks5Proxy {
    Socks5Proxy {
        endpoint: ProxyEndpoint::Tcp(address),
        auth,
    }
}

fn get_status_request() -> GetStatusRequest {
    GetStatusRequest {
        version: Some(get_status_request::Version::V0(
            get_status_request::GetStatusRequestV0 {},
        )),
    }
}

/// A client over `uris` (comma-separated) through `proxy`, trusting the
/// test CA when `trust_ca`.
fn client(uris: &str, proxy: Socks5Proxy, retries: usize, trust_ca: bool) -> DapiClient {
    let addresses: AddressList = uris.parse().expect("address list");
    let settings = RequestSettings {
        connect_timeout: Some(Duration::from_secs(5)),
        timeout: Some(Duration::from_secs(5)),
        retries: Some(retries),
        ..RequestSettings::default()
    };
    let client = DapiClient::new(addresses, settings).with_proxy(proxy);
    if trust_ca {
        client.with_ca_certificate(Certificate::from_pem(CA))
    } else {
        client
    }
}

/// A client over one `uri` through `proxy` that waits `connect_timeout`
/// for a connection and never retries.
fn client_with_connect_timeout(
    uri: &str,
    proxy: Socks5Proxy,
    connect_timeout: Duration,
) -> DapiClient {
    let settings = RequestSettings {
        connect_timeout: Some(connect_timeout),
        timeout: Some(Duration::from_secs(5)),
        retries: Some(0),
        ..RequestSettings::default()
    };
    DapiClient::new(uri.parse().expect("address list"), settings).with_proxy(proxy)
}

/// Sends one GetStatus; the node serves nothing, so the request always
/// fails and the failure is returned.
async fn get_status(client: &DapiClient) -> ExecutionError<DapiClientError> {
    client
        .execute(get_status_request(), RequestSettings::default())
        .await
        .expect_err("the test node serves no methods")
}

fn transport_error(error: &ExecutionError<DapiClientError>) -> &TransportError {
    match &error.inner {
        DapiClientError::Transport(transport) => transport,
        other => panic!("expected a transport error, got {other:?}"),
    }
}

fn status(error: &ExecutionError<DapiClientError>) -> &Status {
    let TransportError::Grpc(status) = transport_error(error);
    status
}

fn connects(log: &Log) -> Vec<Connect> {
    log.lock().expect("log").clone()
}

fn is_banned(client: &DapiClient, uri: &str) -> bool {
    let address: Address = uri.parse().expect("address");
    client.address_list().is_banned(&address)
}

#[tokio::test]
async fn should_send_the_request_through_the_proxy_over_tls() {
    let node = start_node().await;
    let (proxy, log) = start_proxy(node, ProxyBehaviour::default()).await;
    let client = client(
        &format!("https://{node}"),
        tcp_proxy(proxy, Socks5Auth::None),
        0,
        true,
    );

    let error = get_status(&client).await;

    assert_eq!(status(&error).code(), Code::Unimplemented, "{error:?}");
    let seen = connects(&log);
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert_eq!(seen[0].methods, vec![0x00]);
    assert_eq!(seen[0].atyp, 0x01);
    assert_eq!(seen[0].target, node.to_string());
}

#[tokio::test]
async fn should_validate_the_node_certificate_through_the_proxy() {
    let node = start_node().await;
    let (proxy, _) = start_proxy(node, ProxyBehaviour::default()).await;
    let client = client(
        &format!("https://{node}"),
        tcp_proxy(proxy, Socks5Auth::None),
        0,
        false,
    );

    let error = get_status(&client).await;

    let status = status(&error);
    assert_eq!(status.code(), Code::Unavailable, "{status:?}");
    assert!(
        status.message().contains("invalid peer certificate"),
        "{status:?}"
    );
    assert!(!transport_error(&error).is_proxy_failure());
}

#[tokio::test]
async fn should_isolate_every_connection_with_fresh_credentials() {
    let node = start_node().await;
    let (proxy, log) = start_proxy(node, ProxyBehaviour::default()).await;
    let port = node.port();
    // Two endpoints, two connections: the IPv4 and the IPv6 loopback.
    for uri in [
        format!("https://127.0.0.1:{port}"),
        format!("https://[::1]:{port}"),
    ] {
        let client = client(
            &uri,
            tcp_proxy(proxy, Socks5Auth::RandomPerConnection),
            0,
            true,
        );
        let error = get_status(&client).await;
        assert_eq!(status(&error).code(), Code::Unimplemented, "{error:?}");
    }

    let seen = connects(&log);
    assert_eq!(seen.len(), 2, "{seen:?}");
    let usernames: Vec<&str> = seen
        .iter()
        .map(|connect| connect.username.as_deref().expect("credentials"))
        .collect();
    for (connect, username) in seen.iter().zip(&usernames) {
        assert_eq!(connect.methods, vec![0x00, 0x02]);
        assert_eq!(username.len(), 32);
        assert!(username.chars().all(|c| c.is_ascii_hexdigit()));
    }
    assert_ne!(usernames[0], usernames[1]);
}

#[tokio::test]
async fn should_accept_a_proxy_that_only_allows_no_authentication() {
    let node = start_node().await;
    let behaviour = ProxyBehaviour {
        no_auth_only: true,
        ..ProxyBehaviour::default()
    };
    let (proxy, log) = start_proxy(node, behaviour).await;
    let client = client(
        &format!("https://{node}"),
        tcp_proxy(proxy, Socks5Auth::RandomPerConnection),
        0,
        true,
    );

    let error = get_status(&client).await;

    assert_eq!(status(&error).code(), Code::Unimplemented, "{error:?}");
    let seen = connects(&log);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].username, None);
}

#[tokio::test]
async fn should_send_an_ipv6_literal_as_an_ipv6_address() {
    let node = start_node().await;
    let (proxy, log) = start_proxy(node, ProxyBehaviour::default()).await;
    let port = node.port();
    let client = client(
        &format!("https://[::1]:{port}"),
        tcp_proxy(proxy, Socks5Auth::None),
        0,
        true,
    );

    let error = get_status(&client).await;

    assert_eq!(status(&error).code(), Code::Unimplemented, "{error:?}");
    let seen = connects(&log);
    assert_eq!(seen[0].atyp, 0x04);
    assert_eq!(seen[0].target, format!("[::1]:{port}"));
}

#[tokio::test]
async fn should_leave_name_resolution_to_the_proxy() {
    let node = start_node().await;
    let (proxy, log) = start_proxy(node, ProxyBehaviour::default()).await;
    let port = node.port();
    let client = client(
        &format!("https://localhost:{port}"),
        tcp_proxy(proxy, Socks5Auth::None),
        0,
        true,
    );

    let error = get_status(&client).await;

    assert_eq!(status(&error).code(), Code::Unimplemented, "{error:?}");
    let seen = connects(&log);
    assert_eq!(seen[0].atyp, 0x03);
    assert_eq!(seen[0].target, format!("localhost:{port}"));
}

#[tokio::test]
async fn should_neither_ban_nor_retry_when_the_proxy_fails() {
    // A proxy that hangs up on every connection before answering.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let closed = listener.local_addr().expect("address");
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            drop(stream);
        }
    });
    let uris = ["https://127.0.0.1:1", "https://127.0.0.1:2"];
    let client = client(
        &uris.join(","),
        tcp_proxy(closed, Socks5Auth::None),
        2,
        true,
    );

    let error = get_status(&client).await;

    let transport = transport_error(&error);
    assert!(transport.is_proxy_failure(), "{error:?}");
    assert_eq!(status(&error).code(), Code::Unavailable);
    assert!(!error.can_retry());
    assert_eq!(error.retries, 0, "no other node is tried");
    for uri in uris {
        assert!(!is_banned(&client, uri), "{uri} must not be banned");
    }
    assert!(
        transport.clone().is_proxy_failure(),
        "a clone keeps the source"
    );
}

#[tokio::test]
async fn should_blame_the_proxy_for_its_own_failure_and_the_node_for_a_destination_reply() {
    let node = start_node().await;
    let uri = format!("https://{node}");
    for (reply, proxy_failure) in [
        (GENERAL_FAILURE, true),
        (NOT_ALLOWED_BY_RULESET, false),
        (CONNECTION_REFUSED, false),
    ] {
        let behaviour = ProxyBehaviour {
            reply,
            ..ProxyBehaviour::default()
        };
        let (proxy, _) = start_proxy(node, behaviour).await;
        let client = client(&uri, tcp_proxy(proxy, Socks5Auth::None), 0, true);

        let error = get_status(&client).await;

        assert_eq!(status(&error).code(), Code::Unavailable, "{error:?}");
        assert_eq!(
            transport_error(&error).is_proxy_failure(),
            proxy_failure,
            "reply {reply:#04x}: {error:?}"
        );
        assert_eq!(error.can_retry(), !proxy_failure, "reply {reply:#04x}");
        assert_eq!(
            is_banned(&client, &uri),
            !proxy_failure,
            "reply {reply:#04x}"
        );
    }
}

#[tokio::test]
async fn should_blame_a_proxy_that_never_answers_the_greeting() {
    // Accepts every connection and never says anything, like a suspended
    // proxy.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let silent = listener.local_addr().expect("address");
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((stream, _)) = listener.accept().await {
            held.push(stream);
        }
    });
    let uri = "https://127.0.0.1:1";
    let client = client_with_connect_timeout(
        uri,
        tcp_proxy(silent, Socks5Auth::None),
        Duration::from_secs(30),
    );

    let started = std::time::Instant::now();
    let error = get_status(&client).await;

    assert!(started.elapsed() < Duration::from_secs(20), "{error:?}");
    assert!(transport_error(&error).is_proxy_failure(), "{error:?}");
    assert!(!is_banned(&client, uri));
}

#[tokio::test]
async fn should_hold_a_stalled_route_against_the_node() {
    // Answers the greeting, then holds the CONNECT reply as Tor does while
    // it builds a circuit: only the channel's connect timeout ends it.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let stalling = listener.local_addr().expect("address");
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut greeting = [0u8; 3];
            if stream.read_exact(&mut greeting).await.is_ok()
                && stream.write_all(&[0x05, 0x00]).await.is_ok()
            {
                held.push(stream);
            }
        }
    });
    let uri = "https://127.0.0.1:1";
    let client = client_with_connect_timeout(
        uri,
        tcp_proxy(stalling, Socks5Auth::None),
        Duration::from_secs(8),
    );

    let started = std::time::Instant::now();
    let error = get_status(&client).await;

    assert!(started.elapsed() >= Duration::from_secs(8), "{error:?}");
    assert!(!transport_error(&error).is_proxy_failure(), "{error:?}");
    assert!(is_banned(&client, uri));
}

#[cfg(unix)]
#[tokio::test]
async fn should_reach_the_proxy_over_a_unix_socket() {
    use tokio::net::UnixListener;

    let node = start_node().await;
    let path = std::env::temp_dir().join(format!("rs-dapi-client-socks-{}.sock", node.port()));
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind unix proxy");
    let log = Log::default();
    let connections = log.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_socks(
                stream,
                node,
                ProxyBehaviour::default(),
                connections.clone(),
            ));
        }
    });
    let proxy = Socks5Proxy {
        endpoint: ProxyEndpoint::Unix(path.clone()),
        auth: Socks5Auth::None,
    };
    let client = client(&format!("https://{node}"), proxy, 0, true);

    let error = get_status(&client).await;
    let _ = std::fs::remove_file(&path);

    assert_eq!(status(&error).code(), Code::Unimplemented, "{error:?}");
    assert_eq!(connects(&log).len(), 1);
}
