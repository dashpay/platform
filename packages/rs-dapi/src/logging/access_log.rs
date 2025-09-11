//! Access log entry structures and formatting
//!
//! Supports Apache Combined Log Format for compatibility with standard log analyzers.

use chrono::{DateTime, Utc};
use std::net::IpAddr;

/// An access log entry containing request/response information
#[derive(Debug, Clone)]
pub struct AccessLogEntry {
    /// Client IP address
    pub remote_addr: Option<IpAddr>,
    /// Remote user (usually "-" for API servers)
    pub remote_user: Option<String>,
    /// Request timestamp
    pub timestamp: DateTime<Utc>,
    /// HTTP method
    pub method: String,
    /// Request path/URI
    pub uri: String,
    /// HTTP version (e.g., "HTTP/1.1")
    pub http_version: String,
    /// HTTP status code
    pub status: u16,
    /// Response body size in bytes
    pub body_bytes: u64,
    /// Referer header value
    pub referer: Option<String>,
    /// User-Agent header value
    pub user_agent: Option<String>,
    /// Request processing time in microseconds
    pub duration_us: u64,
    /// Protocol type (HTTP, gRPC, WebSocket)
    pub protocol: String,
    /// gRPC service and method (for gRPC requests)
    pub grpc_service: Option<String>,
    pub grpc_method: Option<String>,
    /// gRPC status code (for gRPC requests)
    pub grpc_status: Option<u32>,
}

impl AccessLogEntry {
    /// Create a new access log entry for HTTP requests
    pub fn new_http(
        remote_addr: Option<IpAddr>,
        method: String,
        uri: String,
        http_version: String,
        status: u16,
        body_bytes: u64,
        duration_us: u64,
    ) -> Self {
        Self {
            remote_addr,
            remote_user: None,
            timestamp: Utc::now(),
            method,
            uri,
            http_version,
            status,
            body_bytes,
            referer: None,
            user_agent: None,
            duration_us,
            protocol: "HTTP".to_string(),
            grpc_service: None,
            grpc_method: None,
            grpc_status: None,
        }
    }

    /// Create a new access log entry for gRPC requests
    pub fn new_grpc(
        remote_addr: Option<IpAddr>,
        service: String,
        method: String,
        grpc_status: u32,
        body_bytes: u64,
        duration_us: u64,
    ) -> Self {
        Self {
            remote_addr,
            remote_user: None,
            timestamp: Utc::now(),
            method: "POST".to_string(), // gRPC always uses POST
            uri: format!("/{}/{}", service, method),
            http_version: "HTTP/2.0".to_string(), // gRPC uses HTTP/2
            status: grpc_status_to_http_status(grpc_status),
            body_bytes,
            referer: None,
            user_agent: None,
            duration_us,
            protocol: "gRPC".to_string(),
            grpc_service: Some(service),
            grpc_method: Some(method),
            grpc_status: Some(grpc_status),
        }
    }

    /// Set user agent from request headers
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Set referer from request headers
    pub fn with_referer(mut self, referer: String) -> Self {
        self.referer = Some(referer);
        self
    }

    /// Format as Apache Combined Log Format
    /// Format: remote_addr - remote_user [timestamp] "method uri version" status size "referer" "user_agent" duration_us protocol
    pub fn to_combined_format(&self) -> String {
        let remote_addr = self
            .remote_addr
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "-".to_string());

        let remote_user = self.remote_user.as_deref().unwrap_or("-");

        let timestamp = self.timestamp.format("%d/%b/%Y:%H:%M:%S %z");

        let referer = self.referer.as_deref().unwrap_or("-");

        let user_agent = self.user_agent.as_deref().unwrap_or("-");

        // Extended format with additional fields
        format!(
            r#"{} - {} [{}] "{} {} {}" {} {} "{}" "{}" {}us {}"#,
            remote_addr,
            remote_user,
            timestamp,
            self.method,
            self.uri,
            self.http_version,
            self.status,
            self.body_bytes,
            referer,
            user_agent,
            self.duration_us,
            self.protocol
        )
    }
}

