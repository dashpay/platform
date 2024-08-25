use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::prelude::TimestampMillis;
use crate::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document Contest for vote_poll {vote_poll} is not joinable {stored_info}, it started {start_time} and it is now {current_time}, and you can only join for {joinable_time}")]
#[platform_serialize(unversioned)]
pub struct DocumentContestNotJoinableError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    vote_poll: ContestedDocumentResourceVotePoll,
    stored_info: ContestedDocumentVotePollStoredInfo,
    start_time: TimestampMillis,
    current_time: TimestampMillis,
    joinable_time: TimestampMillis,
}

impl DocumentContestNotJoinableError {
    pub fn new(
        vote_poll: ContestedDocumentResourceVotePoll,
        stored_info: ContestedDocumentVotePollStoredInfo,
        start_time: TimestampMillis,
        current_time: TimestampMillis,
        joinable_time: TimestampMillis,
    ) -> Self {
        Self {
            vote_poll,
            stored_info,
            start_time,
            current_time,
            joinable_time,
        }
    }

    pub fn vote_poll(&self) -> &ContestedDocumentResourceVotePoll {
        &self.vote_poll
    }
    pub fn stored_info(&self) -> &ContestedDocumentVotePollStoredInfo {
        &self.stored_info
    }

    pub fn start_time(&self) -> TimestampMillis {
        self.start_time
    }

    pub fn current_time(&self) -> TimestampMillis {
        self.current_time
    }

    pub fn joinable_time(&self) -> TimestampMillis {
        self.joinable_time
    }
}

impl From<DocumentContestNotJoinableError> for ConsensusError {
    fn from(err: DocumentContestNotJoinableError) -> Self {
        Self::StateError(StateError::DocumentContestNotJoinableError(err))
    }
}
