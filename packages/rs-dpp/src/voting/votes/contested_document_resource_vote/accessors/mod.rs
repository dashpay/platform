use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use crate::voting::votes::contested_document_resource_vote::accessors::v0::ContestedDocumentResourceVoteGettersV0;
use crate::voting::votes::contested_document_resource_vote::ContestedDocumentResourceVote;

pub mod v0;

impl ContestedDocumentResourceVoteGettersV0 for ContestedDocumentResourceVote {
    fn vote_poll(&self) -> &ContestedDocumentResourceVotePoll {
        match self {
            ContestedDocumentResourceVote::V0(v0) => &v0.vote_poll,
        }
    }

    fn vote_poll_owned(self) -> ContestedDocumentResourceVotePoll {
        match self {
            ContestedDocumentResourceVote::V0(v0) => v0.vote_poll,
        }
    }

    fn resource_vote_choice(&self) -> ResourceVoteChoice {
        match self {
            ContestedDocumentResourceVote::V0(v0) => v0.resource_vote_choice,
        }
    }
}
