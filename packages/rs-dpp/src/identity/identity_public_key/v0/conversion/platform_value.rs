use crate::identity::identity_public_key::conversion::platform_value::IdentityPublicKeyPlatformValueConversionMethodsV0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use std::convert::{TryFrom, TryInto};

impl IdentityPublicKeyPlatformValueConversionMethodsV0 for IdentityPublicKeyV0 {
    fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        let mut value = platform_value::to_value(self).map_err(ProtocolError::ValueError)?;
        if self.disabled_at.is_none() {
            value
                .remove("disabledAt")
                .map_err(ProtocolError::ValueError)?;
        }
        Ok(value)
    }

    fn into_object(self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn from_object(
        value: Value,
        _platform_version: &PlatformVersion,
    ) -> Result<IdentityPublicKeyV0, ProtocolError> {
        value.try_into().map_err(ProtocolError::ValueError)
    }
}

impl TryFrom<&IdentityPublicKeyV0> for Value {
    type Error = platform_value::Error;

    fn try_from(value: &IdentityPublicKeyV0) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

impl TryFrom<IdentityPublicKeyV0> for Value {
    type Error = platform_value::Error;

    fn try_from(value: IdentityPublicKeyV0) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

impl TryFrom<Value> for IdentityPublicKeyV0 {
    type Error = platform_value::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value)
    }
}
