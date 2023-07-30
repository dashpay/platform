use crate::data_contract::property_names;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

pub trait DataContractValueConversionMethodsV0 {
    fn from_object(
        raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    fn to_object(&self) -> Result<Value, ProtocolError>;
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError>;
    fn into_object(self) -> Result<Value, ProtocolError>;
}
