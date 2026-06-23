//! Rate-limit ban-duration extraction from Envoy gRPC status details.
//!
//! When Envoy returns `ResourceExhausted` it optionally attaches a
//! `google.rpc.RetryInfo` message inside the `grpc-status-details-bin` binary
//! trailer, telling the client exactly when it may retry.  This module extracts
//! that hint so the address ban can expire at the right moment.
//!
//! If the trailer is absent or carries no `RetryInfo`, a configurable fallback
//! duration is used instead.  Set `DAPI_RATE_LIMIT_BAN_MS` in the process
//! environment (e.g. via dashmate node configuration) to tune the fallback
//! without code changes.

use prost::Message;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Environment variable name for the fallback rate-limit ban duration
/// (milliseconds).
///
/// When a `ResourceExhausted` status does not carry `google.rpc.RetryInfo`,
/// the address is banned for this many milliseconds.  Absent or non-numeric
/// values fall back to [`DEFAULT_RATE_LIMIT_BAN_MS`] (60 000 ms = 60 s).
pub(crate) const RATE_LIMIT_BAN_ENV_VAR: &str = "DAPI_RATE_LIMIT_BAN_MS";

const DEFAULT_RATE_LIMIT_BAN_MS: u64 = 60_000; // 60 s

/// Try to extract `google.rpc.RetryInfo.retry_delay` from the
/// `grpc-status-details-bin` binary trailer of a gRPC status.
///
/// Returns `None` when
/// * the trailer is absent or empty,
/// * the bytes are not a valid `google.rpc.Status`,
/// * no `RetryInfo` detail is present, or
/// * the embedded delay is zero.
///
/// The caller should fall back to [`fallback_rate_limit_ban_duration`] in
/// those cases.
pub(crate) fn rate_limit_ban_duration(status: &dapi_grpc::tonic::Status) -> Option<Duration> {
    let bytes = status.details();
    if bytes.is_empty() {
        return None;
    }

    let grpc_status = GrpcStatus::decode(bytes).ok()?;

    for any in &grpc_status.details {
        // Match by type URL substring; the full canonical URL is
        // "type.googleapis.com/google.rpc.RetryInfo".
        if !any.type_url.contains("RetryInfo") {
            continue;
        }
        let retry_info = RetryInfo::decode(any.value.as_slice()).ok()?;
        if let Some(d) = retry_info.retry_delay {
            let secs = d.seconds.max(0) as u64;
            let nanos = d.nanos.max(0) as u32;
            let duration = Duration::new(secs, nanos);
            if !duration.is_zero() {
                return Some(duration);
            }
        }
    }
    None
}

/// Return the fallback rate-limit ban duration.
///
/// Reads [`RATE_LIMIT_BAN_ENV_VAR`] (milliseconds).  Falls back to
/// `DEFAULT_RATE_LIMIT_BAN_MS` (60 s) when the variable is absent or
/// contains a non-numeric value.
///
/// On WASM targets environment variables are unavailable; the constant default
/// is always returned.
pub(crate) fn fallback_rate_limit_ban_duration() -> Duration {
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(v) = std::env::var(RATE_LIMIT_BAN_ENV_VAR) {
        if let Ok(ms) = v.parse::<u64>() {
            return Duration::from_millis(ms);
        }
    }
    Duration::from_millis(DEFAULT_RATE_LIMIT_BAN_MS)
}

// ---------------------------------------------------------------------------
// Minimal proto types
//
// These mirror only the fields we need from google.rpc.Status,
// google.protobuf.Any, google.rpc.RetryInfo, and google.protobuf.Duration.
// Proto3 field tags match the canonical google.rpc proto definitions.
// ---------------------------------------------------------------------------

#[derive(Clone, prost::Message)]
struct GrpcStatus {
    #[prost(int32, tag = "1")]
    code: i32,
    #[prost(string, tag = "2")]
    message: String,
    /// repeated google.protobuf.Any details = 3;
    #[prost(message, repeated, tag = "3")]
    details: Vec<ProtoAny>,
}

