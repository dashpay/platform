use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;

/// Trait for getters in Contested Document Resource Vote
pub trait ContestedDocumentResourceVoteGettersV0 {
    /// The vote poll
    fn vote_poll(&self) -> &ContestedDocumentResourceVotePoll;

    /// The vote poll as owned
    fn vote_poll_owned(self) -> ContestedDocumentResourceVotePoll;

    /// The choice made in the vote
    fn resource_vote_choice(&self) -> ResourceVoteChoice;
}
