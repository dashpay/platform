use crate::drive::votes::PreviousVoteCount;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::votes::yes_no_vote::v0::YesNoVoteV0;
use dpp::voting::votes::yes_no_vote::YesNoVote;

/// A yes/no vote as the execution layer handles it. A yes/no poll needs no contract to be
/// resolved, so this is the vote itself (the poll's id and the answer) plus what validation
/// learned from the state: the masternode's previous answer to the same poll, which registering
/// the vote removes.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedYesNoVote {
    /// The id of the poll answered.
    pub vote_poll_id: Identifier,
    /// The answer.
    pub vote_choice: YesNoAbstainVoteChoice,
    /// The masternode's previous answer to this poll and how many times it had voted, when it
    /// changes its vote. `None` for a first vote, and whenever validation was skipped.
    pub previous_vote_choice_to_remove: Option<(YesNoAbstainVoteChoice, PreviousVoteCount)>,
}

impl From<ResolvedYesNoVote> for YesNoVote {
    fn from(value: ResolvedYesNoVote) -> Self {
        YesNoVote::V0(YesNoVoteV0 {
            vote_poll_id: value.vote_poll_id,
            vote_choice: value.vote_choice,
        })
    }
}
