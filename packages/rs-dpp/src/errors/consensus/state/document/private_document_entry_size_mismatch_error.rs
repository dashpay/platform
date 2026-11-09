use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The entries of a `PrivateDocumentCreateFromShieldedPool` do not take the `entrySize` the
/// private document type declares (protocol version 15).
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
    "Entries of private document type \"{document_type_name}\" of data contract \
     {data_contract_id} take {entry_size} bytes, but this transition's take {size}"
)]
#[platform_serialize(unversioned)]
pub struct PrivateDocumentEntrySizeMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    document_type_name: String,
    entry_size: u32,
    size: u32,
}

impl PrivateDocumentEntrySizeMismatchError {
    pub fn new(
        data_contract_id: Identifier,
        document_type_name: String,
        entry_size: u32,
        size: u32,
    ) -> Self {
        Self {
            data_contract_id,
            document_type_name,
            entry_size,
            size,
        }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn entry_size(&self) -> u32 {
        self.entry_size
    }

    pub fn size(&self) -> u32 {
        self.size
    }
}

impl From<PrivateDocumentEntrySizeMismatchError> for ConsensusError {
    fn from(err: PrivateDocumentEntrySizeMismatchError) -> Self {
        Self::StateError(StateError::PrivateDocumentEntrySizeMismatchError(err))
    }
}
