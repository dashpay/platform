use std::sync::Arc;

use lazy_static::lazy_static;
use serde_json::Value;

use crate::{
    consensus::basic::identity::{
        InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
        InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
    },
    identity::core_script::CoreScript,
    util::{is_fibonacci_number::is_fibonacci_number, protocol_data::get_protocol_version},
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

        let identity_credit_withdrawal_transition_map =
            transition_json.as_object().ok_or_else(|| {
                SerdeParsingError::new(
                    "Expected identity credit withdrawal transition to be a json object",
                )
            })?;

        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.protocol_version_validator
                .validate(get_protocol_version(
                    identity_credit_withdrawal_transition_map,
                )?)?,
        );

        if !result.is_valid() {
            return Ok(result);
        }

        // validate core_fee is in fibonacci sequence
        let core_fee = transition_json
            .get("coreFee")
            .ok_or_else(|| {
                SerdeParsingError::new("Expected credit withdrawal transition to have coreFee")
            })?
            .as_u64()
            .ok_or_else(|| SerdeParsingError::new("Expected coreFee to be a uint"))?;

        if !is_fibonacci_number(core_fee) {
            result.add_error(InvalidIdentityCreditWithdrawalTransitionCoreFeeError::new(
                core_fee as u32,
            ));
        }

        if !result.is_valid() {
            return Ok(result);
        }

        // validate output_script types
        let output_script_value = transition_json.get("outputScript").ok_or_else(|| {
            SerdeParsingError::new("Expected credit withdrawal transition to have outputScript")
        })?;

        let output_script_bytes: Vec<u8> = serde_json::from_value(output_script_value.clone())?;

        let output_script = CoreScript::from_bytes(output_script_bytes);

        if !output_script.is_p2pkh() && !output_script.is_p2sh() {
            result.add_error(
                InvalidIdentityCreditWithdrawalTransitionOutputScriptError::new(output_script),
            );
        }

        Ok(result)
    }
}
