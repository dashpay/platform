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
    "Document {} on contract {} was last modified at {} and could be deleted by moderators for {} seconds after that, which block time {} is past",
    document_id,
    contract_id,
    last_modified_at,
    window_seconds,
    block_time
)]
#[platform_serialize(unversioned)]
pub struct DocumentModerationWindowElapsedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_id: Identifier,
    last_modified_at: TimestampMillis,
    window_seconds: u32,
    block_time: TimestampMillis,
}

impl DocumentModerationWindowElapsedError {
    pub fn new(
        contract_id: Identifier,
        document_id: Identifier,
        last_modified_at: TimestampMillis,
        window_seconds: u32,
        block_time: TimestampMillis,
    ) -> Self {
        Self {
            contract_id,
            document_id,
            last_modified_at,
            window_seconds,
            block_time,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    /// The document's last modification, in milliseconds: its `$updatedAt`, or its
    /// `$createdAt` on a document type that carries no `$updatedAt`
    pub fn last_modified_at(&self) -> TimestampMillis {
        self.last_modified_at
    }

    /// For how long after it moderators could delete the document
    pub fn window_seconds(&self) -> u32 {
        self.window_seconds
    }

    /// The block time the deletion was judged at, in milliseconds
    pub fn block_time(&self) -> TimestampMillis {
        self.block_time
    }
}

impl From<DocumentModerationWindowElapsedError> for ConsensusError {
    fn from(err: DocumentModerationWindowElapsedError) -> Self {
        Self::StateError(StateError::DocumentModerationWindowElapsedError(err))
    }
}
