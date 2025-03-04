use crate::errors::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

pub trait DataContractValueConversionMethodsV0 {
    fn from_value(
        raw_object: Value,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    fn to_value(&self, platform_version: &PlatformVersion) -> Result<Value, ProtocolError>;
    fn into_value(self, platform_version: &PlatformVersion) -> Result<Value, ProtocolError>;
}
