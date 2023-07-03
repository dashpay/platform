use lazy_static::lazy_static;
use platform_value::Value;
use serde_json::Value as JsonValue;
use std::sync::Arc;

use crate::identity::validation::TPublicKeysValidator;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::version::ProtocolVersionValidator;
use crate::{DashPlatformProtocolInitError, NonConsensusError};
use crate::identity::state_transition::identity_update_transition::identity_update_transition::property_names::PROTOCOL_VERSION;

lazy_static! {
    static ref IDENTITY_JSON_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("./../../schema/identity/identity.json"))
            .expect("Identity Schema file should exist");
}

pub struct IdentityValidator<TPublicKeyValidator> {
    protocol_version_validator: Arc<ProtocolVersionValidator>,
    json_schema_validator: JsonSchemaValidator,
    public_keys_validator: Arc<TPublicKeyValidator>,
}

impl<T: TPublicKeysValidator> IdentityValidator<T> {
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        public_keys_validator: Arc<T>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator = JsonSchemaValidator::new(IDENTITY_JSON_SCHEMA.clone())?;

        let identity_validator = Self {
            protocol_version_validator,
            json_schema_validator,
            public_keys_validator,
        };

        Ok(identity_validator)
    }

    pub fn validate_identity_object(
        &self,
        identity_object: &Value,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        let mut validation_result = self.json_schema_validator.validate(
            &identity_object
                .try_to_validating_json()
                .map_err(NonConsensusError::ValueError)?,
        )?;

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let protocol_version = identity_object.get_integer(PROTOCOL_VERSION)?;
        validation_result.merge(self.protocol_version_validator.validate(protocol_version)?);

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let raw_public_keys = identity_object.get_array_slice("publicKeys")?;
        validation_result.merge(self.public_keys_validator.validate_keys(raw_public_keys)?);

        Ok(validation_result)
    }
}

// fn get_protocol_version(identity_map: &Map<String, Value>) -> Result<u32, SerdeParsingError> {
//     Ok(identity_map
//         .get("protocolVersion")
//         .ok_or_else(|| SerdeParsingError::new("Expected identity to have protocolVersion"))?
//         .as_u64()
//         .ok_or_else(|| SerdeParsingError::new("Expected protocolVersion to be a uint"))?
//         as u32)
// }
//
