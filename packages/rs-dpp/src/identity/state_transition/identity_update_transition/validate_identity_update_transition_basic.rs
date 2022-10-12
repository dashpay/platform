use anyhow::anyhow;
use lazy_static::lazy_static;
use serde_json::Value as JsonValue;
use std::{marker::PhantomData, sync::Arc};

use crate::{
    identity::{
        state_transition::validate_public_key_signatures::TPublicKeysSignaturesValidator,
        validation::TPublicKeysValidator,
    },
    util::json_value::JsonValueExt,
    validation::{JsonSchemaValidator, SimpleValidationResult},
    version::ProtocolVersionValidator,
    NonConsensusError, ProtocolError,
};

use super::identity_update_transition::property_names;

lazy_static! {
    static ref IDENTITY_UPDATE_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "./../../../schema/identity/stateTransition/identityUpdate.json"
    ))
    .expect("Identity Update Schema file should exist");
}

pub struct ValidateIdentityUpdateTransitionBasic<KV, SV> {
    protocol_version_validator: Arc<ProtocolVersionValidator>,
    json_schema_validator: JsonSchemaValidator,
    public_keys_validator: Arc<KV>,

    _public_keys_signatures_validator: PhantomData<SV>,
}

impl<KV, SV> ValidateIdentityUpdateTransitionBasic<KV, SV>
where
    KV: TPublicKeysValidator,
    SV: TPublicKeysSignaturesValidator,
{
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        public_keys_validator: Arc<KV>,
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
            _public_keys_signatures_validator: PhantomData,
        })
    }

    pub fn validate(
        &self,
        raw_state_transition: &JsonValue,
    ) -> Result<SimpleValidationResult, NonConsensusError> {
        let result = self.json_schema_validator.validate(raw_state_transition)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let protocol_version = raw_state_transition
            .get_u64(property_names::PROTOCOL_VERSION)
            .map_err(|e| NonConsensusError::SerdeJsonError(e.to_string()))?;

        let result = self
            .protocol_version_validator
            .validate(protocol_version as u32)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let maybe_raw_public_keys = raw_state_transition.get(property_names::ADD_PUBLIC_KEYS);

        match maybe_raw_public_keys {
            Some(raw_public_keys) => {
                let raw_public_keys_list = raw_public_keys.as_array().ok_or_else(|| {
                    NonConsensusError::SerdeJsonError(format!(
                        "'{}' property isn't an array",
                        property_names::ADD_PUBLIC_KEYS
                    ))
                })?;
                let result = self
                    .public_keys_validator
                    .validate_keys(raw_public_keys_list)?;
                if !result.is_valid() {
                    return Ok(result);
                }

                let result =
                    SV::validate_public_key_signatures(raw_state_transition, raw_public_keys_list)?;
                if !result.is_valid() {
                    return Ok(result);
                }

                Ok(result)
            }
            None => Ok(result),
        }
    }
}
