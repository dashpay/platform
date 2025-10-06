//! Access log entry structures and formatting
//!
//! Supports Apache Combined Log Format for compatibility with standard log analyzers.

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLogFormat {
    Combined,
    Json,
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

    /// Format as JSON string suitable for structured logging pipelines
    pub fn to_json_string(&self) -> String {
        let value = self.to_json_value();
        serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string())
    }

    fn to_json_value(&self) -> Value {
        let mut map = Map::new();

        map.insert(
            "remote_addr".to_string(),
            self.remote_addr
                .map(|addr| Value::String(addr.to_string()))
                .unwrap_or(Value::Null),
        );
        map.insert(
            "remote_user".to_string(),
            self.remote_user
                .as_ref()
                .map(|user| Value::String(user.clone()))
                .unwrap_or(Value::Null),
        );
        map.insert(
            "timestamp".to_string(),
            Value::String(self.timestamp.to_rfc3339()),
        );
        map.insert("method".to_string(), Value::String(self.method.clone()));
        map.insert("uri".to_string(), Value::String(self.uri.clone()));
        map.insert(
            "http_version".to_string(),
            Value::String(self.http_version.clone()),
        );
        map.insert("status".to_string(), Value::Number(self.status.into()));
        map.insert(
            "body_bytes".to_string(),
            Value::Number(self.body_bytes.into()),
        );
        map.insert(
            "referer".to_string(),
            self.referer
                .as_ref()
                .map(|referer| Value::String(referer.clone()))
                .unwrap_or(Value::Null),
        );
        map.insert(
            "user_agent".to_string(),
            self.user_agent
                .as_ref()
                .map(|ua| Value::String(ua.clone()))
                .unwrap_or(Value::Null),
        );
        map.insert(
            "duration_us".to_string(),
            Value::Number(self.duration_us.into()),
        );
        map.insert("protocol".to_string(), Value::String(self.protocol.clone()));

        if let Some(service) = &self.grpc_service {
            map.insert("grpc_service".to_string(), Value::String(service.clone()));
        }

        if let Some(method) = &self.grpc_method {
            map.insert("grpc_method".to_string(), Value::String(method.clone()));
        }

        if let Some(status) = self.grpc_status {
            map.insert("grpc_status".to_string(), Value::Number(status.into()));
        }

        Value::Object(map)
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
    format: AccessLogFormat,
}

impl AccessLogger {
    /// Create a new access logger with specified file path
    pub async fn new(file_path: String, format: AccessLogFormat) -> Result<Self, std::io::Error> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        Ok(Self {
            writer: std::sync::Arc::new(tokio::sync::Mutex::new(Some(file))),
            format,
        })
    }

    /// Log an access log entry
    pub async fn log(&self, entry: &AccessLogEntry) {
        let log_line = match self.format {
            AccessLogFormat::Combined => entry.to_combined_format(),
            AccessLogFormat::Json => entry.to_json_string(),
        } + "\n";

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
    use serde_json::Value;
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

    #[test]
    fn test_access_log_json_format() {
        let entry = AccessLogEntry::new_http(
            Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))),
            "POST".to_string(),
            "/rpc".to_string(),
            "HTTP/1.1".to_string(),
            201,
            256,
            2500,
        )
        .with_user_agent("curl/8.0".to_string())
        .with_referer("https://example.net".to_string());

        let json_line = entry.to_json_string();
        let value: Value = serde_json::from_str(&json_line).expect("valid json");
        assert_eq!(value["method"], "POST");
        assert_eq!(value["status"], 201);
        assert_eq!(value["body_bytes"], 256);
        assert_eq!(value["duration_us"], 2500);
        assert_eq!(value["user_agent"], "curl/8.0");
        assert_eq!(value["referer"], "https://example.net");
        assert_eq!(value["protocol"], "HTTP");
        assert_eq!(value["remote_addr"], "10.0.0.1");
    }
}
