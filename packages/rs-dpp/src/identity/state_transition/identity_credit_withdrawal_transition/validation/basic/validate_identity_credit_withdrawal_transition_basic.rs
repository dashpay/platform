use std::sync::Arc;

use lazy_static::lazy_static;
use serde_json::Value;

use crate::{
    util::protocol_data::get_protocol_version,
    validation::{JsonSchemaValidator, ValidationResult},
    version::ProtocolVersionValidator,
    DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError,
};

lazy_static! {
    static ref INDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA: Value =
        serde_json::from_str(include_str!(
            "../../../../../schema/identity/stateTransition/identityCreditWithdrawal.json"
        ))
        .unwrap();
}

pub struct IdentityCreditWithdrawalTransitionBasicValidator {
    protocol_version_validator: Arc<ProtocolVersionValidator>,
    json_schema_validator: JsonSchemaValidator,
}

impl IdentityCreditWithdrawalTransitionBasicValidator {
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator =
            JsonSchemaValidator::new(INDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA.clone())?;

        let identity_validator = Self {
            protocol_version_validator,
            json_schema_validator,
        };

        Ok(identity_validator)
    }

    pub async fn validate(
        &self,
        transition_json: &Value,
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        let mut result = self.json_schema_validator.validate(transition_json)?;

        let identity_transition_map = transition_json.as_object().ok_or_else(|| {
            SerdeParsingError::new("Expected identity transition to be a json object")
        })?;

        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.protocol_version_validator
                .validate(get_protocol_version(identity_transition_map)?)?,
        );

        if !result.is_valid() {
            return Ok(result);
        }

        Ok(result)
    }
}
