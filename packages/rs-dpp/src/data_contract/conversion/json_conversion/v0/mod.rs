use crate::version::PlatformVersion;
use crate::ProtocolError;
use serde_json::{Value as JsonValue, Value};

pub trait DataContractJsonConversionMethodsV0 {
    fn from_json_object(
        json_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Returns Data Contract as a JSON Value
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    fn to_json_object(&self) -> Result<JsonValue, ProtocolError>;
}
