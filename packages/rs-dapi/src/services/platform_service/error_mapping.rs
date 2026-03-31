use base64::{engine, prelude::Engine as _};
use dapi_grpc::platform::v0::{
    StateTransitionBroadcastError, WaitForStateTransitionResultResponse,
};
use dpp::{consensus::ConsensusError, serialization::PlatformDeserializable};
use std::{fmt::Debug, str::FromStr};
use tonic::{Code, metadata::MetadataValue};

use crate::DapiError;

#[derive(Clone, serde::Serialize)]
pub struct TenderdashStatus {
    pub code: i64,
    /// human-readable error message; will be put into `data` field
    /// Access using [`TenderdashStatus::grpc_message()`].
    pub message: Option<String>,
    /// CBOR-encoded dpp ConsensusError
    pub consensus_error: Option<Vec<u8>>,
}

impl TenderdashStatus {
    /// Construct a Tenderdash status wrapper, validating consensus error payloads upfront.
    pub fn new(code: i64, message: Option<String>, consensus_error: Option<Vec<u8>>) -> Self {
        // sanity check: consensus_error must deserialize to ConsensusError if present
        if let Some(ref bytes) = consensus_error
            && ConsensusError::deserialize_from_bytes(bytes).is_err()
        {
            tracing::debug!(
                data = hex::encode(bytes),
                "TenderdashStatus consensus_error failed to deserialize to ConsensusError"
            );
        }
        Self {
            code,
            message,
            consensus_error,
        }
    }

    /// Convert the Tenderdash status into a gRPC `tonic::Status` with enriched metadata.
    pub fn to_status(&self) -> tonic::Status {
        let status_code = self.grpc_code();
        let status_message = self.grpc_message();

        // check if we can map to a DapiError first
        if let Some(dapi_error) = map_tenderdash_message(&status_message) {
            // avoid infinite recursion
            if !matches!(dapi_error, DapiError::TenderdashClientError(_)) {
                return dapi_error.to_status();
            }
        }

        let mut status: tonic::Status = tonic::Status::new(status_code, status_message);

        self.write_grpc_metadata(status.metadata_mut());

        status
    }

    /// Populate metadata fields expected by clients consuming Drive/Tenderdash errors.
    fn write_grpc_metadata(&self, metadata: &mut tonic::metadata::MetadataMap) {
        // drive-error-data-bin contains serialized DriveErrorDataBin structure
        let mut serialized_drive_error_data = Vec::new();
        ciborium::ser::into_writer(&self, &mut serialized_drive_error_data)
            .inspect_err(|e| {
                tracing::debug!("Failed to serialize drive error data bin: {}", e);
            })
            .ok();

        metadata.insert_bin(
            "drive-error-data-bin",
            MetadataValue::from_bytes(&serialized_drive_error_data),
        );

        // expose the consensus error code directly for clients
        metadata.insert(
            "code",
            MetadataValue::from_str(&self.code.to_string())
                .unwrap_or_else(|_| MetadataValue::from_static("0")),
        );

        if let Some(consensus_error) = &self.consensus_error {
            // Add consensus error metadata
            metadata.insert_bin(
                "dash-serialized-consensus-error-bin",
                MetadataValue::from_bytes(consensus_error),
            );
        }
    }

    /// Derive an end-user message, preferring explicit message over consensus error details.
    fn grpc_message(&self) -> String {
        if let Some(message) = &self.message {
            return message.clone();
        }

        if let Some(consensus_error_bytes) = &self.consensus_error
            && let Ok(consensus_error) =
                ConsensusError::deserialize_from_bytes(consensus_error_bytes).inspect_err(|e| {
                    tracing::debug!("Failed to deserialize consensus error: {}", e);
                })
        {
            return consensus_error.to_string();
        }

        format!("Unknown error with code {}", self.code)
    }

    /// map gRPC code from Tenderdash to tonic::Code.
    ///
    /// See packages/rs-dpp/src/errors/consensus/codes.rs for possible codes.
    fn grpc_code(&self) -> Code {
        let code = Code::from_i32(self.code as i32);
        if code != Code::Unknown {
            return code;
        }

        match self.code {
            17..10000 => Code::Unknown,
            10000..20000 => Code::InvalidArgument,
            20000..30000 => Code::Unauthenticated,
            30000..40000 => Code::FailedPrecondition,
            40000..50000 => Code::InvalidArgument,
            _ => Code::Internal,
        }
    }
}

