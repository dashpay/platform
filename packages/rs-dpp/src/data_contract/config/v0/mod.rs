use crate::data_contract::config;
use crate::data_contract::config::DataContractConfig;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_CONTRACT_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_CAN_BE_DELETED: bool = false;
pub const DEFAULT_CONTRACT_MUTABILITY: bool = true;
pub const DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_DOCUMENT_MUTABILITY: bool = true;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct DataContractConfigV0 {
    /// Can the contract ever be deleted. If the contract is deleted, so should be all
    /// documents associated with it. TODO: There should also be a way to "stop" the contract -
    /// contract and documents are kept in the system, but no new documents can be added to it
    pub allow_contract_deletion: bool,
    /// Is the contract mutable. Means that the document definitions can be changed or new
    /// document definitions can be added to the contract
    pub allowe_contract_update: bool,
    /// Does the contract keep history when the contract itself changes
    pub keep_previous_contract_versions: bool,
    /// Do documents in the contract keep history. This is a default for all documents in
    /// the contract, but can be overridden by the document itself
    pub document_revisions_default: bool,
    /// Are documents in the contract mutable. This specifies whether the document can be
    /// changed or deleted. This is a default for all documents in the contract, but can be
    /// overridden by the document itself
    pub documents_read_only_default: bool,
}

/// Trait representing getters for `DataContractConfigV0`
pub trait DataContractConfigGettersV0 {
    /// Returns whether the contract can be deleted.
    fn is_contract_deletion_allowed(&self) -> bool;

    /// Returns whether the contract is read-only.
    fn is_contract_update_allowed(&self) -> bool;

    /// Returns whether the contract keeps history.
    fn keep_previous_contract_versions(&self) -> bool;

    /// Returns whether documents in the contract keep history by default.
    fn document_revisions(&self) -> bool;

    /// Returns whether documents in the contract are mutable by default.
    fn documents_read_only(&self) -> bool;
}

/// Trait representing setters for `DataContractConfigV0`
pub trait DataContractConfigSettersV0 {
    /// Sets whether the contract can be deleted.
    fn set_allow_contract_deletion(&mut self, value: bool);

    /// Sets whether the contract is read-only.
    fn set_allow_contract_update(&mut self, value: bool);

    /// Sets whether the contract keeps history.
    fn set_keep_previous_contract_versions(&mut self, value: bool);

    /// Sets whether documents in the contract keep history by default.
    fn set_document_revisions_default(&mut self, value: bool);

    /// Sets whether documents in the contract are mutable by default.
    fn set_documents_read_only_default(&mut self, value: bool);
}

impl std::default::Default for DataContractConfigV0 {
    fn default() -> Self {
        DataContractConfigV0 {
            allow_contract_deletion: DEFAULT_CONTRACT_CAN_BE_DELETED,
            allowe_contract_update: !DEFAULT_CONTRACT_MUTABILITY,
            keep_previous_contract_versions: DEFAULT_CONTRACT_KEEPS_HISTORY,
            document_revisions_default: DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
            documents_read_only_default: DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
        }
    }
}

impl DataContractConfigV0 {
    pub fn from_value(value: Value) -> Result<Self, ProtocolError> {
        platform_value::from_value(value).map_err(ProtocolError::ValueError)
    }

    pub fn default_with_version() -> DataContractConfig {
        Self::default().into()
    }
}
