use dpp::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::identity_update_transition::validate_identity_update_transition_basic::IDENTITY_UPDATE_JSON_SCHEMA_VALIDATOR;
use dpp::validation::SimpleConsensusValidationResult;
use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_identity_public_keys_structure::v0::validate_identity_public_keys_structure_v0;
use crate::execution::validation::state_transition::common::validate_protocol_version::v0::validate_protocol_version_v0;
use crate::execution::validation::state_transition::common::validate_schema::v0::validate_schema_v0;

pub(crate) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for IdentityUpdateTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema_v0(&IDENTITY_UPDATE_JSON_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version_v0(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        validate_identity_public_keys_structure_v0(self.add_public_keys.as_slice())
    }
}