impl From<TenderdashStatus> for tonic::Response<WaitForStateTransitionResultResponse> {
    fn from(err: TenderdashStatus) -> Self {
        use dapi_grpc::platform::v0::wait_for_state_transition_result_response::*;
        let st_error = StateTransitionBroadcastError::from(err.clone());

        let message = WaitForStateTransitionResultResponse {
            version: Some(Version::V0(WaitForStateTransitionResultResponseV0 {
                metadata: None,
                result: Some(wait_for_state_transition_result_response_v0::Result::Error(
                    st_error,
                )),
            })),
        };

        let mut response = Self::new(message);

        err.write_grpc_metadata(response.metadata_mut());

        response
    }
}

impl From<TenderdashStatus> for StateTransitionBroadcastError {
    fn from(err: TenderdashStatus) -> Self {
        StateTransitionBroadcastError {
            code: err.code.clamp(0, u32::MAX as i64) as u32,
            message: err.grpc_message(),
            data: err.consensus_error.clone().unwrap_or_default(),
        }
    }
}

impl Debug for TenderdashStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TenderdashStatus")
            .field("code", &self.code)
            .field("message", &self.message)
            .field(
                "consensus_error",
                &self
                    .consensus_error
                    .as_ref()
                    .map(hex::encode)
                    .unwrap_or_else(|| "None".to_string()),
            )
            .finish()
    }
}

/// Decode a potentially unpadded base64 string used by Tenderdash error payloads.
pub(crate) fn base64_decode(input: &str) -> Option<Vec<u8>> {
    static BASE64: engine::GeneralPurpose = {
        let b64_config = engine::GeneralPurposeConfig::new()
            .with_decode_allow_trailing_bits(true)
            .with_encode_padding(false)
            .with_decode_padding_mode(engine::DecodePaddingMode::Indifferent);

        engine::GeneralPurpose::new(&base64::alphabet::STANDARD, b64_config)
    };
    BASE64
        .decode(input)
        .inspect_err(|e| {
            tracing::debug!("Failed to decode base64: {}", e);
        })
        .ok()
}

/// Walk a nested CBOR map by following the provided key path.
fn walk_cbor_for_key<'a>(data: &'a ciborium::Value, keys: &[&str]) -> Option<&'a ciborium::Value> {
    if keys.is_empty() {
        tracing::trace!(?data, "found value, returning");
        return Some(data);
    }

    let current_key = keys[0];
    let rest_keys = &keys[1..];

    let map = data.as_map().or_else(|| {
        tracing::trace!(?data, "Not a CBOR map, cannot walk for key: {:?}", keys);
        None
    })?;

    for (k, v) in map {
        if let ciborium::Value::Text(key_str) = k
            && key_str == current_key
        {
            let found = walk_cbor_for_key(v, rest_keys);
            return found;
        }
    }

    tracing::trace!(?keys, "Key not found in CBOR map: {:?}", keys);
    None
}

/// Decode Tenderdash consensus error metadata from base64 CBOR into raw consensus bytes.
pub(super) fn decode_consensus_error(info_base64: String) -> Option<Vec<u8>> {
    use ciborium::value::Value;

    tracing::trace!(?info_base64, "decode_consensus_error: received info");
    let decoded_bytes = base64_decode(&info_base64)?;
    tracing::trace!(hex = %hex::encode(&decoded_bytes), len = decoded_bytes.len(), "decode_consensus_error: base64 decoded bytes");
    // CBOR-decode decoded_bytes
    let raw_value: Value = ciborium::de::from_reader(decoded_bytes.as_slice())
        .inspect_err(|e| {
            tracing::debug!("Failed to decode drive error info from CBOR: {}", e);
        })
        .ok()?;

    tracing::trace!("Drive error info CBOR value: {:?}", raw_value);

    let serialized_error = walk_cbor_for_key(&raw_value, &["data", "serializedError"])?
        .as_array()?
        .iter()
        .map(|v| {
            v.as_integer().and_then(|n| {
                u8::try_from(n)
                    .inspect_err(|e| {
                        tracing::debug!("Non-u8 value in serializedError array: {}", e);
                    })
                    .ok()
            })
        })
        .collect::<Option<Vec<u8>>>()
        .or_else(|| {
            tracing::debug!("serializedError is not an array of integers");
            None
        })?;

    // sanity check: serialized error must deserialize to ConsensusError
    if ConsensusError::deserialize_from_bytes(&serialized_error).is_err() {
        tracing::debug!(
            data = hex::encode(&serialized_error),
            "Drive error info 'serializedError' failed to deserialize to ConsensusError"
        );
        return None;
    }

    tracing::trace!(
        serialized_error_hex = %hex::encode(&serialized_error),
        len = serialized_error.len(),
        "decode_consensus_error: extracted consensus error bytes",
    );

    Some(serialized_error)
}

