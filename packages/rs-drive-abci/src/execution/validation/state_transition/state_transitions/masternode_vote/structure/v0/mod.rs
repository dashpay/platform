// use dpp::platform_value::
use crate::error::Error;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::validation::SimpleConsensusValidationResult;

const MIN_TRANSFER_AMOUNT: u64 = 1000;

pub(in crate::execution::validation::state_transition::state_transitions::masternode_vote) trait MasternodeVoteStateTransitionStructureValidationV0
{
    fn validate_base_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl MasternodeVoteStateTransitionStructureValidationV0 for MasternodeVoteTransition {
    fn validate_base_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let result = SimpleConsensusValidationResult::new();

        Ok(result)
    }
}
