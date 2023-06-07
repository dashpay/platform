use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONTRACT_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_CAN_BE_DELETED: bool = false;
pub const DEFAULT_CONTRACT_MUTABILITY: bool = true;
pub const DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_DOCUMENT_MUTABILITY: bool = true;

pub mod property {
    pub const CAN_BE_DELETED: &str = "canBeDeleted";
    pub const READONLY: &str = "readonly";
    pub const KEEPS_HISTORY: &str = "keepsHistory";
    pub const DOCUMENTS_KEEP_HISTORY_CONTRACT_DEFAULT: &str = "documentsKeepHistoryContractDefault";
    pub const DOCUMENTS_MUTABLE_CONTRACT_DEFAULT: &str = "documentsMutableContractDefault";
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct ContractConfig {
    /// Can the contract ever be deleted. If the contract is deleted, so should be all
    /// documents associated with it. TODO: There should also be a way to "stop" the contract -
    /// contract and documents are kept in the system, but no new documents can be added to it
    pub can_be_deleted: bool,
    /// Is the contract mutable. Means that the document definitions can be changed or new
    /// document definitions can be added to the contract
    pub readonly: bool,
    /// Does the contract keep history when the contract itself changes
    pub keeps_history: bool,
    /// Do documents in the contract keep history. This is a default for all documents in
    /// the contract, but can be overridden by the document itself
    pub documents_keep_history_contract_default: bool,
    /// Are documents in the contract mutable. This specifies whether the document can be
    /// changed or deleted. This is a default for all documents in the contract, but can be
    /// overridden by the document itself
    pub documents_mutable_contract_default: bool,
}

impl std::default::Default for ContractConfig {
    fn default() -> Self {
        ContractConfig {
            can_be_deleted: DEFAULT_CONTRACT_CAN_BE_DELETED,
            readonly: !DEFAULT_CONTRACT_MUTABILITY,
            keeps_history: DEFAULT_CONTRACT_KEEPS_HISTORY,
            documents_keep_history_contract_default: DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
            documents_mutable_contract_default: DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
        }
    }
}
