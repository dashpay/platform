use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStatus;
use crate::voting::vote_polls::VotePoll;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("VotePoll {vote_poll} not available for voting: {status}")]
#[platform_serialize(unversioned)]
pub struct VotePollNotAvailableForVotingError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    vote_poll: VotePoll,
    status: ContestedDocumentVotePollStatus,
}

impl VotePollNotAvailableForVotingError {
    pub fn new(vote_poll: VotePoll, status: ContestedDocumentVotePollStatus) -> Self {
        Self { vote_poll, status }
    }

    pub fn vote_poll(&self) -> &VotePoll {
        &self.vote_poll
    }
}

impl From<VotePollNotAvailableForVotingError> for ConsensusError {
    fn from(err: VotePollNotAvailableForVotingError) -> Self {
        Self::StateError(StateError::VotePollNotAvailableForVotingError(err))
    }
}
