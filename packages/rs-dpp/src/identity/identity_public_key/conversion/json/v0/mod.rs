use crate::errors::ProtocolError;
use platform_version::version::PlatformVersion;
// use serde_json::Value as JsonValue;

pub trait IdentityPublicKeyJsonConversionMethodsV0 {
    fn to_json(&self) -> Result<serde_json::Value, ProtocolError>;
    fn to_json_object(&self) -> Result<serde_json::Value, ProtocolError>;
    fn from_json_object(
        raw_object: serde_json::Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
