use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;

use dpp::voting::vote_polls::VotePoll;

/// Voting based Operations
#[derive(Clone, Debug)]
pub enum VotingOperationType {
    /// Adds a vote poll to the state.
    AddVotePoll {
        /// The creator of the vote poll
        creator_identity_id: Option<Identifier>,
        /// The vote poll
        vote_poll: VotePoll,
        /// The end date of the vote poll
        end_date: TimestampMillis,
    },
}