impl From<serde_json::Value> for TenderdashStatus {
    // Convert from a JSON error object returned by Tenderdash RPC, typically in the `error` field of a JSON-RPC response.
    fn from(value: serde_json::Value) -> Self {
        if let Some(object) = value.as_object() {
            let code = object
                .get("code")
                .and_then(|c| c.as_i64())
                .unwrap_or_else(|| {
                    tracing::debug!("Tenderdash error missing 'code' field, defaulting to 0");
                    0
                });
            let raw_message = object
                .get("message")
                .and_then(|m| m.as_str())
                .map(|m| m.trim());
            // empty message or "Internal error" is not very informative, so we try to check `data` field
            let message = if raw_message
                .is_none_or(|m| m.is_empty() || m.eq_ignore_ascii_case("Internal error"))
            {
                object
                    .get("data")
                    .and_then(|d| d.as_str())
                    .filter(|s| s.is_ascii())
            } else {
                raw_message
            }
            .map(|s| s.to_string());

            // info contains additional error details, possibly including consensus error
            let consensus_error = object
                .get("info")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .and_then(decode_consensus_error);

            Self {
                code,
                message,
                consensus_error,
            }
        } else {
            tracing::debug!("Tenderdash error is not an object: {:?}", value);
            Self {
                code: u32::MAX as i64,
                message: Some("Invalid error object from Tenderdash".to_string()),
                consensus_error: None,
            }
        }
    }
}

