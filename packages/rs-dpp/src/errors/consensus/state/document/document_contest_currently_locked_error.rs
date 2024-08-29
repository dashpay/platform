use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document Contest for vote_poll {vote_poll} is currently already locked {stored_info}, unlocking is possible by paying {unlock_cost} credits")]
#[platform_serialize(unversioned)]
pub struct DocumentContestCurrentlyLockedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    vote_poll: ContestedDocumentResourceVotePoll,
    stored_info: ContestedDocumentVotePollStoredInfo,
    unlock_cost: u64,
}

impl DocumentContestCurrentlyLockedError {
    pub fn new(
        vote_poll: ContestedDocumentResourceVotePoll,
        stored_info: ContestedDocumentVotePollStoredInfo,
        unlock_cost: u64,
    ) -> Self {
        Self {
            vote_poll,
            stored_info,
            unlock_cost,
        }
    }

    pub fn vote_poll(&self) -> &ContestedDocumentResourceVotePoll {
        &self.vote_poll
    }
    pub fn stored_info(&self) -> &ContestedDocumentVotePollStoredInfo {
        &self.stored_info
    }

    pub fn unlock_cost(&self) -> u64 {
        self.unlock_cost
    }
}

impl From<DocumentContestCurrentlyLockedError> for ConsensusError {
    fn from(err: DocumentContestCurrentlyLockedError) -> Self {
        Self::StateError(StateError::DocumentContestCurrentlyLockedError(err))
    }
}
