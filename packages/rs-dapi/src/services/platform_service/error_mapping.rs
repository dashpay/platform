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

pub(crate) fn decode_drive_error_info(info: &str) -> Option<DriveErrorInfo> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::{ser, value::Value};

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
