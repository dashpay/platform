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
                    if let Value::Text(data_key_str) = data_key {
                        if data_key_str == "serializedError" {
                            match data_value {
                                Value::Bytes(bytes) => {
                                    details.serialized_error = Some(bytes);
                                }
                                Value::Text(text) => {
                                    if let Ok(bytes) = BASE64_STANDARD.decode(text.as_bytes()) {
                                        details.serialized_error = Some(bytes);
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            details.data.insert(data_key_str, data_value);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Some(details)
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
