use crate::consensus::basic::data_contract::DataContractUpdateEntryKind;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

/// A delta-based contract update changes or removes an entry the stored
/// contract does not have: an updated document type or schema definition
/// that was never registered, or a removed keyword that is not present.
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data Contract {data_contract_id} has no {entry_kind} '{name}' to update")]
#[platform_serialize(unversioned)]
pub struct DataContractUpdateEntryNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    entry_kind: DataContractUpdateEntryKind,
    name: String,
}

impl DataContractUpdateEntryNotFoundError {
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

impl From<DataContractUpdateEntryNotFoundError> for ConsensusError {
    fn from(err: DataContractUpdateEntryNotFoundError) -> Self {
        Self::StateError(StateError::DataContractUpdateEntryNotFoundError(err))
    }
}
