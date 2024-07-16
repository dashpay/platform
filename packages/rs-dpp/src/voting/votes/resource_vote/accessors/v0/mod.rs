use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_polls::VotePoll;

/// Trait for getters in Resource Vote
pub trait ResourceVoteGettersV0 {
    /// The vote poll
    fn vote_poll(&self) -> &VotePoll;

    /// The vote poll as owned
    fn vote_poll_owned(self) -> VotePoll;

    /// The choice made in the vote
    fn resource_vote_choice(&self) -> ResourceVoteChoice;
}
