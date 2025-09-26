use base64::{
    engine,
    prelude::{BASE64_STANDARD, Engine as _},
};
use dapi_grpc::platform::v0::StateTransitionBroadcastError;
use dpp::{consensus::ConsensusError, serialization::PlatformDeserializable};

use std::{collections::BTreeMap, fmt::Debug};
use tonic::{Code, metadata::MetadataValue};

#[derive(Clone)]
pub struct TenderdashStatus {
    pub code: i64,
    // human-readable error message; will be put into `data` field
    pub message: Option<String>,
    // CBOR-encoded dpp ConsensusError
    pub consensus_error: Option<Vec<u8>>,
}

impl TenderdashStatus {
    pub fn new(code: i64, message: Option<String>, consensus_error: Option<Vec<u8>>) -> Self {
        Self {
            code,
            message,
            consensus_error,
        }
    }

    pub fn to_status(&self) -> tonic::Status {
        let status_code = self.grpc_code();
        let status_message = self.grpc_message();

        let mut status: tonic::Status = tonic::Status::new(status_code, status_message);

        if let Some(consensus_error) = &self.consensus_error {
            // Add consensus error metadata
            status.metadata_mut().insert_bin(
                "dash-serialized-consensus-error-bin",
                MetadataValue::from_bytes(consensus_error),
            );
        }
        status
    }

    fn grpc_message(&self) -> String {
        if let Some(message) = &self.message {
            return message.clone();
        }

        if let Some(consensus_error_bytes) = &self.consensus_error
            && let Ok(consensus_error) =
                ConsensusError::deserialize_from_bytes(&consensus_error_bytes).inspect_err(|e| {
                    tracing::warn!("Failed to deserialize consensus error: {}", e);
                })
        {
            return consensus_error.to_string();
        }

        return format!("Unknown error with code {}", self.code);
    }

    /// map gRPC code from Tenderdash to tonic::Code.
    ///
    /// See packages/rs-dpp/src/errors/consensus/codes.rs for possible codes.
    fn grpc_code(&self) -> Code {
        match self.code {
            0 => Code::Ok,
            1 => Code::Cancelled,
            2 => Code::Unknown,
            3 => Code::InvalidArgument,
            4 => Code::DeadlineExceeded,
            5 => Code::NotFound,
            6 => Code::AlreadyExists,
            7 => Code::PermissionDenied,
            8 => Code::ResourceExhausted,
            9 => Code::FailedPrecondition,
            10 => Code::Aborted,
            11 => Code::OutOfRange,
            12 => Code::Unimplemented,
            13 => Code::Internal,
            14 => Code::Unavailable,
            15 => Code::DataLoss,
            16 => Code::Unauthenticated,
            code => {
                if (17..=9999).contains(&code) {
                    Code::Unknown
                } else if (10000..20000).contains(&code) {
                    Code::InvalidArgument
                } else if (20000..30000).contains(&code) {
                    Code::Unauthenticated
                } else if (30000..40000).contains(&code) {
                    Code::FailedPrecondition
                } else if (40000..50000).contains(&code) {
                    Code::InvalidArgument
                } else {
                    Code::Internal
                }
            }
        }
    }
}

impl From<TenderdashStatus> for StateTransitionBroadcastError {
    fn from(err: TenderdashStatus) -> Self {
        StateTransitionBroadcastError {
            code: err.code.min(u32::MAX as i64) as u32,
            message: err.message.unwrap_or_else(|| "Unknown error".to_string()),
            data: err.consensus_error.unwrap_or_default(),
        }
    }
}

impl Debug for TenderdashStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TenderdashBroadcastError")
            .field("code", &self.code)
            .field("message", &self.message)
            .field(
                "consensus_error",
                &self
                    .consensus_error
                    .as_ref()
                    .map(|e| hex::encode(e))
                    .unwrap_or_else(|| "None".to_string()),
            )
            .finish()
    }
}

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
            tracing::warn!("Failed to decode base64: {}", e);
        })
        .ok()
}

// Iteratively parses `data` as a map, checks if it contains the sequence of keys in `keys`
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

fn decode_consensus_error(info_base64: String) -> Option<Vec<u8>> {
    use ciborium::value::Value;
    let decoded_bytes = base64_decode(&info_base64)?;
    // CBOR-decode decoded_bytes
    let raw_value: Value = ciborium::de::from_reader(decoded_bytes.as_slice())
        .inspect_err(|e| {
            tracing::warn!("Failed to decode drive error info from CBOR: {}", e);
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
                        tracing::warn!("Non-u8 value in serializedError array: {}", e);
                    })
                    .ok()
            })
        })
        .collect::<Option<Vec<u8>>>()
        .or_else(|| {
            tracing::warn!("serializedError is not an array of integers");
            None
        })?;

    // sanity check: serialized error must deserialize to ConsensusError
    if ConsensusError::deserialize_from_bytes(&serialized_error).is_err() {
        tracing::warn!(
            data = hex::encode(&serialized_error),
            "Drive error info 'serializedError' failed to deserialize to ConsensusError"
        );
        return None;
    }

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
            let message = object
                .get("message")
                .and_then(|m| m.as_str())
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
            tracing::warn!("Tenderdash error is not an object: {:?}", value);
            Self {
                code: u32::MAX as i64,
                message: Some("Invalid error object from Tenderdash".to_string()),
                consensus_error: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::{ser, value::Value};

    fn setup_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_test_writer()
            .try_init();
    }
    #[test_case::test_case(
        "oWRkYXRhoW9zZXJpYWxpemVkRXJyb3KYIgMAGCwYHRgeGIoYwhh+GHwYvRhmGJ0UGNUYuhjlARjgGN0YmBhkERinGB0YPRh5GDIMGBkWGLcYfhMYzg=="; "info_fixture_1"
    )]
    fn test_info_fixture(info_base64: &str) {
        setup_tracing();
        let decoded = decode_consensus_error(info_base64.to_string()).unwrap();
        ConsensusError::deserialize_from_bytes(&decoded).expect("should deserialize");
    }
}
