pub(crate) mod resolve;

use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::drive::votes::resolved::vote_polls::ResolvedVotePoll;

/// Represents the version 0 of a resolved resource vote.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedResourceVoteV0 {
    /// The resolved vote poll associated with this resource vote.
    pub resolved_vote_poll: ResolvedVotePoll,
    /// The choice made in the resource vote.
    pub resource_vote_choice: ResourceVoteChoice,
}