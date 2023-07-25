use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use crate::serialization_traits::ValueConvertible;

pub trait IdentityPlatformValueConversionMethodsV0 : ValueConvertible {
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        self.to_object()
    }
}
