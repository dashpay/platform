use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dashcore_rpc::dashcore_rpc_json::MasternodeType;
use dpp::consensus::state::voting::masternode_not_found_error::MasternodeNotFoundError;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::ProTxHash;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use drive::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::masternode_vote) trait MasternodeVoteStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<MasternodeVoteTransitionAction>, Error>;
}

impl MasternodeVoteStateTransitionTransformIntoActionValidationV0 for MasternodeVoteTransition {
    fn transform_into_action_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<MasternodeVoteTransitionAction>, Error> {
        let Some(masternode) = platform
            .state
            .full_masternode_list()
            .get(&ProTxHash::from_byte_array(self.pro_tx_hash().to_buffer()))
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                MasternodeNotFoundError::new(self.pro_tx_hash()).into(),
            ));
        };

        let strength = match masternode.node_type {
            MasternodeType::Regular => 1,
            MasternodeType::Evo => 4,
        };

        Ok(ConsensusValidationResult::new_with_data(
            MasternodeVoteTransitionAction::transform_from_transition(
                self,
                strength,
                platform.drive,
                tx,
                platform_version,
            )?,
        ))
    }
}
