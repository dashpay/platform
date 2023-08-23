use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use platform_value::Value;

pub trait IdentityPlatformValueConversionMethodsV0<'a>: ValueConvertible<'a> {
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError>
    where
        Self: Sized + Clone,
    {
        self.to_object()
    }
}
