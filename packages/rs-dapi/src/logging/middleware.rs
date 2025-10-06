//! Middleware for access logging across different protocols
//!
//! Provides Tower layers for HTTP and gRPC access logging with
//! structured logging.

use crate::logging::access_log::{AccessLogEntry, AccessLogger};
use axum::extract::ConnectInfo;
use axum::http::{Request, Response, Version};
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;
use tonic::Status as TonicStatus;
use tonic::transport::server::TcpConnectInfo;
use tower::{Layer, Service};
use tracing::{Instrument, debug, error, info_span};

/// Tower layer for access logging
#[derive(Clone)]
pub struct AccessLogLayer {
    access_logger: AccessLogger,
}

impl AccessLogLayer {
    /// Wrap the provided access logger in a Tower layer for HTTP/gRPC services.
    pub fn new(access_logger: AccessLogger) -> Self {
        Self { access_logger }
    }
}

impl<S> Layer<S> for AccessLogLayer {
    type Service = AccessLogService<S>;

    fn layer(&self, service: S) -> Self::Service {
        /// Wrap the inner service with an access logging capability.
        AccessLogService {
            inner: service,
            access_logger: self.access_logger.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AccessLogService<S> {
    inner: S,
    access_logger: AccessLogger,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for AccessLogService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Delegate readiness checks to the inner service.
        self.inner.poll_ready(cx)
    }

    /// Capture request metadata, invoke the inner service, and emit access logs.
    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let start_time = Instant::now();
        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let version = format!("{:?}", req.version());

        // Detect protocol type
        let protocol_type = detect_protocol_type(&req);

        // Extract client IP
        let remote_addr = extract_remote_ip(&req);

        // Extract user agent
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Extract referer
        let referer = req
            .headers()
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let mut inner = self.inner.clone();
        let access_logger = self.access_logger.clone();

        Box::pin(async move {
            // Create span for structured logging with protocol info
            let span = info_span!(
                "request",
                method = %method,
                uri = %uri,
                protocol = %protocol_type,
                remote_addr = ?remote_addr
            );

            let result = inner.call(req).instrument(span).await;

            match result {
                Ok(response) => {
                    let duration = start_time.elapsed();
                    let status = response.status().as_u16();
                    let grpc_status_code = extract_grpc_status(&response, status);

                    // TODO: Get actual response body size
                    // This would require buffering the response which adds complexity
                    let body_bytes = 0;

                    // Create appropriate access log entry based on protocol
                    let entry = match protocol_type.as_str() {
                        "gRPC" => {
                            let (service, method_name) = parse_grpc_path(&uri);
                            AccessLogEntry::new_grpc(
                                remote_addr,
                                service,
                                method_name,
                                grpc_status_code,
                                body_bytes,
                                duration.as_micros() as u64,
                            )
                        }
                        _ => {
                            // HTTP / JSON-RPC
                            let mut entry = AccessLogEntry::new_http(
                                remote_addr,
                                method.clone(),
                                uri.clone(),
                                version,
                                status,
                                body_bytes,
                                duration.as_micros() as u64,
                            );

                            if let Some(ua) = user_agent {
                                entry = entry.with_user_agent(ua);
                            }

                            if let Some(ref_) = referer {
                                entry = entry.with_referer(ref_);
                            }

                            entry
                        }
                    };

                    access_logger.log(&entry).await;

                    // Log to structured logging
                    debug!(
                        method = %method,
                        uri = %uri,
                        protocol = %protocol_type,
                        status = status,
                        duration_us = duration.as_micros() as u64,
                        "Request completed"
                    );

                    Ok(response)
                }
                Err(err) => {
                    let duration = start_time.elapsed();

                    error!(
                        method = %method,
                        uri = %uri,
                        protocol = %protocol_type,
                        duration_us = duration.as_micros() as u64,
                        "Request failed"
                    );

                    Err(err)
                }
            }
        })
    }
}

/// Detect protocol type from HTTP request
fn detect_protocol_type<T>(req: &Request<T>) -> String {
    // Check Content-Type header for JSON-RPC
    if let Some(content_type) = req.headers().get("content-type") {
        if let Ok(ct_str) = content_type.to_str() {
            if ct_str.contains("application/json") {
                // Could be JSON-RPC, but we need to check the path or method
                return "JSON-RPC".to_string();
            }
        }
    }

    // Check if this is a gRPC request
    // gRPC requests typically have content-type: application/grpc
    // or use HTTP/2 and have specific headers
    if let Some(content_type) = req.headers().get("content-type") {
        if let Ok(ct_str) = content_type.to_str() {
            if ct_str.starts_with("application/grpc") {
                return "gRPC".to_string();
            }
        }
    }

    // Check for gRPC-specific headers
    if req.headers().contains_key("grpc-encoding")
        || req.headers().contains_key("grpc-accept-encoding")
        || req.headers().contains_key("te")
    {
        return "gRPC".to_string();
    }

    // Check HTTP version - gRPC typically uses HTTP/2
    if req.version() == Version::HTTP_2 {
        // Could be gRPC, but let's be more specific
        let path = req.uri().path();
        if path.contains('.') && path.matches('/').count() >= 2 {
            // Looks like a gRPC service path: /package.service/method
            return "gRPC".to_string();
        }
    }

    // Default to plain HTTP
    "HTTP".to_string()
}

/// Parse gRPC service and method from request path
/// Path format: /<package>.<service>/<method>
fn parse_grpc_path(path: &str) -> (String, String) {
    if let Some(path) = path.strip_prefix('/') {
        if let Some(slash_pos) = path.rfind('/') {
            let service_path = &path[..slash_pos];
            let method = &path[slash_pos + 1..];
            return (service_path.to_string(), method.to_string());
        }
    }

    // Fallback for unparseable paths
    (path.to_string(), "unknown".to_string())
}

/// Convert HTTP status code to gRPC status code
fn http_status_to_grpc_status(http_status: u16) -> u32 {
    match http_status {
        200 => 0,  // OK
        400 => 3,  // INVALID_ARGUMENT
        401 => 16, // UNAUTHENTICATED
        403 => 7,  // PERMISSION_DENIED
        404 => 5,  // NOT_FOUND
        409 => 6,  // ALREADY_EXISTS
        412 => 9,  // FAILED_PRECONDITION
        429 => 8,  // RESOURCE_EXHAUSTED
        499 => 1,  // CANCELLED
        500 => 13, // INTERNAL
        501 => 12, // UNIMPLEMENTED
        503 => 14, // UNAVAILABLE
        504 => 4,  // DEADLINE_EXCEEDED
        _ => 2,    // UNKNOWN
    }
}

/// Retrieve the remote IP address from Axum or Tonic connection metadata.
fn extract_remote_ip<B>(req: &Request<B>) -> Option<IpAddr> {
    if let Some(connect_info) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return Some(connect_info.ip());
    }

