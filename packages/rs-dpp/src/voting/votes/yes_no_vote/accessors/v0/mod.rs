use crate::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use platform_value::Identifier;

/// Getters of a yes/no vote.
pub trait YesNoVoteGettersV0 {
    /// The id of the poll answered.
    fn vote_poll_id(&self) -> Identifier;

    /// The answer.
    fn vote_choice(&self) -> YesNoAbstainVoteChoice;
}
