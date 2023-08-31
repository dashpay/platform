use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document {document_id} {timestamp_name} timestamp {timestamp} are out of block time window from {time_window_start} and {time_window_end}")]
#[platform_serialize(unversioned)]
pub struct DocumentTimestampWindowViolationError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    timestamp_name: String,
    document_id: Identifier,
    timestamp: i64,
    time_window_start: i64,
    time_window_end: i64,
}

impl DocumentTimestampWindowViolationError {
    pub fn new(
        timestamp_name: String,
        document_id: Identifier,
        timestamp: i64,
        time_window_start: i64,
        time_window_end: i64,
    ) -> Self {
        Self {
            timestamp_name,
            document_id,
            timestamp,
            time_window_start,
            time_window_end,
        }
    }

    pub fn timestamp_name(&self) -> String {
        self.timestamp_name.clone()
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    pub fn timestamp(&self) -> &i64 {
        &self.timestamp
    }

    pub fn time_window_start(&self) -> &i64 {
        &self.time_window_start
    }

    pub fn time_window_end(&self) -> &i64 {
        &self.time_window_end
    }
}

impl From<DocumentTimestampWindowViolationError> for ConsensusError {
    fn from(err: DocumentTimestampWindowViolationError) -> Self {
        Self::StateError(StateError::DocumentTimestampWindowViolationError(err))
    }
}
