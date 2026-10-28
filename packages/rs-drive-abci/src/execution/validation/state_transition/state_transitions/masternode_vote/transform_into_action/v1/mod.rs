use crate::error::Error;
use crate::execution::platform_events::voting::masternode_vote_strength;
use crate::execution::validation::state_transition::state_transitions::masternode_vote::transform_into_action::v0::{
    previous_vote_to_replace_v0, voting_masternode_v0,
    MasternodeVoteStateTransitionTransformIntoActionValidationV0,
};
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::voting::vote_poll_not_found_error::VotePollNotFoundError;
use dpp::consensus::ConsensusError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use drive::drive::votes::resolved::votes::resolved_yes_no_vote::ResolvedYesNoVote;
use drive::drive::votes::resolved::votes::ResolvedVote;
use drive::state_transition_action::identity::masternode_vote::v0::MasternodeVoteTransitionActionV0;
use drive::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;

use crate::execution::validation::state_transition::ValidationMode;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::yes_no_vote::accessors::v0::YesNoVoteGettersV0;
use dpp::voting::votes::Vote;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition::state_transitions::masternode_vote) trait MasternodeVoteStateTransitionTransformIntoActionValidationV1
{
    fn transform_into_action_v1<C>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<MasternodeVoteTransitionAction>, Error>;
}

impl MasternodeVoteStateTransitionTransformIntoActionValidationV1 for MasternodeVoteTransition {
    /// Version 1 (protocol version 15) also transforms a vote on a yes/no poll, with the rule
    /// of a resource vote ([`previous_vote_to_replace_v0`]): the same answer cannot be given twice,
    /// and a masternode may vote on a poll at most `votes_allowed_per_masternode` times, its
    /// first vote included. A resource vote on a contested poll transforms through version 0,
    /// unchanged.
    fn transform_into_action_v1<C>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<MasternodeVoteTransitionAction>, Error> {
        let yes_no_vote = match self.vote() {
            Vote::YesNoVote(yes_no_vote) => yes_no_vote,
            Vote::ResourceVote(resource_vote) => {
                // A resource vote choice (towards an identity, abstain, lock) answers a
                // contested document resource poll. No such poll is named by a resource vote
                // that carries a yes/no poll, whatever validation mode we are in.
                if let VotePoll::YesNoVotePoll(_) = resource_vote.vote_poll() {
                    return Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::VotePollNotFoundError(
                            VotePollNotFoundError::new(resource_vote.vote_poll().clone()),
                        )),
                    ));
                }
                return self.transform_into_action_v0(
                    platform,
                    validation_mode,
                    tx,
                    platform_version,
                );
            }
        };

        let mut previous_vote_choice_to_remove = None;
        if validation_mode != ValidationMode::NoValidation {
            // Before we transform into action we want to make sure that we have not yet voted
            let vote_poll = yes_no_vote.vote_poll();
            let vote_id = vote_poll.unique_id(platform_version)?;
            let maybe_existing_vote_choice = platform.drive.fetch_identity_yes_no_vote(
                self.pro_tx_hash(),
                vote_id,
                tx,
                &mut vec![],
                platform_version,
            )?;
            match previous_vote_to_replace_v0(
                self.pro_tx_hash(),
                maybe_existing_vote_choice,
                &yes_no_vote.vote_choice(),
                || VotePoll::YesNoVotePoll(vote_poll.clone()),
                platform_version,
            ) {
                Ok(previous_vote) => previous_vote_choice_to_remove = previous_vote,
                Err(error) => return Ok(ConsensusValidationResult::new_with_error(error)),
            }
        }

        let masternode = match voting_masternode_v0(platform, self.pro_tx_hash()) {
            Ok(masternode) => masternode,
            Err(error) => return Ok(ConsensusValidationResult::new_with_error(error)),
        };

        // A yes/no vote needs nothing from the state to be resolved; the previous answer found
        // above rides with it.
        let resolved_vote = ResolvedYesNoVote {
            vote_poll: yes_no_vote.vote_poll().clone(),
            vote_choice: yes_no_vote.vote_choice(),
            previous_vote_choice_to_remove,
        };
        let action = MasternodeVoteTransitionActionV0 {
            pro_tx_hash: self.pro_tx_hash(),
            voter_identity_id: self.voter_identity_id(),
            voting_address: masternode.state.voting_address,
            vote_strength: masternode_vote_strength(&masternode.node_type, platform_version),
            vote: ResolvedVote::YesNoVote(resolved_vote),
            previous_resource_vote_choice_to_remove: None,
            nonce: self.nonce(),
        }
        .into();

        Ok(ConsensusValidationResult::new_with_data(action))
    }
}
