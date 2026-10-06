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

/// The approval of a moderator's deletion of a settled document that is not settled: it was last
/// modified within the window its type gives its moderators (`moderatorAbilities.deleteWithin`),
/// in which a moderator deletes it alone, with a `DeleteDocument`.
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
    "Document {} on contract {} was last modified at {} and moderators delete it alone for {} seconds after that, which block time {} is within: it is not settled",
    document_id,
    contract_id,
    last_modified_at,
    window_seconds,
    block_time
)]
#[platform_serialize(unversioned)]
pub struct DocumentNotSettledError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_id: Identifier,
    last_modified_at: TimestampMillis,
    window_seconds: u32,
    block_time: TimestampMillis,
}

impl DocumentNotSettledError {
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

    /// The moderated contract
    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    /// The document the deletion names
    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    /// The document's last modification, in milliseconds: its `$updatedAt`, or its
    /// `$createdAt` on a document type that carries no `$updatedAt`
    pub fn last_modified_at(&self) -> TimestampMillis {
        self.last_modified_at
    }

    /// For how long after it a moderator deletes the document alone
    pub fn window_seconds(&self) -> u32 {
        self.window_seconds
    }

    /// The block time the approval was judged at, in milliseconds
    pub fn block_time(&self) -> TimestampMillis {
        self.block_time
    }
}

impl From<DocumentNotSettledError> for ConsensusError {
    fn from(err: DocumentNotSettledError) -> Self {
        Self::StateError(StateError::DocumentNotSettledError(err))
    }
}
