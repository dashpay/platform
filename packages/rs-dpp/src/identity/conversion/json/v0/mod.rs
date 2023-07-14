use crate::ProtocolError;
use serde_json::Value as JsonValue;

pub trait IdentityJsonConversionMethodsV0 {
    fn to_json_object(&self) -> Result<JsonValue, ProtocolError>;
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    fn from_json(json_object: JsonValue) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