// Map some common Tenderdash error messages to DapiError variants
pub(super) fn map_tenderdash_message(message: &str) -> Option<DapiError> {
    let msg = message.trim().to_lowercase();
    if msg == "tx already exists in cache" {
        return Some(DapiError::AlreadyExists(msg.to_string()));
    }

    if msg.starts_with("tx too large.") {
        let message = msg.replace("tx too large.", "").trim().to_string();
        return Some(DapiError::InvalidArgument(
            "state transition is too large. ".to_string() + &message,
        ));
    }

    if msg.starts_with("mempool is full") {
        return Some(DapiError::ResourceExhausted(msg.to_string()));
    }

    if msg.contains("context deadline exceeded") {
        return Some(DapiError::Timeout(
            "broadcasting state transition is timed out".to_string(),
        ));
    }

    if msg.contains("too_many_requests") {
        return Some(DapiError::ResourceExhausted(
            "tenderdash is not responding: too many requests".to_string(),
        ));
    }

    if msg.starts_with("broadcast confirmation not received:") {
        return Some(DapiError::Timeout(msg.to_string()));
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    use dpp::{serialization::PlatformSerializableWithPlatformVersion, version::PlatformVersion};
    use serde::Deserialize;

    fn setup_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_test_writer()
            .try_init();
    }

    #[derive(Deserialize)]
    struct DriveErrorDataBinMetadata {
        code: i64,
    }

    #[test]
    fn to_status_sets_expected_metadata() {
        setup_tracing();

        let consensus_error = ConsensusError::DefaultError;
        let original_consensus_error_bytes = consensus_error
            .serialize_to_bytes_with_platform_version(PlatformVersion::latest())
            .expect("should serialize");

        let status = TenderdashStatus::new(
            42,
            Some("metadata test".to_string()),
            Some(original_consensus_error_bytes.clone()),
        )
        .to_status();

        let metadata = status.metadata();

        let drive_error_bytes = metadata
            .get_bin("drive-error-data-bin")
            .inspect(|v| {
                tracing::debug!(?v, "drive-error-data-bin metadata");
            })
            .expect("missing drive-error-data-bin metadata")
            .to_bytes()
            .expect("drive-error-data-bin should be valid bytes");
        let drive_error: DriveErrorDataBinMetadata =
            ciborium::de::from_reader(drive_error_bytes.as_ref())
                .expect("drive-error-data-bin should deserialize");
        assert_eq!(drive_error.code, 42);

        let consensus_error_bytes = metadata
            .get_bin("dash-serialized-consensus-error-bin")
            .expect("missing consensus error metadata")
            .to_bytes()
            .expect("consensus error metadata should be valid bytes");
        assert_eq!(
            consensus_error_bytes.as_ref(),
            original_consensus_error_bytes.as_slice()
        );
    }
    #[test_case::test_case(
        "oWRkYXRhoW9zZXJpYWxpemVkRXJyb3KYIgMAGCwYHRgeGIoYwhh+GHwYvRhmGJ0UGNUYuhjlARjgGN0YmBhkERinGB0YPRh5GDIMGBkWGLcYfhMYzg=="; "info_fixture_1"
    )]
    fn test_info_fixture(info_base64: &str) {
        setup_tracing();
        let decoded = decode_consensus_error(info_base64.to_string())
            .expect("decode consensus error from fixture");
        ConsensusError::deserialize_from_bytes(&decoded).expect("should deserialize");
    }

    // -- map_tenderdash_message tests --

    #[test]
    fn map_tenderdash_message_tx_already_exists() {
        let result = map_tenderdash_message("tx already exists in cache");
        assert!(result.is_some());
        assert!(matches!(
            result.expect("should map to AlreadyExists"),
            DapiError::AlreadyExists(_)
        ));
    }

    #[test]
    fn map_tenderdash_message_tx_too_large() {
        let result = map_tenderdash_message("tx too large. Max size: 100kb");
        assert!(result.is_some());
        match result.expect("should map to InvalidArgument") {
            DapiError::InvalidArgument(msg) => {
                assert!(msg.contains("state transition is too large"));
            }
            other => panic!("expected InvalidArgument, got {:?}", other),
        }
    }

    #[test]
    fn map_tenderdash_message_mempool_full() {
        let result = map_tenderdash_message("mempool is full");
        assert!(result.is_some());
        assert!(matches!(
            result.expect("should map to ResourceExhausted"),
            DapiError::ResourceExhausted(_)
        ));
    }

    #[test]
    fn map_tenderdash_message_context_deadline() {
        let result = map_tenderdash_message("rpc error: context deadline exceeded");
        assert!(result.is_some());
        match result.expect("should map to Timeout") {
            DapiError::Timeout(msg) => {
                assert!(msg.contains("timed out"));
            }
            other => panic!("expected Timeout, got {:?}", other),
        }
    }

    #[test]
    fn map_tenderdash_message_too_many_requests() {
        let result = map_tenderdash_message("too_many_requests");
        assert!(result.is_some());
        assert!(matches!(
            result.expect("should map to ResourceExhausted"),
            DapiError::ResourceExhausted(_)
        ));
    }

    #[test]
    fn map_tenderdash_message_broadcast_confirmation() {
        let result =
            map_tenderdash_message("broadcast confirmation not received: timed out waiting");
        assert!(result.is_some());
        assert!(matches!(
            result.expect("should map to Timeout"),
            DapiError::Timeout(_)
        ));
    }

    #[test]
    fn map_tenderdash_message_unknown_returns_none() {
        assert!(map_tenderdash_message("some random error").is_none());
    }

    // -- TenderdashStatus grpc_code tests --

    #[test]
    fn grpc_code_standard_codes() {
        // OK = 0
        let status = TenderdashStatus::new(0, Some("ok".to_string()), None);
        assert_eq!(status.to_status().code(), tonic::Code::Ok);

        // InvalidArgument = 3
        let status = TenderdashStatus::new(3, Some("bad".to_string()), None);
        assert_eq!(status.to_status().code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn grpc_code_consensus_ranges() {
        // 10000..20000 => InvalidArgument
        let status = TenderdashStatus::new(10001, Some("consensus".to_string()), None);
        assert_eq!(status.to_status().code(), tonic::Code::InvalidArgument);

        // 20000..30000 => Unauthenticated
        let status = TenderdashStatus::new(25000, Some("auth".to_string()), None);
        assert_eq!(status.to_status().code(), tonic::Code::Unauthenticated);

        // 30000..40000 => FailedPrecondition
        let status = TenderdashStatus::new(35000, Some("precond".to_string()), None);
        assert_eq!(status.to_status().code(), tonic::Code::FailedPrecondition);

        // 40000..50000 => InvalidArgument
        let status = TenderdashStatus::new(45000, Some("validation".to_string()), None);
        assert_eq!(status.to_status().code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn grpc_code_out_of_range_returns_internal() {
        let status = TenderdashStatus::new(99999, Some("unknown".to_string()), None);
        assert_eq!(status.to_status().code(), tonic::Code::Internal);
    }

    // -- TenderdashStatus grpc_message tests --

    #[test]
    fn grpc_message_prefers_explicit_message() {
        let status = TenderdashStatus::new(1, Some("explicit msg".to_string()), None);
        let tonic_status = status.to_status();
        assert_eq!(tonic_status.message(), "explicit msg");
    }

    #[test]
    fn grpc_message_fallback_to_unknown_error() {
        let status = TenderdashStatus::new(42, None, None);
        let tonic_status = status.to_status();
        assert!(tonic_status.message().contains("42"));
    }

    // -- TenderdashStatus::from(Value) tests --

    #[test]
    fn from_json_value_with_code_and_message() {
        let value = serde_json::json!({"code": 10, "message": "test error"});
        let status = TenderdashStatus::from(value);
        assert_eq!(status.code, 10);
        assert_eq!(status.message.as_deref(), Some("test error"));
    }

    #[test]
    fn from_json_value_missing_code_defaults_to_zero() {
        let value = serde_json::json!({"message": "no code"});
        let status = TenderdashStatus::from(value);
        assert_eq!(status.code, 0);
    }

    #[test]
    fn from_json_value_empty_message_uses_data() {
        let value = serde_json::json!({"code": 1, "message": "", "data": "detail from data"});
        let status = TenderdashStatus::from(value);
        assert_eq!(status.message.as_deref(), Some("detail from data"));
    }

    #[test]
    fn from_json_value_internal_error_message_uses_data() {
        let value =
            serde_json::json!({"code": 1, "message": "Internal error", "data": "real detail"});
        let status = TenderdashStatus::from(value);
        assert_eq!(status.message.as_deref(), Some("real detail"));
    }

    #[test]
    fn from_json_value_non_object() {
        let value = serde_json::json!("not an object");
        let status = TenderdashStatus::from(value);
        assert_eq!(status.code, u32::MAX as i64);
        assert!(
            status
                .message
                .as_deref()
                .expect("message should be present for non-object input")
                .contains("Invalid error object")
        );
    }

    // -- TenderdashStatus Debug --

    #[test]
    fn tenderdash_status_debug_format() {
        let status = TenderdashStatus::new(42, Some("msg".to_string()), Some(vec![0xab, 0xcd]));
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("msg"));
        assert!(debug_str.contains("abcd")); // hex encoded consensus error
    }

    #[test]
    fn tenderdash_status_debug_no_consensus_error() {
        let status = TenderdashStatus::new(0, None, None);
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("None"));
    }

    // -- base64_decode tests --

    #[test]
    fn base64_decode_valid() {
        let decoded = base64_decode("aGVsbG8=");
        assert_eq!(decoded, Some(b"hello".to_vec()));
    }

    #[test]
    fn base64_decode_invalid() {
        let decoded = base64_decode("!!!not valid base64!!!");
        assert!(decoded.is_none());
    }

    #[test]
    fn base64_decode_unpadded() {
        // "hello" base64 without padding
        let decoded = base64_decode("aGVsbG8");
        assert_eq!(decoded, Some(b"hello".to_vec()));
    }

    // -- decode_consensus_error with invalid data --

    #[test]
    fn decode_consensus_error_invalid_base64() {
        assert!(decode_consensus_error("!!!".to_string()).is_none());
    }

    #[test]
    fn decode_consensus_error_not_cbor() {
        // Valid base64 but not valid CBOR
        let b64 = base64::prelude::BASE64_STANDARD.encode(b"not cbor data");
        assert!(decode_consensus_error(b64).is_none());
    }

    // -- From<TenderdashStatus> for StateTransitionBroadcastError --

    #[test]
    fn tenderdash_status_to_broadcast_error() {
        let status = TenderdashStatus::new(42, Some("broadcast err".to_string()), None);
        let broadcast_err: StateTransitionBroadcastError = status.into();
        assert_eq!(broadcast_err.code, 42);
        assert_eq!(broadcast_err.message, "broadcast err");
    }

    #[test]
    fn tenderdash_status_to_broadcast_error_clamps_negative_code() {
        let status = TenderdashStatus::new(-1, Some("neg".to_string()), None);
        let broadcast_err: StateTransitionBroadcastError = status.into();
        assert_eq!(broadcast_err.code, 0);
    }

    // -- to_status with map_tenderdash_message shortcut --

    #[test]
    fn to_status_maps_known_tenderdash_message() {
        let status = TenderdashStatus::new(0, Some("tx already exists in cache".to_string()), None);
        let tonic_status = status.to_status();
        // "tx already exists in cache" maps to AlreadyExists, which maps to already_exists
        assert_eq!(tonic_status.code(), tonic::Code::AlreadyExists);
    }
}
