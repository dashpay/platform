use anyhow::anyhow;
use lazy_static::lazy_static;
use platform_value::Value;
use serde_json::Value as JsonValue;
use std::sync::Arc;

use crate::{
    identity::{
        state_transition::validate_public_key_signatures::TPublicKeysSignaturesValidator,
        validation::TPublicKeysValidator,
    },
    validation::{JsonSchemaValidator, SimpleConsensusValidationResult},
    version::ProtocolVersionValidator,
    NonConsensusError, ProtocolError,
};

use super::identity_update_transition::property_names;

lazy_static! {
    pub static ref IDENTITY_UPDATE_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "./../../../schema/identity/stateTransition/identityUpdate.json"
    ))
    .expect("Identity Update Schema file should exist");
    pub static ref IDENTITY_UPDATE_JSON_SCHEMA_VALIDATOR: JsonSchemaValidator =
        JsonSchemaValidator::new(IDENTITY_UPDATE_SCHEMA.clone())
            .expect("unable to compile jsonschema");
}

pub struct ValidateIdentityUpdateTransitionBasic<KV, SV> {
    protocol_version_validator: ProtocolVersionValidator,
    json_schema_validator: JsonSchemaValidator,
    public_keys_validator: Arc<KV>,
    public_keys_signatures_validator: Arc<SV>,
}

impl<KV, SV> ValidateIdentityUpdateTransitionBasic<KV, SV>
where
    KV: TPublicKeysValidator,
    SV: TPublicKeysSignaturesValidator,
{
    pub fn new(
        protocol_version_validator: ProtocolVersionValidator,
        public_keys_validator: Arc<KV>,
        public_keys_signatures_validator: Arc<SV>,
    ) -> Result<Self, ProtocolError> {
        let json_schema_validator = JsonSchemaValidator::new(IDENTITY_UPDATE_SCHEMA.clone())
            .map_err(|e| {
                anyhow!(
                    "creating schema validator for Identity Update failed: {}",
                    e
                )
            })?;
        Ok(Self {
            protocol_version_validator,
            public_keys_validator,
            json_schema_validator,
            public_keys_signatures_validator,
        })
    }

    pub fn validate(
        &self,
        raw_state_transition: &Value,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        let result = self.json_schema_validator.validate(
            &raw_state_transition
                .try_to_validating_json()
                .map_err(NonConsensusError::ValueError)?,
        )?;
        if !result.is_valid() {
            return Ok(result);
        }

        let protocol_version = raw_state_transition
            .get_integer(property_names::PROTOCOL_VERSION)
            .map_err(NonConsensusError::ValueError)?;

        let result = self.protocol_version_validator.validate(protocol_version)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let maybe_raw_public_keys = raw_state_transition
            .get_optional_array_slice(property_names::ADD_PUBLIC_KEYS)
            .map_err(NonConsensusError::ValueError)?;

        match maybe_raw_public_keys {
            Some(raw_public_keys) => {
                let result = self.public_keys_validator.validate_keys(raw_public_keys)?;
                if !result.is_valid() {
                    return Ok(result);
                }

                let result = self
                    .public_keys_signatures_validator
                    .validate_public_key_signatures(raw_state_transition, raw_public_keys)?;
                if !result.is_valid() {
                    return Ok(result);
                }

                Ok(result)
            }
            None => Ok(result),
        }
    }

    pub fn protocol_version_validator(&mut self) -> &mut ProtocolVersionValidator {
        &mut self.protocol_version_validator
    }
}
