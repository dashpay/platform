use crate::error::Error;

use crate::execution::validation::state_transition::common::validate_protocol_version::v0::validate_protocol_version_v0;
use crate::execution::validation::state_transition::common::validate_schema::v0::validate_schema_v0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::validation::SimpleConsensusValidationResult;

pub(crate) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for IdentityTopUpTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema_v0(&IDENTITY_TOP_UP_TRANSITION_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(validate_protocol_version_v0(self.protocol_version))
    }
}
