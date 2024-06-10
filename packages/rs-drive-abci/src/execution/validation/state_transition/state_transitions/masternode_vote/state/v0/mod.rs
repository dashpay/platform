use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::voting::masternode_vote_already_present_error::MasternodeVoteAlreadyPresentError;
use dpp::consensus::state::voting::vote_poll_not_available_for_voting_error::VotePollNotAvailableForVotingError;
use dpp::consensus::state::voting::vote_poll_not_found_error::VotePollNotFoundError;
use dpp::consensus::ConsensusError;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;

use crate::execution::validation::state_transition::masternode_vote::transform_into_action::v0::MasternodeVoteStateTransitionTransformIntoActionValidationV0;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::Vote;
use drive::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use drive::drive::votes::resolved::votes::resolved_resource_vote::accessors::v0::ResolvedResourceVoteGettersV0;
use drive::drive::votes::resolved::votes::ResolvedVote;
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
}

impl MasternodeVoteStateTransitionStateValidationV0 for MasternodeVoteTransition {
    fn validate_state_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
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
                        if let Some(existing_resource_vote_choice) =
                            maybe_existing_resource_vote_choice
                        {
                            if existing_resource_vote_choice == resource_vote.resource_vote_choice()
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
                            }
                        }
                    }
                }
            }
        }

        let result = self.transform_into_action_v0(platform, tx, platform_version)?;

        if !result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(result.errors));
        }

        let action = result.into_data()?;

        // We need to make sure that the vote poll exists and is in started state
        match action.vote_ref() {
            ResolvedVote::ResolvedResourceVote(resource_vote) => {
                let vote_poll = resource_vote.vote_poll();
                match vote_poll {
                    ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                        contested_document_resource_vote_poll,
                    ) => {
                        let Some(stored_info) = platform
                            .drive
                            .fetch_contested_document_vote_poll_stored_info(
                                contested_document_resource_vote_poll,
                                None,
                                tx,
                                platform_version,
                            )?
                            .1
                        else {
                            return Ok(ConsensusValidationResult::new_with_error(
                                ConsensusError::StateError(StateError::VotePollNotFoundError(
                                    VotePollNotFoundError::new(vote_poll.into()),
                                )),
                            ));
                        };
                        let vote_poll_status = stored_info.vote_poll_status();
                        match &vote_poll_status {
                            ContestedDocumentVotePollStatus::NotStarted
                            | ContestedDocumentVotePollStatus::Awarded(_)
                            | ContestedDocumentVotePollStatus::Locked => {
                                Ok(ConsensusValidationResult::new_with_error(
                                    ConsensusError::StateError(
                                        StateError::VotePollNotAvailableForVotingError(
                                            VotePollNotAvailableForVotingError::new(
                                                vote_poll.into(),
                                                vote_poll_status,
                                            ),
                                        ),
                                    ),
                                ))
                            }
                            ContestedDocumentVotePollStatus::Started(_) => {
                                Ok(ConsensusValidationResult::new_with_data(action.into()))
                            }
                        }
                    }
                }
            }
        }
    }
}
