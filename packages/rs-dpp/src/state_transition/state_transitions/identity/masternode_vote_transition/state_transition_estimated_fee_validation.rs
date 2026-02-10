use crate::fee::Credits;
use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use crate::state_transition::StateTransitionEstimatedFeeValidation;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for MasternodeVoteTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        Ok(platform_version
            .fee_version
            .state_transition_min_fees
            .masternode_vote)
    }
}
