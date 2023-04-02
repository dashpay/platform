use dpp::{
    identity::state_transition::identity_create_transition::validation::basic::INDENTITY_CREATE_TRANSITION_SCHEMA,
    state_transition::StateTransitionConvert,
    validation::{ConsensusValidationResult, JsonSchemaValidator},
    version::ProtocolVersionValidator,
};
use dpp::{
    identity::state_transition::identity_create_transition::IdentityCreateTransition,
    state_transition::StateTransitionAction, validation::SimpleConsensusValidationResult,
};
use drive::{drive::Drive, grovedb::Transaction};

use crate::error::Error;

use super::StateTransitionValidation;

impl StateTransitionValidation for IdentityCreateTransition {
    fn validate_type(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // Reuse jsonschema validation on a whole state transition
        let json_schema_validator =
            JsonSchemaValidator::new(INDENTITY_CREATE_TRANSITION_SCHEMA.clone())
                .expect("unable to compile jsonschema");
        let result = json_schema_validator
            .validate(
                &(self
                    .to_object(true)
                    .expect("data contract is serializable")
                    .try_into_validating_json()
                    .expect("TODO")),
            )
            .expect("TODO: how jsonschema validation will ever fail?");
        if !result.is_valid() {
            return Ok(result);
        }

        // Validate protocol version
        let protocol_version_validator = ProtocolVersionValidator::default();
        let result = protocol_version_validator
            .validate(self.protocol_version)
            .expect("TODO: again, how this will ever fail, why do we even need a validator trait");
        if !result.is_valid() {
            return Ok(result);
        }

        todo!()
    }

    fn validate_signature(&self, drive: &Drive) -> Result<SimpleConsensusValidationResult, Error> {
        todo!()
    }

    fn validate_key_signature(&self) -> Result<SimpleConsensusValidationResult, Error> {
        todo!()
    }

    fn validate_state(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}
