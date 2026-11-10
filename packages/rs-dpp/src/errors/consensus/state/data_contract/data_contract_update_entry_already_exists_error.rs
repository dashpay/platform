use crate::consensus::basic::data_contract::DataContractUpdateEntryKind;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A delta-based contract update adds an entry under a name or position the
/// stored contract already uses. Existing document types and schema
/// definitions change through the update sections, and existing groups,
/// tokens and keywords cannot be replaced at all.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeUntrusted,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
)]
#[error("Data Contract {data_contract_id} already has {entry_kind} '{name}', it can not be added")]
#[platform_serialize(unversioned)]
pub struct DataContractUpdateEntryAlreadyExistsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    entry_kind: DataContractUpdateEntryKind,
    name: String,
}

impl DataContractUpdateEntryAlreadyExistsError {
    pub fn new(
        data_contract_id: Identifier,
        entry_kind: DataContractUpdateEntryKind,
        name: String,
    ) -> Self {
        Self {
            data_contract_id,
            entry_kind,
            name,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }

    pub fn entry_kind(&self) -> DataContractUpdateEntryKind {
        self.entry_kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl From<DataContractUpdateEntryAlreadyExistsError> for ConsensusError {
    fn from(err: DataContractUpdateEntryAlreadyExistsError) -> Self {
        Self::StateError(StateError::DataContractUpdateEntryAlreadyExistsError(err))
    }
}
