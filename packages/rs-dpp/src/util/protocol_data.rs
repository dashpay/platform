use serde_json::{Map, Value};

use crate::SerdeParsingError;

pub fn get_raw_public_keys(
    identity_map: &Map<String, Value>,
) -> Result<&Vec<Value>, SerdeParsingError> {
    identity_map
        .get("publicKeys")
        .ok_or_else(|| SerdeParsingError::new("Expected identity.publicKeys to exist"))?
        .as_array()
        .ok_or_else(|| SerdeParsingError::new("Expected identity.publicKeys to be an array"))
}
