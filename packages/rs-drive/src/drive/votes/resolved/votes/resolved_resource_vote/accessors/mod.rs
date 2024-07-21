use crate::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use crate::drive::votes::resolved::votes::resolved_resource_vote::accessors::v0::ResolvedResourceVoteGettersV0;
use crate::drive::votes::resolved::votes::resolved_resource_vote::ResolvedResourceVote;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;

/// Module containing version 0 of the implementation.
pub mod v0;

impl ResolvedResourceVoteGettersV0 for ResolvedResourceVote {
    fn vote_poll(&self) -> &ResolvedVotePoll {
        match self {
            ResolvedResourceVote::V0(v0) => &v0.resolved_vote_poll,
        }
    }

    fn vote_poll_owned(self) -> ResolvedVotePoll {
        match self {
            ResolvedResourceVote::V0(v0) => v0.resolved_vote_poll,
        }
    }

    fn resource_vote_choice(&self) -> ResourceVoteChoice {
        match self {
            ResolvedResourceVote::V0(v0) => v0.resource_vote_choice,
        }
    }
}
