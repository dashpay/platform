use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_polls::VotePoll;
use crate::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use crate::voting::votes::resource_vote::ResourceVote;

pub mod v0;

impl ResourceVoteGettersV0 for ResourceVote {
    fn vote_poll(&self) -> &VotePoll {
        match self {
            ResourceVote::V0(v0) => &v0.vote_poll,
        }
    }

    fn vote_poll_owned(self) -> VotePoll {
        match self {
            ResourceVote::V0(v0) => v0.vote_poll,
        }
    }

    fn resource_vote_choice(&self) -> ResourceVoteChoice {
        match self {
            ResourceVote::V0(v0) => v0.resource_vote_choice,
        }
    }
}
