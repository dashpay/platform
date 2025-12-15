use crate::fee::Credits;
use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use crate::state_transition::StateTransitionEstimatedFeeValidation;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for MasternodeVoteTransition {
    fn calculate_estimated_fee(&self, platform_version: &PlatformVersion) -> Credits {
        platform_version
            .fee_version
            .state_transition_min_fees
            .masternode_vote
    }
}
