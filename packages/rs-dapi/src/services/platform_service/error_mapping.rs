use base64::prelude::{BASE64_STANDARD, Engine as _};
use ciborium::{de, ser, value::Value};
use dapi_grpc::platform::v0::StateTransitionBroadcastError;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use tonic::{
    Code, Status,
    metadata::{MetadataMap, MetadataValue},
};
use tracing::warn;

/// Map Drive/Tenderdash error codes to a gRPC status without building
/// additional metadata. The status code mapping follows Dash consensus ranges.
pub fn map_drive_code_to_status(code: i64, info: Option<&str>) -> Status {
    let decoded_info = info.and_then(decode_drive_error_info);
    let mut metadata = MetadataMap::new();

    let message = decoded_info
        .as_ref()
        .and_then(|details| details.message.clone())
        .or_else(|| info.map(|value| value.to_string()))
        .unwrap_or_else(|| format!("Drive error code: {}", code));

    if let Some(details) = decoded_info.as_ref() {
        if let Some(serialized) = details.serialized_error.as_ref() {
            let value = MetadataValue::from_bytes(serialized);
            metadata.insert_bin("dash-serialized-consensus-error-bin", value);
        }

        if let Some(data_bytes) = encode_drive_error_data(&details.data) {
            let value = MetadataValue::from_bytes(&data_bytes);
            metadata.insert_bin("drive-error-data-bin", value);
        }
    }

    let is_consensus_error = (10000..50000).contains(&code);

    if is_consensus_error
        && info.is_some()
        && metadata
            .get_bin("dash-serialized-consensus-error-bin")
            .is_none()
    {
        if let Some(info_str) = info {
            if !info_str.is_empty() {
                match BASE64_STANDARD.decode(info_str.as_bytes()) {
                    Ok(info_bytes) => {
                        let parsed: Result<Value, _> = de::from_reader(info_bytes.as_slice());
                        match parsed {
                            Ok(value) => {
                                if let Some(bytes) = extract_serialized_error_bytes(&value, false) {
                                    let metadata_value =
                                        MetadataValue::from_bytes(bytes.as_slice());
                                    metadata.insert_bin(
                                        "dash-serialized-consensus-error-bin",
                                        metadata_value,
                                    );
                                }
                            }
                            Err(_) => {
                                if !info_bytes.is_empty() {
                                    let metadata_value = MetadataValue::from_bytes(&info_bytes);
                                    metadata.insert_bin(
                                        "dash-serialized-consensus-error-bin",
                                        metadata_value,
                                    );
                                }
                            }
                        }
                    }
                    Err(error) => {
                        warn!(
                            "failed to decode consensus error info from base64: {}",
                            error
                        );
                    }
                }
            }
        }
    }

    if is_consensus_error {
        if let Ok(value) = MetadataValue::try_from(code.to_string()) {
            metadata.insert("code", value);
        }
    }

    let status_code = map_grpc_code(code).unwrap_or_else(|| fallback_status_code(code));

    if metadata.is_empty() {
        Status::new(status_code, message)
    } else {
        Status::with_metadata(status_code, message, metadata)
    }
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

    if error.data.is_empty()
        && let Some(data_str) = data
        && let Ok(data_bytes) = BASE64_STANDARD.decode(data_str)
    {
        error.data = data_bytes;
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
                        if details.serialized_error.is_none()
                            && let Some(bytes) = extract_serialized_error_bytes(&data_value, true)
                        {
                            details.serialized_error = Some(bytes);
                            continue;
                        }
                    }

                    details.data.insert(data_key_str, data_value);
                }
            }
            _ => {}
        }
    }

    Some(details)
}

