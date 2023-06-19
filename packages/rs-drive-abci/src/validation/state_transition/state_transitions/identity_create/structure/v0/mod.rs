use crate::error::Error;
use crate::validation::state_transition::common::{validate_protocol_version, validate_schema};
use crate::validation::state_transition::key_validation::validate_identity_public_keys_structure;
use dpp::identity::state_transition::identity_create_transition::validation::basic::IDENTITY_CREATE_TRANSITION_SCHEMA_VALIDATOR;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;

pub(in crate::validation::state_transition) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for IdentityCreateTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&IDENTITY_CREATE_TRANSITION_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        validate_identity_public_keys_structure(self.public_keys.as_slice())
    }
}
