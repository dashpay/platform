use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use platform_value::Value;

pub trait IdentityPlatformValueConversionMethodsV0: ValueConvertible {
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError>
    where
        Self: Sized,
    {
        self.to_object()
    }
}
