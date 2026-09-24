use crate::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use crate::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;

/// Getters of a yes/no vote.
pub trait YesNoVoteGettersV0 {
    /// The poll answered.
    fn vote_poll(&self) -> &YesNoVotePoll;

    /// The poll answered, owned.
    fn vote_poll_owned(self) -> YesNoVotePoll;

    /// The answer.
    fn vote_choice(&self) -> YesNoAbstainVoteChoice;
}
