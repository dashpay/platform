use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
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
    "The document brought back for {} on contract {} hashes to {}, not to the {} its removal record holds",
    document_id,
    contract_id,
    hex::encode(actual_hash),
    hex::encode(expected_hash)
)]
#[platform_serialize(unversioned)]
pub struct DocumentRestoreHashMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_id: Identifier,
    expected_hash: [u8; 32],
    actual_hash: [u8; 32],
}

impl DocumentRestoreHashMismatchError {
    pub fn new(
        contract_id: Identifier,
        document_id: Identifier,
        expected_hash: [u8; 32],
        actual_hash: [u8; 32],
    ) -> Self {
        Self {
            contract_id,
            document_id,
            expected_hash,
            actual_hash,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    /// The hash the removal record holds: of the document as it was serialized when it was
    /// removed
    pub fn expected_hash(&self) -> [u8; 32] {
        self.expected_hash
    }

    /// The hash of the document the transition brought back
    pub fn actual_hash(&self) -> [u8; 32] {
        self.actual_hash
    }
}

impl From<DocumentRestoreHashMismatchError> for ConsensusError {
    fn from(err: DocumentRestoreHashMismatchError) -> Self {
        Self::StateError(StateError::DocumentRestoreHashMismatchError(err))
    }
}
