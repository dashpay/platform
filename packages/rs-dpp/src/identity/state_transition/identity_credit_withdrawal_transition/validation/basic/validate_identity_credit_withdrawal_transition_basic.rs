use std::sync::Arc;

use lazy_static::lazy_static;
use platform_value::Value;
use serde_json::Value as JsonValue;

use crate::validation::SimpleConsensusValidationResult;
use crate::{
    consensus::basic::identity::{
        InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
        InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
        NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
    },
    contracts::withdrawals_contract,
    identity::core_script::CoreScript,
    util::is_fibonacci_number::is_fibonacci_number,
    validation::JsonSchemaValidator,
    version::ProtocolVersionValidator,
    DashPlatformProtocolInitError, NonConsensusError,
};

lazy_static! {
    pub static ref IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA: JsonValue =
        serde_json::from_str(include_str!(
            "../../../../../schema/identity/stateTransition/identityCreditWithdrawal.json"
        ))
        .unwrap();
    pub static ref IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA_VALIDATOR: JsonSchemaValidator =
        JsonSchemaValidator::new(IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA.clone())
            .expect("unable to compile jsonschema");
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
            JsonSchemaValidator::new(IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA.clone())?;

        let identity_validator = Self {
            protocol_version_validator,
            json_schema_validator,
        };

        Ok(identity_validator)
    }

    pub async fn validate(
        &self,
        transition_object: &Value,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        let mut result = self.json_schema_validator.validate(
            &transition_object
                .try_to_validating_json()
                .map_err(NonConsensusError::ValueError)?,
        )?;

        if !result.is_valid() {
            return Ok(result);
        }

        result.merge(
            self.protocol_version_validator.validate(
                transition_object
                    .get_integer("protocolVersion")
                    .map_err(NonConsensusError::ValueError)?,
            )?,
        );

        if !result.is_valid() {
            return Ok(result);
        }

        // validate pooling is always equals to 0
        let pooling = transition_object
            .get_integer(withdrawals_contract::property_names::POOLING)
            .map_err(NonConsensusError::ValueError)?;

        if pooling > 0 {
            result.add_error(
                NotImplementedIdentityCreditWithdrawalTransitionPoolingError::new(pooling),
            );

            return Ok(result);
        }

        // validate core_fee is in fibonacci sequence
        let core_fee_per_byte = transition_object
            .get_integer(withdrawals_contract::property_names::CORE_FEE_PER_BYTE)
            .map_err(NonConsensusError::ValueError)?;

        if !is_fibonacci_number(core_fee_per_byte) {
            result.add_error(InvalidIdentityCreditWithdrawalTransitionCoreFeeError::new(
                core_fee_per_byte,
            ));

            return Ok(result);
        }

        // validate output_script types
        let output_script: CoreScript = transition_object
            .get_bytes_into(withdrawals_contract::property_names::OUTPUT_SCRIPT)
            .map_err(NonConsensusError::ValueError)?;

        if !output_script.is_p2pkh() && !output_script.is_p2sh() {
            result.add_error(
                InvalidIdentityCreditWithdrawalTransitionOutputScriptError::new(output_script),
            );
        }

        Ok(result)
    }
}
