use base64::prelude::{BASE64_STANDARD, Engine as _};
use ciborium::{de, ser, value::Value};
use dapi_grpc::platform::v0::StateTransitionBroadcastError;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use tonic::{Code, Status, metadata::MetadataMap, metadata::MetadataValue};

/// Map Drive/Tenderdash error codes to gRPC Status consistently
pub fn map_drive_code_to_status(code: u32, info: Option<String>) -> Status {
    let info_clone = info.clone();
    let decoded_info = info
        .as_deref()
        .and_then(|value| decode_drive_error_info(value));

    let message = decoded_info
        .as_ref()
        .and_then(|details| details.message.clone())
        .or(info_clone)
        .unwrap_or_else(|| format!("Drive error code: {}", code));

    let mut metadata = MetadataMap::new();

    if let Some(details) = decoded_info.as_ref() {
        if let Some(data_bytes) = encode_drive_error_data(&details.data) {
            metadata.insert_bin(
                "drive-error-data-bin",
                MetadataValue::from_bytes(&data_bytes),
            );
        }

        if let Some(serialized) = details.serialized_error.as_ref() {
            metadata.insert_bin(
                "dash-serialized-consensus-error-bin",
                MetadataValue::from_bytes(serialized),
            );
        }

        if (10000..50000).contains(&code) {
            if let Ok(value) = MetadataValue::try_from(code.to_string()) {
                metadata.insert("code", value);
            }
        }
    }

    if let Some(grpc_code) = map_grpc_code(code) {
        return status_with_metadata(grpc_code, message, metadata);
    }

    if (17..=9999).contains(&code) {
        return status_with_metadata(Code::Unknown, message, metadata);
    }

    if (10000..20000).contains(&code) {
        return status_with_metadata(Code::InvalidArgument, message, metadata);
    }

    if (20000..30000).contains(&code) {
        return status_with_metadata(Code::Unauthenticated, message, metadata);
    }

    if (30000..40000).contains(&code) {
        return status_with_metadata(Code::FailedPrecondition, message, metadata);
    }

    if (40000..50000).contains(&code) {
        return status_with_metadata(Code::InvalidArgument, message, metadata);
    }

    Status::internal(format!("Unknown Drive error code: {}", code))
}

/// Build StateTransitionBroadcastError consistently from code/info/data
pub fn build_state_transition_error(
    code: u32,
    info: &str,
    data: Option<&str>,
) -> StateTransitionBroadcastError {
    let decoded_info = decode_drive_error_info(info);

    let mut error = StateTransitionBroadcastError {
        code,
        message: decoded_info
            .as_ref()
            .and_then(|details| details.message.clone())
            .unwrap_or_else(|| info.to_string()),
        data: Vec::new(),
    };

    if let Some(details) = decoded_info {
        if let Some(serialized) = details.serialized_error {
            error.data = serialized;
        } else if let Some(data_bytes) = encode_drive_error_data(&details.data) {
            error.data = data_bytes;
        }
    }

    if error.data.is_empty() {
        if let Some(data_str) = data {
            if let Ok(data_bytes) = BASE64_STANDARD.decode(data_str) {
                error.data = data_bytes;
            }
        }
    }

    error
}

#[derive(Debug, Default, Clone)]
struct DriveErrorInfo {
    message: Option<String>,
    data: BTreeMap<String, Value>,
    serialized_error: Option<Vec<u8>>,
}

fn decode_drive_error_info(info: &str) -> Option<DriveErrorInfo> {
    let decoded_bytes = BASE64_STANDARD.decode(info).ok()?;
    let raw_value: Value = de::from_reader(decoded_bytes.as_slice()).ok()?;

    let Value::Map(entries) = raw_value else {
        return None;
    };

    let mut details = DriveErrorInfo::default();

    for (key, value) in entries {
        match (key, value) {
            (Value::Text(key), Value::Text(text)) if key == "message" => {
                details.message = Some(text);
            }
            (Value::Text(key), Value::Bytes(bytes)) if key == "message" => {
                if let Ok(text) = String::from_utf8(bytes) {
                    details.message = Some(text);
                }
            }
            (Value::Text(key), Value::Map(data_entries)) if key == "data" => {
                for (data_key, data_value) in data_entries {
                    let Value::Text(data_key_str) = data_key else {
                        tracing::debug!(
                            ?data_key,
                            "Skipping non-string data key in Drive error info"
                        );
                        continue;
                    };

                    if matches!(
                        data_key_str.as_str(),
                        "serializedError" | "serialized_error"
                    ) {
                        if details.serialized_error.is_none() {
                            if let Some(bytes) = extract_serialized_error_bytes(data_value) {
                                details.serialized_error = Some(bytes);
                            }
                        }
                    } else {
                        details.data.insert(data_key_str, data_value);
                    }
                }
            }
            _ => {}
        }
    }

    Some(details)
}

