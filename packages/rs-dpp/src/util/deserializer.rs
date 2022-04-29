use crate::errors::ProtocolError;
use crate::identifier::Identifier;
use crate::util::string_encoding::Encoding;
use serde_json::{Map, Number, Value as JsonValue};
use std::convert::TryInto;

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

// replaces Identifiers field names of type JsonValue::Vec<u8> with JsonValue::String().
// The base58 is used for conversion
pub fn parse_identities(
    json_map: &mut Map<String, JsonValue>,
    field_names: &[&str],
) -> Result<(), ProtocolError> {
    for field in field_names {
        if let Some(v) = json_map.get_mut(*field) {
            let mut json_value = JsonValue::Null;
            std::mem::swap(v, &mut json_value);
            let data_bytes: Vec<u8> = serde_json::from_value(json_value).map_err(|e| {
                ProtocolError::DecodingError(format!("unable to decode '{}' - {:?}", field, e))
            })?;
            let identifier = Identifier::from_bytes(&data_bytes)?;
            *v = JsonValue::String(identifier.to_string(Encoding::Base58));
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
    json_map: &mut Map<String, JsonValue>,
    field_names: &[&str],
) -> Result<(), ProtocolError> {
    for field in field_names {
        if let Some(v) = json_map.get_mut(*field) {
            let mut json_value = JsonValue::Null;
            std::mem::swap(v, &mut json_value);
            let data_bytes: Vec<u8> = serde_json::from_value(json_value).map_err(|e| {
                ProtocolError::DecodingError(format!("unable to decode '{}'  - {:?}", field, e))
            })?;

            *v = JsonValue::String(base64::encode(data_bytes));
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
    json_map: &mut Map<String, JsonValue>,
) -> Result<(), ProtocolError> {
    let protocol_version = get_protocol_version(protocol_bytes)?;
    json_map.insert(
        String::from("$protocolVersion"),
        JsonValue::Number(Number::from(protocol_version)),
    );
    Ok(())
}
