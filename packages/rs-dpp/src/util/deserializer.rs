use crate::errors::ProtocolError;
use crate::identifier;
use crate::identifier::Identifier;
use crate::util::string_encoding::Encoding;
use serde_json::Value as JsonValue;
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::convert::TryInto;

const PROPERTY_CONTENT_MEDIA_TYPE: &str = "contentMediaType";

pub fn get_protocol_version(version_bytes: &[u8]) -> Result<u32, ProtocolError> {
    Ok(if version_bytes.len() != 4 {
        return Err(ProtocolError::NoProtocolVersionError);
    } else {
        let version_set_bytes: [u8; 4] = version_bytes
            .try_into()
            .map_err(|_| ProtocolError::NoProtocolVersionError)?;
        u32::from_be_bytes(version_set_bytes)
    })
}

// the parse_identifiers for entire Json Value
// this is something that we need to change

// replaces Identifiers field names of type JsonValue::Vec<u8> with JsonValue::String().
// The base58 is used for conversion
pub fn parse_identities(
    json_map: &mut Map<String, Value>,
    field_names: &[&str],
) -> Result<(), ProtocolError> {
    for field in field_names {
        if let Some(v) = json_map.get_mut(*field) {
            let mut json_value = Value::Null;
            std::mem::swap(v, &mut json_value);
            let data_bytes: Vec<u8> = serde_json::from_value(json_value).map_err(|e| {
                ProtocolError::DecodingError(format!("unable to decode '{}' - {:?}", field, e))
            })?;
            let identifier = Identifier::from_bytes(&data_bytes)?;
            *v = Value::String(identifier.to_string(Encoding::Base58));
        } else {
            return Err(ProtocolError::ParsingError(format!(
                "unable to find '{}'",
                field
            )));
        };
    }
    Ok(())
}

// replaces field names of type JsonValue::Vec<u8> with JsonValue::String().
// The base64 is used for conversion
pub fn parse_bytes(
    json_map: &mut Map<String, Value>,
    field_names: &[&str],
) -> Result<(), ProtocolError> {
    for field in field_names {
        if let Some(v) = json_map.get_mut(*field) {
            let mut json_value = Value::Null;
            std::mem::swap(v, &mut json_value);
            let data_bytes: Vec<u8> = serde_json::from_value(json_value).map_err(|e| {
                ProtocolError::DecodingError(format!("unable to decode '{}'  - {:?}", field, e))
            })?;

            *v = Value::String(base64::encode(data_bytes));
        } else {
            return Err(ProtocolError::ParsingError(format!(
                "unable to find '{}'",
                field
            )));
        };
    }
    Ok(())
}

pub fn parse_protocol_version(
    protocol_bytes: &[u8],
    json_map: &mut Map<String, Value>,
) -> Result<(), ProtocolError> {
    let protocol_version = get_protocol_version(protocol_bytes)?;
    json_map.insert(
        String::from("$protocolVersion"),
        Value::Number(Number::from(protocol_version)),
    );
    Ok(())
}

pub fn identifiers_to_base58(
    binary_properties: &HashMap<String, JsonValue>,
    dynamic_data: &mut JsonValue,
) {
    let identifier_paths = binary_properties
        .iter()
        .filter(|(_, p)| identifier_filter(p))
        .map(|(name, _)| name.as_str());

    replace_paths_with_base58(identifier_paths, dynamic_data)
}

pub fn identifiers_to_bytes(
    binary_properties: &HashMap<String, JsonValue>,
    dynamic_data: &mut JsonValue,
) {
    let identifier_paths = binary_properties
        .iter()
        .filter(|(_, p)| identifier_filter(p))
        .map(|(name, _)| name.as_str());

    replace_paths_with_bytes(identifier_paths, dynamic_data)
}

fn identifier_filter(value: &JsonValue) -> bool {
    if let JsonValue::Object(object) = value {
        if let Some(JsonValue::String(media_type)) = object.get(PROPERTY_CONTENT_MEDIA_TYPE) {
            return media_type == identifier::MEDIA_TYPE;
        }
    }
    false
}

pub fn replace_paths_with_base58<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    value: &mut JsonValue,
) {
    for p in paths {
        replace_path_with_base58(p, value);
    }
}

pub fn replace_path_with_base58(path: &str, value: &mut JsonValue) -> Option<()> {
    unimplemented!()
}

pub fn replace_paths_with_bytes<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    value: &mut JsonValue,
) {
    for p in paths {
        replace_path_with_bytes(p, value);
    }
}

pub fn replace_path_with_bytes(path: &str, value: &mut JsonValue) -> Option<()> {
    unimplemented!()
}