fn extract_serialized_error_bytes(value: Value) -> Option<Vec<u8>> {
    match value {
        Value::Bytes(bytes) => Some(bytes),
        Value::Text(text) => BASE64_STANDARD
            .decode(text.as_bytes())
            .ok()
            .or_else(|| hex::decode(&text).ok()),
        Value::Map(entries) => {
            for (key, nested_value) in entries {
                if let Value::Text(key_str) = key {
                    if matches!(key_str.as_str(), "serializedError" | "serialized_error") {
                        return extract_serialized_error_bytes(nested_value);
                    }
                }
            }
            None
        }
        Value::Array(values) => values
            .into_iter()
            .filter_map(extract_serialized_error_bytes)
            .next(),
        _ => None,
    }
}

fn encode_drive_error_data(data: &BTreeMap<String, Value>) -> Option<Vec<u8>> {
    if data.is_empty() {
        return None;
    }

    let map_entries: Vec<(Value, Value)> = data
        .iter()
        .map(|(key, value)| (Value::Text(key.clone()), value.clone()))
        .collect();

    let mut buffer = Vec::new();
    if ser::into_writer(&Value::Map(map_entries), &mut buffer).is_ok() {
        Some(buffer)
    } else {
        None
    }
}

fn map_grpc_code(code: u32) -> Option<Code> {
    match code {
        0 => Some(Code::Ok),
        1 => Some(Code::Cancelled),
        2 => Some(Code::Unknown),
        3 => Some(Code::InvalidArgument),
        4 => Some(Code::DeadlineExceeded),
        5 => Some(Code::NotFound),
        6 => Some(Code::AlreadyExists),
        7 => Some(Code::PermissionDenied),
        8 => Some(Code::ResourceExhausted),
        9 => Some(Code::FailedPrecondition),
        10 => Some(Code::Aborted),
        11 => Some(Code::OutOfRange),
        12 => Some(Code::Unimplemented),
        13 => Some(Code::Internal),
        14 => Some(Code::Unavailable),
        15 => Some(Code::DataLoss),
        16 => Some(Code::Unauthenticated),
        _ => None,
    }
}

fn status_with_metadata(code: Code, message: String, metadata: MetadataMap) -> Status {
    if metadata.is_empty() {
        Status::new(code, message)
    } else {
        Status::with_metadata(code, message, metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attaches_serialized_consensus_error_metadata() {
        use base64::engine::general_purpose::STANDARD as BASE64;

        // Build CBOR blob matching Drive's response_info_for_version implementation
        let mut buffer = Vec::new();
        let serialized_error_bytes = vec![0x01, 0x02, 0x03];
        let value = Value::Map(vec![(
            Value::Text("data".to_string()),
            Value::Map(vec![(
                Value::Text("serializedError".to_string()),
                Value::Bytes(serialized_error_bytes.clone()),
            )]),
        )]);

        ser::into_writer(&value, &mut buffer).expect("serialize cbor");
        let encoded_info = BASE64.encode(buffer);

        let status = map_drive_code_to_status(10246, Some(encoded_info));

        assert_eq!(status.code(), Code::InvalidArgument);

        let metadata = status.metadata();
        let consensus_error = metadata
            .get_bin("dash-serialized-consensus-error-bin")
            .expect("consensus error metadata");

        let consensus_error_bytes = consensus_error
            .to_bytes()
            .expect("decode consensus error metadata");

        assert_eq!(consensus_error_bytes.as_ref(), serialized_error_bytes.as_slice());

        let code_value = metadata.get("code").expect("code metadata");
        assert_eq!(code_value, "10246");
    }

    #[test]
    fn handles_snake_case_serialized_error_key() {
        use base64::engine::general_purpose::STANDARD as BASE64;

        let mut buffer = Vec::new();
        let serialized_error_bytes = vec![0x0A, 0x0B, 0x0C];
        let serialized_error_base64 = BASE64.encode(&serialized_error_bytes);

        let value = Value::Map(vec![
            (
                Value::Text("message".to_string()),
                Value::Text("some consensus violation".to_string()),
            ),
            (
                Value::Text("data".to_string()),
                Value::Map(vec![
                    (
                        Value::Text("serialized_error".to_string()),
                        Value::Text(serialized_error_base64),
                    ),
                ]),
            ),
        ]);

        ser::into_writer(&value, &mut buffer).expect("serialize cbor");
        let encoded_info = BASE64.encode(buffer);

        let status = map_drive_code_to_status(10212, Some(encoded_info));
        assert_eq!(status.code(), Code::InvalidArgument);

        let metadata = status.metadata();
        let consensus_error = metadata
            .get_bin("dash-serialized-consensus-error-bin")
            .expect("consensus error metadata for snake case key");

        let consensus_error_bytes = consensus_error
            .to_bytes()
            .expect("decode consensus error metadata for snake case key");
        assert_eq!(consensus_error_bytes.as_ref(), serialized_error_bytes.as_slice());

        let code_value = metadata.get("code").expect("code metadata");
        assert_eq!(code_value, "10212");
    }
}
