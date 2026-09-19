use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use std::fmt;
use thiserror::Error;

/// The kind of contract entry a delta-based contract update names.
///
/// A V1 data contract update transition carries additive and in-place
/// changes per entry kind rather than a whole contract, so validation
/// errors report which section of the delta a name came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub enum DataContractUpdateEntryKind {
    /// A document type, keyed by its name.
    DocumentType,
    /// A shared `$defs` sub-schema, keyed by its definition name.
    SchemaDef,
    /// A group, keyed by its contract position.
    Group,
    /// A token, keyed by its contract position.
    Token,
    /// A search keyword.
    Keyword,
}

impl fmt::Display for DataContractUpdateEntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DataContractUpdateEntryKind::DocumentType => "document type",
            DataContractUpdateEntryKind::SchemaDef => "schema definition",
            DataContractUpdateEntryKind::Group => "group",
            DataContractUpdateEntryKind::Token => "token",
            DataContractUpdateEntryKind::Keyword => "keyword",
        };
        f.write_str(name)
    }
}

/// A delta-based contract update names the same entry in two sections
/// that cannot both apply: a document type or schema definition listed
/// as both new and updated, or a keyword listed as both added and removed.
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
#[error(
    "Data Contract {data_contract_id} update names {entry_kind} '{name}' in two conflicting sections"
)]
#[platform_serialize(unversioned)]
pub struct DataContractUpdateOverlappingEntriesError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    entry_kind: DataContractUpdateEntryKind,
    name: String,
}

impl DataContractUpdateOverlappingEntriesError {
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

impl From<DataContractUpdateOverlappingEntriesError> for ConsensusError {
    fn from(err: DataContractUpdateOverlappingEntriesError) -> Self {
        Self::BasicError(BasicError::DataContractUpdateOverlappingEntriesError(err))
    }
}
