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

use crate::{error::Error, validation::state_transition::common::{validate_protocol_version, validate_schema}};

use super::StateTransitionValidation;

impl StateTransitionValidation for IdentityCreateTransition {
    fn validate_type(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(INDENTITY_CREATE_TRANSITION_SCHEMA.clone(), self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version(self.protocol_version);
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
