use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A document write to a data contract would take one of the sums its document types keep (an
/// index `summable` by a property, a `rangeSummable` index, or `documentsSummable`) out of the
/// range a sum holds, a signed 64 bit integer. The write is refused as a paid error from protocol
/// version 15: the identity contract nonce is bumped and nothing else of the write is stored.
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
#[error("a document write to contract {contract_id} would take a document sum out of the signed 64-bit range a sum holds")]
#[platform_serialize(unversioned)]
pub struct DocumentSumOverflowError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
}

impl DocumentSumOverflowError {
    pub fn new(contract_id: Identifier) -> Self {
        Self { contract_id }
    }

    pub fn contract_id(&self) -> &Identifier {
        &self.contract_id
    }
}

impl From<DocumentSumOverflowError> for ConsensusError {
    fn from(err: DocumentSumOverflowError) -> Self {
        Self::StateError(StateError::DocumentSumOverflowError(err))
    }
}
