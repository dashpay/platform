use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use drive::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;

use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::masternode_vote) trait MasternodeVoteStateTransitionStateValidationV0
{
    fn validate_state_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl MasternodeVoteStateTransitionStateValidationV0 for MasternodeVoteTransition {
    fn validate_state_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        self.transform_into_action_v0(platform, tx, platform_version)
    }

    fn transform_into_action_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        Ok(ConsensusValidationResult::new_with_data(
            MasternodeVoteTransitionAction::transform_from_transition(
                self,
                platform.drive,
                tx,
                platform_version,
            )?
            .into(),
        ))
    }
}
