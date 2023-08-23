use crate::error::Error;

use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::validation::SimpleConsensusValidationResult;

pub(in crate::execution::validation::state_transition::state_transitions::identity_top_up) trait IdentityTopUpStateTransitionStructureValidationV0
{
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityTopUpStateTransitionStructureValidationV0 for IdentityTopUpTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        // TODO: Add validation for the structure of the IdentityTopUpTransition
        Ok(SimpleConsensusValidationResult::default())
    }
}
