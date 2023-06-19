use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::identity::state_transition::identity_create_transition::validation::basic::IDENTITY_CREATE_TRANSITION_SCHEMA_VALIDATOR;
use dpp::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::identity::state_transition::identity_update_transition::validate_identity_update_transition_basic::IDENTITY_UPDATE_JSON_SCHEMA_VALIDATOR;
use dpp::validation::SimpleConsensusValidationResult;
use crate::error::Error;
use crate::validation::state_transition::common::{validate_protocol_version, validate_schema};
use crate::validation::state_transition::key_validation::validate_identity_public_keys_structure;

pub(in crate::validation::state_transition) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for IdentityUpdateTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&IDENTITY_UPDATE_JSON_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        validate_identity_public_keys_structure(self.add_public_keys.as_slice())
    }
}
