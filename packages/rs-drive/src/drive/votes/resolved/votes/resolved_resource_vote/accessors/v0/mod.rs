use crate::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;

/// Trait for getters in Resource Vote
pub trait ResolvedResourceVoteGettersV0 {
    /// The vote poll
    fn vote_poll(&self) -> &ResolvedVotePoll;

    /// The vote poll as owned
    fn vote_poll_owned(self) -> ResolvedVotePoll;

    /// The choice made in the vote
    fn resource_vote_choice(&self) -> ResourceVoteChoice;
}
