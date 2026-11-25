use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::masternode_vote::state::v1::MasternodeVoteStateTransitionStateValidationV1;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::voting::vote_poll_not_available_for_voting_error::VotePollNotAvailableForVotingError;
use dpp::consensus::state::voting::vote_poll_not_found_error::VotePollNotFoundError;
use dpp::consensus::ConsensusError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfoV0Getters;
use drive::drive::votes::paths::{
    vote_contested_resource_end_date_queries_at_time_tree_path_vec, VOTE_POLLS_LEFT_OPEN_END_DATE,
};
use drive::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use drive::drive::votes::resolved::votes::resolved_resource_vote::accessors::v0::ResolvedResourceVoteGettersV0;
use drive::drive::votes::resolved::votes::ResolvedVote;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use drive::util::grove_operations::DirectQueryType;

pub(in crate::execution::validation::state_transition::state_transitions::masternode_vote) trait MasternodeVoteStateTransitionStateValidationV2
{
    fn validate_state_v2<C>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl MasternodeVoteStateTransitionStateValidationV2 for MasternodeVoteTransition {
    /// Validates a masternode vote as version 1 does, then refuses a vote on a contest its
    /// end left open (`check_for_ended_vote_polls` 2): the contest is past its end date, and
    /// no vote on it is cast or changed until a protocol version resolves it. Its record
    /// still says it is started, so it is told by the end date that lists it,
    /// [`VOTE_POLLS_LEFT_OPEN_END_DATE`]. The refusal is the one a finished contest gets,
    /// with the status the record holds.
    fn validate_state_v2<C>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let result = self.validate_state_v1(action, platform, tx, platform_version)?;
        if !result.is_valid() {
            return Ok(result);
        }
        let Some(StateTransitionAction::MasternodeVoteAction(masternode_vote_action)) =
            result.data.as_ref()
        else {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "a valid masternode vote must return its action",
            )));
        };
        let ResolvedVote::ResolvedResourceVote(resource_vote) = masternode_vote_action.vote_ref()
        else {
            return Ok(result);
        };
        let vote_poll = resource_vote.vote_poll();
        let ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
            contested_document_resource_vote_poll,
        ) = vote_poll;
        let left_open = platform.drive.grove_has_raw(
            vote_contested_resource_end_date_queries_at_time_tree_path_vec(
                VOTE_POLLS_LEFT_OPEN_END_DATE,
            )
            .as_slice()
            .into(),
            contested_document_resource_vote_poll
                .unique_id()?
                .as_slice(),
            DirectQueryType::StatefulDirectQuery,
            tx,
            &mut vec![],
            &platform_version.drive,
        )?;
        if !left_open {
            return Ok(result);
        }
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
        Ok(ConsensusValidationResult::new_with_error(
            ConsensusError::StateError(StateError::VotePollNotAvailableForVotingError(
                VotePollNotAvailableForVotingError::new(
                    vote_poll.into(),
                    stored_info.vote_poll_status(),
                ),
            )),
        ))
    }
}
