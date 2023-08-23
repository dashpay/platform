use crate::identity::identity_public_key::conversion::json::IdentityPublicKeyJsonConversionMethodsV0;
use crate::identity::identity_public_key::conversion::platform_value::IdentityPublicKeyPlatformValueConversionMethodsV0;
use crate::identity::identity_public_key::fields::BINARY_DATA_FIELDS;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{ReplacementType, Value};
use serde_json::Value as JsonValue;
use std::convert::{TryFrom, TryInto};

impl IdentityPublicKeyJsonConversionMethodsV0 for IdentityPublicKeyV0 {
    fn to_json_object(&self) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object()?
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }

    fn from_json_object(
        raw_object: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut value: Value = raw_object.into();
        value.replace_at_paths(BINARY_DATA_FIELDS, ReplacementType::BinaryBytes)?;
        Self::from_object(value, platform_version)
    }
}

impl TryFrom<&str> for IdentityPublicKeyV0 {
    type Error = ProtocolError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut platform_value: Value = serde_json::from_str::<JsonValue>(value)
            .map_err(|e| ProtocolError::StringDecodeError(e.to_string()))?
            .into();
        platform_value.replace_at_paths(BINARY_DATA_FIELDS, ReplacementType::BinaryBytes)?;
        platform_value.try_into().map_err(ProtocolError::ValueError)
    }
}
