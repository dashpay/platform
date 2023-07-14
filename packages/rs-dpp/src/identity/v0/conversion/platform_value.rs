use std::convert::TryInto;
use platform_value::Value;
use crate::identity::conversion::platform_value::IdentityPlatformValueConversionMethodsV0;
use crate::identity::{IdentityV0, property_names};
use crate::ProtocolError;
use crate::version::PlatformVersion;

impl IdentityPlatformValueConversionMethodsV0 for IdentityV0 {
    fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn into_object(self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

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



    /// Creates an identity from a raw object
    fn from_object(raw_object: Value, platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        raw_object.try_into()
    }
}


