use crate::errors::ProtocolError;
use crate::serialization::ValueConvertible;
use platform_value::Value;

pub trait IdentityPlatformValueConversionMethodsV0<'a>: ValueConvertible<'a> {
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError>
    where
        Self: Sized + Clone,
    {
        self.to_object()
    }
}
