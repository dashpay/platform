use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::TimestampMillis;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "Document {} on contract {} was removed at {} and could be restored by moderators for {} milliseconds after that, which block time {} is past",
    document_id,
    contract_id,
    removed_at,
    window_ms,
    block_time
)]
#[platform_serialize(unversioned)]
pub struct DocumentRestoreWindowElapsedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_id: Identifier,
    removed_at: TimestampMillis,
    window_ms: u64,
    block_time: TimestampMillis,
}

impl DocumentRestoreWindowElapsedError {
    pub fn new(
        contract_id: Identifier,
        document_id: Identifier,
        removed_at: TimestampMillis,
        window_ms: u64,
        block_time: TimestampMillis,
    ) -> Self {
        Self {
            contract_id,
            document_id,
            removed_at,
            window_ms,
            block_time,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    /// The time of the block that removed the document, in milliseconds
    pub fn removed_at(&self) -> TimestampMillis {
        self.removed_at
    }

    /// For how long after the removal, in milliseconds, moderators could restore the document
    pub fn window_ms(&self) -> u64 {
        self.window_ms
    }

    /// The block time the restore was judged at, in milliseconds
    pub fn block_time(&self) -> TimestampMillis {
        self.block_time
    }
}

impl From<DocumentRestoreWindowElapsedError> for ConsensusError {
    fn from(err: DocumentRestoreWindowElapsedError) -> Self {
        Self::StateError(StateError::DocumentRestoreWindowElapsedError(err))
    }
}
