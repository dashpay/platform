use lazy_static::lazy_static;
use platform_value::Value;
use serde_json::Value as JsonValue;

use crate::validation::SimpleConsensusValidationResult;
use crate::{
    validation::JsonSchemaValidator, version::ProtocolVersionValidator,
    DashPlatformProtocolInitError, NonConsensusError,
};

lazy_static! {
    pub static ref IDENTITY_CREDIT_TRANSFER_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(
        include_str!("../../../../../schema/identity/stateTransition/identityCreditTransfer.json")
    )
    .unwrap();
    pub static ref IDENTITY_CREDIT_TRANSFER_TRANSITION_SCHEMA_VALIDATOR: JsonSchemaValidator =
        JsonSchemaValidator::new(IDENTITY_CREDIT_TRANSFER_TRANSITION_SCHEMA.clone())
            .expect("unable to compile jsonschema");
}

pub struct IdentityCreditTransferTransitionBasicValidator {
    protocol_version_validator: ProtocolVersionValidator,
    json_schema_validator: JsonSchemaValidator,
}

impl IdentityCreditTransferTransitionBasicValidator {
    pub fn new(
        protocol_version_validator: ProtocolVersionValidator,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator =
            JsonSchemaValidator::new(IDENTITY_CREDIT_TRANSFER_TRANSITION_SCHEMA.clone())?;

        let validator = Self {
            protocol_version_validator,
            json_schema_validator,
        };

        Ok(validator)
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

        Ok(result)
    }

    pub fn protocol_version_validator(&mut self) -> &mut ProtocolVersionValidator {
        &mut self.protocol_version_validator
    }
}
