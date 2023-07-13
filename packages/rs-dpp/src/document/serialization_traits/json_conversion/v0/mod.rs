use crate::ProtocolError;
use platform_value::Identifier;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::convert::TryInto;

pub trait DocumentJsonMethodsV0 {
    fn to_json_with_identifiers_using_bytes(&self) -> Result<JsonValue, ProtocolError>;
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    fn from_json_value<S>(document_value: JsonValue) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
        Self: Sized;
}
