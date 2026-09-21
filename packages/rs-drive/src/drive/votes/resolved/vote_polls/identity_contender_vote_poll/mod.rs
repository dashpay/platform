use crate::drive::votes::paths::{
    vote_identity_contender_poll_choice_tree_path_vec,
    vote_identity_contender_poll_choice_votes_path_vec, vote_identity_contender_poll_tree_path_vec,
};
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::voting::contender_structs::IdentityContenderInfo;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use std::collections::BTreeMap;

/// The paths of an identity contender vote poll in the votes tree. The poll needs nothing from
/// state to resolve: its unique id keys everything.
pub trait IdentityContenderVotePollPaths {
    /// The tree of the poll: its stored info, its abstain votes and its contenders
    fn poll_path_vec(&self) -> Result<Vec<Vec<u8>>, Error>;
    /// The tree of one choice: a contender's identity id or the abstain key
    fn choice_path_vec(&self, vote_choice: &ResourceVoteChoice) -> Result<Vec<Vec<u8>>, Error>;
    /// The sum tree holding the votes for one choice
    fn choice_votes_path_vec(
        &self,
        vote_choice: &ResourceVoteChoice,
    ) -> Result<Vec<Vec<u8>>, Error>;
}

impl IdentityContenderVotePollPaths for IdentityContenderVotePoll {
    fn poll_path_vec(&self) -> Result<Vec<Vec<u8>>, Error> {
        Ok(vote_identity_contender_poll_tree_path_vec(
            self.unique_id()?.as_slice(),
        ))
    }

    fn choice_path_vec(&self, vote_choice: &ResourceVoteChoice) -> Result<Vec<Vec<u8>>, Error> {
        Ok(vote_identity_contender_poll_choice_tree_path_vec(
            self.unique_id()?.as_slice(),
            vote_choice,
        ))
    }

    fn choice_votes_path_vec(
        &self,
        vote_choice: &ResourceVoteChoice,
    ) -> Result<Vec<Vec<u8>>, Error> {
        Ok(vote_identity_contender_poll_choice_votes_path_vec(
            self.unique_id()?.as_slice(),
            vote_choice,
        ))
    }
}

/// One contender of an identity contender vote poll as read from the votes tree, with the tally
/// of the votes towards it when the read asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityContenderWithTally {
    /// The contender's identity
    pub identity_id: Identifier,
    /// When the identity joined and through what
    pub info: IdentityContenderInfo,
    /// The sum of the strengths of the votes towards the contender
    pub vote_tally: Option<u32>,
}

/// What ending a phase of an identity contender vote poll produced, which the clean-up that
/// follows acts on.
#[derive(Debug, Clone, PartialEq)]
pub enum IdentityContenderVotePollEndOutcome {
    /// The join phase ended with more than one contender: the vote phase runs until the poll's
    /// vote end time, where a new end date entry now waits. The join end date entry is what is
    /// left to remove.
    VotePhaseStarted,
    /// The poll resolved, at the end of its join phase with at most one contender or at the end
    /// of its vote phase. The contenders, the votes and the poll's end date entry are removed;
    /// the stored info keeps the result.
    Resolved {
        /// The winner, if the poll had a contender
        winner: Option<Identifier>,
        /// Who voted for each choice, including contenders nobody voted for, so that every vote
        /// reference the masternodes hold is removed
        votes: BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
    },
}
