use crate::identity::conversion::json::IdentityJsonConversionMethodsV0;
use crate::identity::conversion::platform_value::IdentityPlatformValueConversionMethodsV0;
use crate::identity::{identity_public_key, IdentityV0, IDENTIFIER_FIELDS_RAW_OBJECT};
use crate::ProtocolError;
use platform_value::{ReplacementType, Value};
use serde_json::Value as JsonValue;
use std::convert::TryInto;

impl IdentityJsonConversionMethodsV0 for IdentityV0 {
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

    /// Creates an identity from a json structure
    fn from_json(json_object: JsonValue) -> Result<Self, ProtocolError> {
        let mut platform_value: Value = json_object.into();

        platform_value
            .replace_at_paths(IDENTIFIER_FIELDS_RAW_OBJECT, ReplacementType::Identifier)?;

        if let Some(public_keys_array) = platform_value.get_optional_array_mut_ref("publicKeys")? {
            for public_key in public_keys_array.iter_mut() {
                public_key.replace_at_paths(
                    identity_public_key::BINARY_DATA_FIELDS,
                    ReplacementType::BinaryBytes,
                )?;
            }
        }

        let identity: Self = platform_value::from_value(platform_value)?;

        Ok(identity)
    }
}
