use crate::version::PlatformVersion;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

pub trait DataContractJsonConversionMethodsV0 {
    fn from_json(
        json_value: JsonValue,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    /// Returns Data Contract as a JSON Value
    fn to_json(&self, platform_version: &PlatformVersion) -> Result<JsonValue, ProtocolError>;
    /// Returns Data Contract as a JSON Value
    fn to_validating_json(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError>;
}
