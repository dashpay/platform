use crate::ProtocolError;
use serde_json::Value as JsonValue;

pub trait IdentityPublicKeyJsonConversionMethodsV0 {
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    fn to_json_object(&self) -> Result<JsonValue, ProtocolError>;
    fn from_json_object(raw_object: JsonValue) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
