use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

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
