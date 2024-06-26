use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dashcore_rpc::dashcore_rpc_json::MasternodeType;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::voting::masternode_not_found_error::MasternodeNotFoundError;
use dpp::consensus::state::voting::masternode_vote_already_present_error::MasternodeVoteAlreadyPresentError;
use dpp::consensus::state::voting::masternode_voted_too_many_times::MasternodeVotedTooManyTimesError;
use dpp::consensus::ConsensusError;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::ProTxHash;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use drive::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;

use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::Vote;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition::state_transitions::masternode_vote) trait MasternodeVoteStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<MasternodeVoteTransitionAction>, Error>;
}

impl MasternodeVoteStateTransitionTransformIntoActionValidationV0 for MasternodeVoteTransition {
    fn transform_into_action_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<MasternodeVoteTransitionAction>, Error> {
        let mut previous_resource_vote_choice_to_remove = None;
        if validation_mode != ValidationMode::NoValidation {
            // Before we transform into action we want to make sure that we have not yet voted
            match self.vote() {
                Vote::ResourceVote(resource_vote) => {
                    match resource_vote.vote_poll() {
                        VotePoll::ContestedDocumentResourceVotePoll(vote_poll) => {
                            let vote_id = vote_poll.unique_id()?;
                            let maybe_existing_resource_vote_choice =
                                platform.drive.fetch_identity_contested_resource_vote(
                                    self.pro_tx_hash(),
                                    vote_id,
                                    tx,
                                    &mut vec![],
                                    platform_version,
                                )?;
                            if let Some((existing_resource_vote_choice, previous_vote_count)) =
                                maybe_existing_resource_vote_choice
                            {
                                if existing_resource_vote_choice
                                    == resource_vote.resource_vote_choice()
                                {
                                    // We are submitting a vote for something we already have
                                    return Ok(ConsensusValidationResult::new_with_error(
                                        ConsensusError::StateError(
                                            StateError::MasternodeVoteAlreadyPresentError(
                                                MasternodeVoteAlreadyPresentError::new(
                                                    self.pro_tx_hash(),
                                                    resource_vote.vote_poll().clone(),
                                                ),
                                            ),
                                        ),
                                    ));
                                } else if previous_vote_count
                                    >= platform_version
                                        .dpp
                                        .validation
                                        .voting
                                        .votes_allowed_per_masternode
                                {
                                    // We are submitting a vote for something we already have
                                    return Ok(ConsensusValidationResult::new_with_error(
                                        ConsensusError::StateError(
                                            StateError::MasternodeVotedTooManyTimesError(
                                                MasternodeVotedTooManyTimesError::new(
                                                    self.pro_tx_hash(),
                                                    previous_vote_count,
                                                    platform_version
                                                        .dpp
                                                        .validation
                                                        .voting
                                                        .votes_allowed_per_masternode,
                                                ),
                                            ),
                                        ),
                                    ));
                                } else {
                                    previous_resource_vote_choice_to_remove =
                                        Some((existing_resource_vote_choice, previous_vote_count));
                                }
                            }
                        }
                    }
                }
            }
        }

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
                masternode.state.voting_address,
                strength,
                previous_resource_vote_choice_to_remove,
                platform.drive,
                tx,
                platform_version,
            )?,
        ))
    }
}
