use crate::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use crate::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use crate::voting::votes::yes_no_vote::accessors::v0::YesNoVoteGettersV0;
use crate::voting::votes::yes_no_vote::YesNoVote;

pub mod v0;

impl YesNoVoteGettersV0 for YesNoVote {
    fn vote_poll(&self) -> &YesNoVotePoll {
        match self {
            YesNoVote::V0(v0) => &v0.vote_poll,
        }
    }

    fn vote_poll_owned(self) -> YesNoVotePoll {
        match self {
            YesNoVote::V0(v0) => v0.vote_poll,
        }
    }

    fn vote_choice(&self) -> YesNoAbstainVoteChoice {
        match self {
            YesNoVote::V0(v0) => v0.vote_choice,
        }
    }
}
