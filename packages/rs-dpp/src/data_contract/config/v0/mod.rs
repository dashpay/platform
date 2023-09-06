use crate::data_contract::config::DataContractConfig;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::Value;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONTRACT_KEEPS_HISTORY: bool = false;
pub const DEFAULT_ALLOW_CONTRACT_DELETION: bool = false;
pub const DEFAULT_ALLOW_CONTRACT_UPDATE: bool = true;
pub const DEFAULT_DOCUMENTS_KEEP_HISTORY: bool = false;
pub const DEFAULT_DOCUMENTS_MUTABILITY: bool = true;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct DataContractConfigV0 {
    /// Can the contract ever be deleted. If the contract is deleted, so should be all
    /// documents associated with it. TODO: There should also be a way to "stop" the contract -
    /// contract and documents are kept in the system, but no new documents can be added to it
    pub allow_contract_deletion: bool,
    /// Is the contract mutable. Means that the document definitions can be changed or new
    /// document definitions can be added to the contract
    pub allow_contract_update: bool,
    /// Does the contract keep history when the contract itself changes
    pub keeps_previous_contract_versions: bool,
    /// Do documents in the contract keep history. This is a default for all documents in
    /// the contract, but can be overridden by the document itself
    pub documents_keep_history_contract_default: bool,
    /// Are documents in the contract mutable. This specifies whether the document can be
    /// changed or deleted. This is a default for all documents in the contract, but can be
    /// overridden by the document itself
    pub documents_mutability_contract_default: bool,
    /// Encryption key storage requirements
    pub requires_identity_encryption_bounded_key: Option<StorageKeyRequirements>,
    /// Decryption key storage requirements
    pub requires_identity_decryption_bounded_key: Option<StorageKeyRequirements>,
}

/// Trait representing getters for `DataContractConfigV0`
pub trait DataContractConfigGettersV0 {
    /// Returns whether the contract can be deleted.
    fn is_contract_deletion_allowed(&self) -> bool;

    /// Returns whether the contract is read-only.
    fn is_contract_update_allowed(&self) -> bool;

    /// Returns whether the contract keeps history.
    fn keeps_previous_contract_versions(&self) -> bool;

    /// Returns whether documents in the contract keep history by default.
    fn documents_keep_history_contract_default(&self) -> bool;

    /// Returns whether documents in the contract are mutable by default.
    fn documents_mutability_contract_default(&self) -> bool;

    /// Encryption key storage requirements
    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements>;

    /// Decryption key storage requirements
    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements>;
}

/// Trait representing setters for `DataContractConfigV0`
pub trait DataContractConfigSettersV0 {
    /// Sets whether the contract can be deleted.
    fn set_allow_contract_deletion(&mut self, value: bool);

    /// Sets whether the contract is read-only.
    fn set_allow_contract_update(&mut self, value: bool);

    /// Sets whether the contract keeps history.
    fn set_keeps_previous_contract_versions(&mut self, value: bool);

    /// Sets whether documents in the contract keep history by default.
    fn set_documents_keep_history_contract_default(&mut self, value: bool);

    /// Sets whether documents in the contract are mutable by default.
    fn set_documents_mutability_contract_default(&mut self, value: bool);

    /// Sets Encryption key storage requirements.
    fn set_requires_identity_encryption_bounded_key(
        &mut self,
        value: Option<StorageKeyRequirements>,
    );

    /// Sets Decryption key storage requirements.
    fn set_requires_identity_decryption_bounded_key(
        &mut self,
        value: Option<StorageKeyRequirements>,
    );
}

impl Default for DataContractConfigV0 {
    fn default() -> Self {
        DataContractConfigV0 {
            allow_contract_deletion: DEFAULT_ALLOW_CONTRACT_DELETION,
            allow_contract_update: DEFAULT_ALLOW_CONTRACT_UPDATE,
            keeps_previous_contract_versions: DEFAULT_CONTRACT_KEEPS_HISTORY,
            documents_keep_history_contract_default: DEFAULT_DOCUMENTS_KEEP_HISTORY,
            documents_mutability_contract_default: DEFAULT_DOCUMENTS_MUTABILITY,
            requires_identity_encryption_bounded_key: None,
            requires_identity_decryption_bounded_key: None,
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
