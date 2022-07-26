use serde::{Deserialize, Serialize};

pub const DEFAULT_CONTRACT_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_MUTABILITY: bool = true;
pub const DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_DOCUMENT_MUTABILITY: bool = true;

pub mod property {
    pub const READONLY: &str = "readonly";
    pub const KEEPS_HISTORY: &str = "keepsHistory";
    pub const DOCUMENTS_KEEP_HISTORY_CONTRACT_DEFAULT: &str = "documentsKeepHistoryContractDefault";
    pub const DOCUMENTS_MUTABLE_CONTRACT_DEFAULT: &str = "documentsMutableContractDefault";
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct ContractConfig {
    /// Is the contract mutable
    pub readonly: bool,
    /// Does the contract keep history when the contract itself changes
    pub keeps_history: bool,
    /// Do documents in the contract keep history
    pub documents_keep_history_contract_default: bool,
    /// Are documents in the contract mutable
    pub documents_mutable_contract_default: bool,
}

impl std::default::Default for ContractConfig {
    fn default() -> Self {
        ContractConfig {
            readonly: !DEFAULT_CONTRACT_MUTABILITY,
            keeps_history: DEFAULT_CONTRACT_KEEPS_HISTORY,
            documents_keep_history_contract_default: DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
            documents_mutable_contract_default: DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
        }
    }
}