fn extract_serialized_error_bytes(value: &Value, allow_direct: bool) -> Option<Vec<u8>> {
    match value {
        Value::Bytes(bytes) => allow_direct.then(|| bytes.clone()),
        Value::Text(text) => {
            if allow_direct {
                BASE64_STANDARD
                    .decode(text.as_bytes())
                    .ok()
                    .or_else(|| hex::decode(text).ok())
            } else {
                None
            }
        }
        Value::Map(entries) => {
            for (key, nested_value) in entries {
                let nested_allow = allow_direct
                    || matches!(key, Value::Text(key_str)
                    if matches!(
                        key_str.as_str(),
                        "serializedError" | "serialized_error"
                    ));

                if let Some(bytes) = extract_serialized_error_bytes(nested_value, nested_allow) {
                    return Some(bytes);
                }
            }
            None
        }
        Value::Array(values) => {
            for nested_value in values {
                if let Some(bytes) = extract_serialized_error_bytes(nested_value, allow_direct) {
                    return Some(bytes);
                }
            }
            None
        }
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

fn map_grpc_code(code: i64) -> Option<Code> {
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

fn fallback_status_code(code: i64) -> Code {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::{ser, value::Value};
    use tonic::Code;

    #[test]
    fn consensus_error_with_serialized_bytes_populates_metadata() {
        let info = encode_consensus_info(&[1_u8, 2, 3, 4]);
        let status = map_drive_code_to_status(20010, Some(&info));

        assert_eq!(status.code(), Code::Unauthenticated);

        let metadata = status.metadata();

        let consensus_metadata = metadata
            .get_bin("dash-serialized-consensus-error-bin")
            .expect("serialized consensus error metadata should be present")
            .to_bytes()
            .expect("consensus metadata must contain valid bytes");

        assert_eq!(consensus_metadata.as_ref(), &[1_u8, 2, 3, 4]);

        let code_metadata = metadata
            .get("code")
            .expect("consensus code metadata should be present");
        assert_eq!(code_metadata.to_str().unwrap(), "20010");
    }

    #[test]
    fn consensus_error_without_serialized_bytes_keeps_metadata_empty() {
        let info = "oWRkYXRhoW9zZXJpYWxpemVkRXJyb3KYMwEYOBggGN4YyxiDGM4YwxizBRj2GMgYixMYRBhwGPUYvBioBhQMGCAYeRiDGIMYaxhCGNcYthiBGKoLFBjZABj8AAMBGIgY/AADARiIGPwAAw0YQA";
        let status = map_drive_code_to_status(20000, Some(info));

        assert_eq!(status.code(), Code::Unauthenticated);

        let metadata = status.metadata();

        assert!(
            metadata
                .get_bin("dash-serialized-consensus-error-bin")
                .is_none(),
            "metadata must stay empty when no serialized error is present"
        );

        let code_metadata = metadata
            .get("code")
            .expect("consensus code metadata should be present");
        assert_eq!(code_metadata.to_str().unwrap(), "20000");
    }

    #[test]
    fn consensus_metadata_omits_non_binary_serialized_error() {
        let consensus_info = Value::Map(vec![(
            Value::Text("data".to_string()),
            Value::Map(vec![(
                Value::Text("serializedError".to_string()),
                Value::Text("ConsensusError".to_string()),
            )]),
        )]);

        let mut buffer = Vec::new();
        ser::into_writer(&consensus_info, &mut buffer).expect("consensus info encoding");
        let info = BASE64_STANDARD.encode(buffer);

        let status = map_drive_code_to_status(10010, Some(&info));

        let metadata = status.metadata();

        assert!(
            metadata
                .get_bin("dash-serialized-consensus-error-bin")
                .is_none(),
            "non-binary serialized error data must not populate consensus metadata"
        );

        assert_eq!(status.code(), Code::InvalidArgument);
    }

    fn encode_consensus_info(serialized_error: &[u8]) -> String {
        let info_value = Value::Map(vec![(
            Value::Text("data".to_string()),
            Value::Map(vec![(
                Value::Text("serializedError".to_string()),
                Value::Bytes(serialized_error.to_vec()),
            )]),
        )]);

        let mut buffer = Vec::new();
        ser::into_writer(&info_value, &mut buffer).expect("consensus info encoding");
        BASE64_STANDARD.encode(buffer)
    }
}