    if let Some(connect_info) = req.extensions().get::<TcpConnectInfo>() {
        if let Some(addr) = connect_info.remote_addr() {
            return Some(addr.ip());
        }
    }

    None
}

/// Determine the gRPC status code from response headers, extensions, or fallback mapping.
fn extract_grpc_status<B>(response: &Response<B>, http_status: u16) -> u32 {
    if let Some(value) = response.headers().get("grpc-status") {
        if let Ok(as_str) = value.to_str() {
            if let Ok(code) = as_str.parse::<u32>() {
                return code;
            }
        }
    }

    if let Some(status) = response.extensions().get::<TonicStatus>() {
        return status.code() as u32;
    }

    if http_status == 200 {
        0
    } else {
        http_status_to_grpc_status(http_status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use axum::http::{Request, Response};
    use std::net::{Ipv4Addr, SocketAddr};
    use tonic::Code;

    #[test]
    fn extract_remote_ip_from_connect_info() {
        let mut req: Request<()> = Request::default();
        let addr = SocketAddr::from((Ipv4Addr::new(10, 1, 2, 3), 8080));
        req.extensions_mut().insert(ConnectInfo(addr));

        assert_eq!(extract_remote_ip(&req), Some(addr.ip()));
    }

    #[test]
    fn extract_remote_ip_from_tcp_connect_info() {
        let mut req: Request<()> = Request::default();
        let addr = SocketAddr::from((Ipv4Addr::new(192, 168, 0, 5), 9000));
        let connect_info = TcpConnectInfo {
            local_addr: None,
            remote_addr: Some(addr),
        };
        req.extensions_mut().insert(connect_info);

        assert_eq!(extract_remote_ip(&req), Some(addr.ip()));
    }

    #[test]
    fn extract_grpc_status_reads_header() {
        let mut response: Response<()> = Response::new(());
        response
            .headers_mut()
            .insert("grpc-status", HeaderValue::from_static("7"));

        assert_eq!(extract_grpc_status(&response, 200), 7);
    }

    #[test]
    fn extract_grpc_status_reads_extension() {
        let mut response: Response<()> = Response::new(());
        response
            .extensions_mut()
            .insert(tonic::Status::new(Code::Unavailable, "server unavailable"));

        assert_eq!(
            extract_grpc_status(&response, 200),
            Code::Unavailable as u32
        );
    }

    #[test]
    fn extract_grpc_status_falls_back_to_http_status() {
        let response: Response<()> = Response::new(());
        assert_eq!(extract_grpc_status(&response, 503), 14);
    }
}