/// Convert gRPC status code to HTTP status code for logging
fn grpc_status_to_http_status(grpc_status: u32) -> u16 {
    match grpc_status {
        0 => 200,  // OK
        1 => 499,  // CANCELLED -> Client Closed Request
        2 => 500,  // UNKNOWN -> Internal Server Error
        3 => 400,  // INVALID_ARGUMENT -> Bad Request
        4 => 504,  // DEADLINE_EXCEEDED -> Gateway Timeout
        5 => 404,  // NOT_FOUND -> Not Found
        6 => 409,  // ALREADY_EXISTS -> Conflict
        7 => 403,  // PERMISSION_DENIED -> Forbidden
        8 => 429,  // RESOURCE_EXHAUSTED -> Too Many Requests
        9 => 412,  // FAILED_PRECONDITION -> Precondition Failed
        10 => 409, // ABORTED -> Conflict
        11 => 400, // OUT_OF_RANGE -> Bad Request
        12 => 501, // UNIMPLEMENTED -> Not Implemented
        13 => 500, // INTERNAL -> Internal Server Error
        14 => 503, // UNAVAILABLE -> Service Unavailable
        15 => 500, // DATA_LOSS -> Internal Server Error
        16 => 401, // UNAUTHENTICATED -> Unauthorized
        _ => 500,  // Unknown -> Internal Server Error
    }
}

/// Logger for access log entries
#[derive(Debug, Clone)]
pub struct AccessLogger {
    writer: std::sync::Arc<tokio::sync::Mutex<Option<tokio::fs::File>>>,
}

impl AccessLogger {
    /// Create a new access logger with specified file path
    pub async fn new(file_path: String) -> Result<Self, std::io::Error> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        Ok(Self {
            writer: std::sync::Arc::new(tokio::sync::Mutex::new(Some(file))),
        })
    }

    /// Log an access log entry
    pub async fn log(&self, entry: &AccessLogEntry) {
        let log_line = entry.to_combined_format() + "\n";

        let mut writer_guard = self.writer.lock().await;
        if let Some(ref mut file) = *writer_guard {
            use tokio::io::AsyncWriteExt;
            if let Err(e) = file.write_all(log_line.as_bytes()).await {
                tracing::warn!("Failed to write access log: {}", e);
            }
            if let Err(e) = file.flush().await {
                tracing::warn!("Failed to flush access log: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_http_access_log_format() {
        let entry = AccessLogEntry::new_http(
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))),
            "GET".to_string(),
            "/v1/platform/status".to_string(),
            "HTTP/1.1".to_string(),
            200,
            1024,
            5000,
        )
        .with_user_agent("Mozilla/5.0".to_string());

        let log_line = entry.to_combined_format();

        assert!(log_line.contains("192.168.1.100"));
        assert!(log_line.contains("GET /v1/platform/status HTTP/1.1"));
        assert!(log_line.contains("200 1024"));
        assert!(log_line.contains("Mozilla/5.0"));
        assert!(log_line.contains("5000us"));
        assert!(log_line.contains("HTTP"));
    }

    #[test]
    fn test_grpc_access_log_format() {
        let entry = AccessLogEntry::new_grpc(
            Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            "org.dash.platform.dapi.v0.Platform".to_string(),
            "getStatus".to_string(),
            0, // OK
            2048,
            10000,
        );

        let log_line = entry.to_combined_format();

        assert!(log_line.contains("127.0.0.1"));
        assert!(log_line.contains("POST /org.dash.platform.dapi.v0.Platform/getStatus HTTP/2.0"));
        assert!(log_line.contains("200 2048"));
        assert!(log_line.contains("10000us"));
        assert!(log_line.contains("gRPC"));
    }

    #[test]
    fn test_grpc_status_conversion() {
        assert_eq!(grpc_status_to_http_status(0), 200); // OK
        assert_eq!(grpc_status_to_http_status(3), 400); // INVALID_ARGUMENT
        assert_eq!(grpc_status_to_http_status(5), 404); // NOT_FOUND
        assert_eq!(grpc_status_to_http_status(13), 500); // INTERNAL
        assert_eq!(grpc_status_to_http_status(16), 401); // UNAUTHENTICATED
    }
}
