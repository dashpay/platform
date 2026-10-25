use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::voting::masternode_not_found_error::MasternodeNotFoundError;
use dpp::consensus::state::voting::masternode_vote_already_present_error::MasternodeVoteAlreadyPresentError;
use dpp::consensus::state::voting::masternode_voted_too_many_times::MasternodeVotedTooManyTimesError;
use dpp::consensus::ConsensusError;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::ProTxHash;
use dpp::dashcore_rpc::dashcore_rpc_json::{MasternodeListItem, MasternodeType};
use dpp::identifier::Identifier;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use drive::drive::votes::PreviousVoteCount;
use drive::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;

use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::PlatformStateV0Methods;
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
                            match previous_vote_to_replace(
                                self.pro_tx_hash(),
                                maybe_existing_resource_vote_choice,
                                &resource_vote.resource_vote_choice(),
                                || resource_vote.vote_poll().clone(),
                                platform_version,
                            ) {
                                Ok(previous_vote) => {
                                    previous_resource_vote_choice_to_remove = previous_vote
                                }
                                Err(error) => {
                                    return Ok(ConsensusValidationResult::new_with_error(error))
                                }
                            }
                        }
                        // Unreachable: this v0 is selected up to protocol version 14, where a
                        // masternode vote that names a yes/no poll does not decode
                        // (`StateTransition::active_version_range`).
                        VotePoll::YesNoVotePoll(_) => {
                            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                                "yes/no vote polls need transform_into_action version 1",
                            )))
                        }
                    }
                }
                // Unreachable for the same reason: no yes/no vote decodes before 15.
                Vote::YesNoVote(_) => {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "yes/no votes need transform_into_action version 1",
                    )))
                }
            }
        }

        let masternode = match voting_masternode(platform, self.pro_tx_hash()) {
            Ok(masternode) => masternode,
            Err(error) => return Ok(ConsensusValidationResult::new_with_error(error)),
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

/// What a masternode's earlier vote on a poll means for its new one, the same for both poll
/// kinds: the same answer again is refused, a change once the masternode has voted
/// `votes_allowed_per_masternode` times is refused, and any other change carries the earlier
/// answer for registration to remove. `None` for a first vote.
///
/// Moved here unchanged from `transform_into_action_v0` (in place: the same checks, in the same
/// order, with the same errors at every protocol version that selects it), so version 1 applies
/// the rule to yes/no votes too.
pub(super) fn previous_vote_to_replace<Choice: PartialEq>(
    pro_tx_hash: Identifier,
    existing_vote: Option<(Choice, PreviousVoteCount)>,
    new_vote_choice: &Choice,
    vote_poll: impl FnOnce() -> VotePoll,
    platform_version: &PlatformVersion,
) -> Result<Option<(Choice, PreviousVoteCount)>, ConsensusError> {
    let Some((existing_vote_choice, previous_vote_count)) = existing_vote else {
        return Ok(None);
    };
    if &existing_vote_choice == new_vote_choice {
        // We are submitting a vote for something we already have
        Err(ConsensusError::StateError(
            StateError::MasternodeVoteAlreadyPresentError(MasternodeVoteAlreadyPresentError::new(
                pro_tx_hash,
                vote_poll(),
            )),
        ))
    } else if previous_vote_count
        >= platform_version
            .dpp
            .validation
            .voting
            .votes_allowed_per_masternode
    {
        // We are submitting a vote for something we already have
        Err(ConsensusError::StateError(
            StateError::MasternodeVotedTooManyTimesError(MasternodeVotedTooManyTimesError::new(
                pro_tx_hash,
                previous_vote_count,
                platform_version
                    .dpp
                    .validation
                    .voting
                    .votes_allowed_per_masternode,
            )),
        ))
    } else {
        Ok(Some((existing_vote_choice, previous_vote_count)))
    }
}

/// The voting masternode in the block's masternode list, or the error refusing a vote from one
/// that is not in it. Moved here unchanged from `transform_into_action_v0` (in place, as
/// [`previous_vote_to_replace`]) for version 1.
pub(super) fn voting_masternode<'a, C>(
    platform: &PlatformRef<'a, C>,
    pro_tx_hash: Identifier,
) -> Result<&'a MasternodeListItem, ConsensusError> {
    platform
        .state
        .full_masternode_list()
        .get(&ProTxHash::from_byte_array(pro_tx_hash.to_buffer()))
        .ok_or_else(|| MasternodeNotFoundError::new(pro_tx_hash).into())
}
