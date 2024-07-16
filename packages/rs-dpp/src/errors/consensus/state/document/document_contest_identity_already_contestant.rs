use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "An Identity with the id {identity_id} is already a contestant for the vote_poll {vote_poll}"
)]
#[platform_serialize(unversioned)]
pub struct DocumentContestIdentityAlreadyContestantError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    vote_poll: ContestedDocumentResourceVotePoll,
    identity_id: Identifier,
}

impl DocumentContestIdentityAlreadyContestantError {
    pub fn new(vote_poll: ContestedDocumentResourceVotePoll, identity_id: Identifier) -> Self {
        Self {
            vote_poll,
            identity_id,
        }
    }

    pub fn vote_poll(&self) -> &ContestedDocumentResourceVotePoll {
        &self.vote_poll
    }
    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}

impl From<DocumentContestIdentityAlreadyContestantError> for ConsensusError {
    fn from(err: DocumentContestIdentityAlreadyContestantError) -> Self {
        Self::StateError(StateError::DocumentContestIdentityAlreadyContestantError(
            err,
        ))
    }
}
