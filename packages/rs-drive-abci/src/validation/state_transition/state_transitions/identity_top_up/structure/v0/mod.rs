use crate::error::Error;
use crate::validation::state_transition::common::{validate_protocol_version, validate_schema};

use dpp::identity::state_transition::identity_topup_transition::validation::basic::IDENTITY_TOP_UP_TRANSITION_SCHEMA_VALIDATOR;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::validation::SimpleConsensusValidationResult;

pub(in crate::validation::state_transition) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for IdentityTopUpTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&IDENTITY_TOP_UP_TRANSITION_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(validate_protocol_version(self.protocol_version))
    }
}
