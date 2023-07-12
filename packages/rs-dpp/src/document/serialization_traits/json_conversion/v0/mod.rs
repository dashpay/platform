use std::convert::TryInto;
use serde::Deserialize;
use platform_value::Identifier;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

pub trait DocumentJsonMethodsV0 {
    fn to_json_with_identifiers_using_bytes(&self) -> Result<JsonValue, ProtocolError>;
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    fn from_json_value<S>(document_value: JsonValue) -> Result<Self, ProtocolError>
        where
                for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
                Self: Sized;
}