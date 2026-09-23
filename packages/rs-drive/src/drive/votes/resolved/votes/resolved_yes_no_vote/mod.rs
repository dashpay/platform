use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use dpp::voting::votes::yes_no_vote::accessors::v0::YesNoVoteGettersV0;
use dpp::voting::votes::yes_no_vote::v0::YesNoVoteV0;
use dpp::voting::votes::yes_no_vote::YesNoVote;

/// How many times a masternode already voted on a poll, before the vote being cast.
pub type PreviousYesNoVoteCount = u16;

/// A yes/no vote as the execution layer handles it. A yes/no poll needs no contract to be
/// resolved, so this is the vote itself plus what validation learned from the state: the
/// masternode's previous answer to the same poll, which registering the vote removes.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedYesNoVote {
    /// The poll answered.
    pub vote_poll: YesNoVotePoll,
    /// The answer.
    pub vote_choice: YesNoAbstainVoteChoice,
    /// The masternode's previous answer to this poll and how many times it had voted, when it
    /// changes its vote. `None` for a first vote, and whenever validation was skipped.
    pub previous_vote_choice_to_remove: Option<(YesNoAbstainVoteChoice, PreviousYesNoVoteCount)>,
}

impl ResolvedYesNoVote {
    /// The vote as it came in, with no previous answer attached.
    pub fn from_vote(vote: &YesNoVote) -> Self {
        ResolvedYesNoVote {
            vote_poll: vote.vote_poll().clone(),
            vote_choice: vote.vote_choice(),
            previous_vote_choice_to_remove: None,
        }
    }

    /// The vote as it came in, owned, with no previous answer attached.
    pub fn from_vote_owned(vote: YesNoVote) -> Self {
        let vote_choice = vote.vote_choice();
        ResolvedYesNoVote {
            vote_poll: vote.vote_poll_owned(),
            vote_choice,
            previous_vote_choice_to_remove: None,
        }
    }
}

impl From<ResolvedYesNoVote> for YesNoVote {
    fn from(value: ResolvedYesNoVote) -> Self {
        YesNoVote::V0(YesNoVoteV0 {
            vote_poll: value.vote_poll,
            vote_choice: value.vote_choice,
        })
    }
}
