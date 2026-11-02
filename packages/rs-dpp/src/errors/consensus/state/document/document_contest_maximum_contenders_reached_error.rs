use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A document would add a contender to a contest that already holds the most contenders a
/// contest accepts (protocol version 14).
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "The vote poll {vote_poll} already has {max_contenders} contenders, the most a contest accepts"
)]
#[platform_serialize(unversioned)]
pub struct DocumentContestMaximumContendersReachedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    vote_poll: ContestedDocumentResourceVotePoll,
    max_contenders: u16,
}

impl DocumentContestMaximumContendersReachedError {
    pub fn new(vote_poll: ContestedDocumentResourceVotePoll, max_contenders: u16) -> Self {
        Self {
            vote_poll,
            max_contenders,
        }
    }

    pub fn vote_poll(&self) -> &ContestedDocumentResourceVotePoll {
        &self.vote_poll
    }

    pub fn max_contenders(&self) -> u16 {
        self.max_contenders
    }
}

impl From<DocumentContestMaximumContendersReachedError> for ConsensusError {
    fn from(err: DocumentContestMaximumContendersReachedError) -> Self {
        Self::StateError(StateError::DocumentContestMaximumContendersReachedError(
            err,
        ))
    }
}
