use crate::errors::ProtocolError;
use crate::identifier;
use crate::identifier::Identifier;
use crate::util::string_encoding::Encoding;
use anyhow::{anyhow, bail};
use serde_json::Value as JsonValue;
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::convert::TryFrom;
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

/// replaces identifiers stored as Vec<u8> with base58 encoded string
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

/// replaces identifiers stored as base58 encoded string with bytes (Vec<u8>)
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

pub fn replace_paths_with_base58<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    value: &mut JsonValue,
) {
    for p in paths {
        // we ignore errors so far
        let _ = replace_path_with_base58(p, value);
    }
}

pub fn replace_path_with_base58(path: &str, value: &mut JsonValue) -> Result<(), anyhow::Error> {
    let path_literal = JsonPathLiteral(path);
    let path: JsonPath = path_literal.try_into().unwrap();

    let mut json_value = Value::Null;
    let v = get_value_from_path_mut(&path, value)
        .ok_or_else(|| anyhow!("path '{:?}' not found", path))?;

    std::mem::swap(v, &mut json_value);
    let data_bytes: Vec<u8> = serde_json::from_value(json_value)
        .map_err(|e| anyhow!("unable to decode '{:?}' - {:?}", path, e))?;

    let identifier = Identifier::from_bytes(&data_bytes)?;
    *v = Value::String(identifier.to_string(Encoding::Base58));

    Ok(())
}

pub fn replace_paths_with_bytes<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    value: &mut JsonValue,
) {
    for p in paths {
        let _ = replace_path_with_bytes(p, value);
    }
}

pub fn replace_path_with_bytes(path: &str, value: &mut JsonValue) -> Result<(), anyhow::Error> {
    let path_literal = JsonPathLiteral(path);
    let path: JsonPath = path_literal.try_into().unwrap();

    let mut json_value = Value::Null;
    let v = get_value_from_path_mut(&path, value)
        .ok_or_else(|| anyhow!("path '{:?}' not found", path))?;

    std::mem::swap(v, &mut json_value);
    let data_string: String = serde_json::from_value(json_value)
        .map_err(|e| anyhow!("unable to deserialize to string '{:?}' - {:?}", path, e))?;

    let identifier = Identifier::from_string(&data_string, Encoding::Base58)?.to_vec();
    *v = Value::Array(identifier);
    Ok(())
}

pub fn get_value_from_path_mut<'a>(
    path: &[JsonPathStep],
    value: &'a mut JsonValue,
) -> Option<&'a mut JsonValue> {
    let mut last_ptr: &mut JsonValue = value;

    for step in path {
        match step {
            JsonPathStep::Index(index) => {
                last_ptr = last_ptr.get_mut(index)?;
            }

            JsonPathStep::Key(key) => {
                last_ptr = last_ptr.get_mut(key)?;
            }
        }
    }
    Some(last_ptr)
}

#[derive(Debug, Clone)]
pub enum JsonPathStep {
    Key(String),
    Index(usize),
}

pub struct JsonPathLiteral<'a>(&'a str);
impl<'a> std::ops::Deref for JsonPathLiteral<'a> {
    type Target = &'a str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type JsonPath = Vec<JsonPathStep>;

impl<'a> TryFrom<JsonPathLiteral<'a>> for JsonPath {
    type Error = ProtocolError;

    fn try_from(path: JsonPathLiteral<'a>) -> Result<Self, Self::Error> {
        let mut steps: Vec<JsonPathStep> = vec![];
        let raw_steps = path.split('.');

        for step in raw_steps {
            if let Ok((step_key, step_index)) = try_parse_indexed_field(step) {
                steps.push(JsonPathStep::Key(step_key.to_string()));
                steps.push(JsonPathStep::Index(step_index));
            } else {
                steps.push(JsonPathStep::Key(step.to_string()))
            };
        }
        Ok(steps)
    }
}

// try to parse indexed step path. i.e: "property_name[0]"
fn try_parse_indexed_field(step: &str) -> Result<(String, usize), anyhow::Error> {
    let chars: Vec<char> = step.chars().collect();
    let index_open = chars.iter().rev().position(|c| c == &'[');
    let index_close = chars.iter().rev().position(|c| c == &']');

    if index_open.is_none() {
        bail!("open index bracket not found");
    }
    if index_close.is_none() {
        bail!("close index bracket not found");
    }
    if index_open > index_close {
        bail!("open bracket is ahead of close bracket")
    }
    if index_close.unwrap() != chars.len() {
        bail!("the close bracket must be the last character")
    }

    let index_str: String = chars[index_open.unwrap() + 1..index_close.unwrap()]
        .iter()
        .collect();

    let index: usize = index_str
        .parse()
        .map_err(|e| anyhow!("unable to parse '{}' into usize: {}", index_str, e))?;
    let key: String = chars[0..index_open.unwrap()].iter().collect();

    Ok((key, index))
}

fn identifier_filter(value: &JsonValue) -> bool {
    if let JsonValue::Object(object) = value {
        if let Some(JsonValue::String(media_type)) = object.get(PROPERTY_CONTENT_MEDIA_TYPE) {
            return media_type == identifier::MEDIA_TYPE;
        }
    }
    false
}