#[derive(Clone, prost::Message)]
struct ProtoAny {
    /// string type_url = 1;
    #[prost(string, tag = "1")]
    type_url: String,
    /// bytes value = 2;
    #[prost(bytes = "vec", tag = "2")]
    value: Vec<u8>,
}

#[derive(Clone, prost::Message)]
struct RetryInfo {
    /// google.protobuf.Duration retry_delay = 1;
    #[prost(message, optional, tag = "1")]
    retry_delay: Option<ProtoDuration>,
}

#[derive(Clone, prost::Message)]
struct ProtoDuration {
    /// int64 seconds = 1;
    #[prost(int64, tag = "1")]
    seconds: i64,
    /// int32 nanos = 2;
    #[prost(int32, tag = "2")]
    nanos: i32,
}

// ---------------------------------------------------------------------------
// Test helpers shared with other test modules
// ---------------------------------------------------------------------------

/// Build a `ResourceExhausted` tonic status carrying a `google.rpc.RetryInfo`
/// with the given delay.  Available in test builds only.
#[cfg(test)]
pub(crate) fn make_retry_info_status(seconds: i64, nanos: i32) -> dapi_grpc::tonic::Status {
    let retry_info = RetryInfo {
        retry_delay: Some(ProtoDuration { seconds, nanos }),
    };
    let mut ri_bytes = Vec::new();
    retry_info.encode(&mut ri_bytes).expect("encode RetryInfo");

    let grpc_status = GrpcStatus {
        code: 8, // RESOURCE_EXHAUSTED
        message: String::new(),
        details: vec![ProtoAny {
            type_url: "type.googleapis.com/google.rpc.RetryInfo".to_string(),
            value: ri_bytes,
        }],
    };
    let mut gs_bytes = Vec::new();
    grpc_status
        .encode(&mut gs_bytes)
        .expect("encode GrpcStatus");

    dapi_grpc::tonic::Status::with_details_and_metadata(
        dapi_grpc::tonic::Code::ResourceExhausted,
        "rate limited",
        gs_bytes.into(),
        Default::default(),
    )
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_ban_duration_extracts_retry_info() {
        let status = make_retry_info_status(5, 0);
        assert_eq!(
            rate_limit_ban_duration(&status),
            Some(Duration::from_secs(5))
        );
    }

    #[test]
    fn test_rate_limit_ban_duration_fractional_seconds() {
        // 2 seconds + 500 000 000 ns = 2.5 s
        let status = make_retry_info_status(2, 500_000_000);
        assert_eq!(
            rate_limit_ban_duration(&status),
            Some(Duration::new(2, 500_000_000))
        );
    }

    #[test]
    fn test_rate_limit_ban_duration_no_details_returns_none() {
        let status = dapi_grpc::tonic::Status::resource_exhausted("too many requests");
        assert!(rate_limit_ban_duration(&status).is_none());
    }

    #[test]
    fn test_rate_limit_ban_duration_zero_delay_returns_none() {
        // A RetryInfo with delay = 0 should be treated as absent.
        let status = make_retry_info_status(0, 0);
        assert!(rate_limit_ban_duration(&status).is_none());
    }

    #[test]
    fn test_rate_limit_ban_duration_unknown_type_url_returns_none() {
        // A detail whose type_url does not contain "RetryInfo" must be ignored.
        let other = GrpcStatus {
            code: 8,
            message: String::new(),
            details: vec![ProtoAny {
                type_url: "type.googleapis.com/google.rpc.QuotaFailure".to_string(),
                value: vec![],
            }],
        };
        let mut bytes = Vec::new();
        other.encode(&mut bytes).unwrap();
        let status = dapi_grpc::tonic::Status::with_details_and_metadata(
            dapi_grpc::tonic::Code::ResourceExhausted,
            "quota",
            bytes.into(),
            Default::default(),
        );
        assert!(rate_limit_ban_duration(&status).is_none());
    }

    #[test]
    fn test_fallback_rate_limit_ban_duration_positive() {
        // Whatever the env var is (or isn't set), the fallback is always > 0.
        assert!(fallback_rate_limit_ban_duration().as_millis() > 0);
    }
}
