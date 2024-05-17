use crate::errors::ProtocolError;
// use serde_json::Value as JsonValue;

pub trait IdentityJsonConversionMethodsV0 {
    fn to_json_object(&self) -> Result<serde_json::Value, ProtocolError>;
    fn to_json(&self) -> Result<serde_json::Value, ProtocolError>;
    fn from_json(json_object: serde_json::Value) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
