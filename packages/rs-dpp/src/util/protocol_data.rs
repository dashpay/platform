use serde_json::{Map, Value};

use crate::SerdeParsingError;

pub fn get_protocol_version(
    protocol_structure_json: &Map<String, Value>,
) -> Result<u32, SerdeParsingError> {
    Ok(protocol_structure_json
        .get("protocolVersion")
        .ok_or_else(|| SerdeParsingError::new("Expected identity to have protocolVersion"))?
        .as_u64()
        .ok_or_else(|| SerdeParsingError::new("Expected protocolVersion to be a uint"))?
        as u32)
}

pub fn get_raw_public_keys(
    identity_map: &Map<String, Value>,
) -> Result<&Vec<Value>, SerdeParsingError> {
    identity_map
        .get("publicKeys")
        .ok_or_else(|| SerdeParsingError::new("Expected identity.publicKeys to exist"))?
        .as_array()
        .ok_or_else(|| SerdeParsingError::new("Expected identity.publicKeys to be an array"))
}
