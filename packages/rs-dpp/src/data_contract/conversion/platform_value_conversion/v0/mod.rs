use platform_value::Value;
use crate::data_contract::property_names;
use crate::data_contract::property_names::SYSTEM_VERSION;
use crate::ProtocolError;
use crate::version::PlatformVersion;

pub trait DataContractValueConversionMethodsV0 {
    fn from_raw_object(
        raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> where Self: Sized;
    fn to_object(&self) -> Result<Value, ProtocolError>;
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError>;
    fn into_object(self) -> Result<Value, ProtocolError>;
}
