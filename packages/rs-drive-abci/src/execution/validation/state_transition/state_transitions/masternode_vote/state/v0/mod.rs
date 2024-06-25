use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::voting::vote_poll_not_available_for_voting_error::VotePollNotAvailableForVotingError;
use dpp::consensus::state::voting::vote_poll_not_found_error::VotePollNotFoundError;
use dpp::consensus::ConsensusError;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;

use crate::error::execution::ExecutionError;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters,
};
use drive::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use drive::drive::votes::resolved::votes::resolved_resource_vote::accessors::v0::ResolvedResourceVoteGettersV0;
use drive::drive::votes::resolved::votes::ResolvedVote;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::masternode_vote) trait MasternodeVoteStateTransitionStateValidationV0
{
    fn validate_state_v0<C>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl MasternodeVoteStateTransitionStateValidationV0 for MasternodeVoteTransition {
    fn validate_state_v0<C>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let Some(StateTransitionAction::MasternodeVoteAction(masternode_vote_action)) = action
        else {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "action should be known in validate state for masternode vote transition",
            )));
        };

        // We need to make sure that the vote poll exists and is in started state
        match masternode_vote_action.vote_ref() {
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
                                Ok(ConsensusValidationResult::new_with_data(
                                    masternode_vote_action.into(),
                                ))
                            }
                        }
                    }
                }
            }
        }
    }
}
