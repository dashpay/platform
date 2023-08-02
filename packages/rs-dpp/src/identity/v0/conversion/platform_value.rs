use crate::identity::conversion::platform_value::IdentityPlatformValueConversionMethodsV0;
use crate::identity::{property_names, IdentityV0};
use crate::serialization::ValueConvertible;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use std::convert::TryInto;

impl<'a> ValueConvertible<'a> for IdentityV0 {}

impl<'a> IdentityPlatformValueConversionMethodsV0<'a> for IdentityV0 {
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        //same as object for Identities
        let mut value = self.to_object()?;
        if let Some(keys) = value.get_optional_array_mut_ref(property_names::PUBLIC_KEYS)? {
            for key in keys.iter_mut() {
                key.remove_optional_value_if_null("disabledAt")?;
            }
        }
        Ok(value)
    }
}
