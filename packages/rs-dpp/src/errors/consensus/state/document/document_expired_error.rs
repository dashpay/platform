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

/// A document whose type declares a `ttl` has expired (`$createdAt` plus the time to live is
/// at or before block time), so it can no longer be replaced, transferred, bought, repriced
/// or restored by a moderator. It still exists until the platform's cleanup deletes it after
/// a block's state transitions; its owner may still delete it. Protocol version 14.
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
    "Document {} of type \"{}\" on contract {} expired at {}, its $createdAt plus the type's time to live, which block time {} is not before",
    document_id,
    document_type_name,
    contract_id,
    expired_at,
    block_time
)]
#[platform_serialize(unversioned)]
pub struct DocumentExpiredError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    document_id: Identifier,
    expired_at: TimestampMillis,
    block_time: TimestampMillis,
}

impl DocumentExpiredError {
    pub fn new(
        contract_id: Identifier,
        document_type_name: String,
        document_id: Identifier,
        expired_at: TimestampMillis,
        block_time: TimestampMillis,
    ) -> Self {
        Self {
            contract_id,
            document_type_name,
            document_id,
            expired_at,
            block_time,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn document_type_name(&self) -> &String {
        &self.document_type_name
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    /// When the document expired, in milliseconds: its `$createdAt` plus the type's `ttl`
    pub fn expired_at(&self) -> TimestampMillis {
        self.expired_at
    }

    /// The block time the action was judged at, in milliseconds
    pub fn block_time(&self) -> TimestampMillis {
        self.block_time
    }
}

impl From<DocumentExpiredError> for ConsensusError {
    fn from(err: DocumentExpiredError) -> Self {
        Self::StateError(StateError::DocumentExpiredError(err))
    }
}
