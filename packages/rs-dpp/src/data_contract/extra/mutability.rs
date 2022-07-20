use serde::{Deserialize, Serialize};

pub const DEFAULT_CONTRACT_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_MUTABILITY: bool = true;
pub const DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_DOCUMENT_MUTABILITY: bool = true;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
// TODO change the name to something more meaningful
pub struct Mutability {
    /// Is the contract mutable
    pub readonly: bool,
    /// Does the contract keep history when the contract itself changes
    pub keeps_history: bool,
    /// Do documents in the contract keep history
    pub documents_keep_history_contract_default: bool,
    /// Are documents in the contract mutable
    pub documents_mutable_contract_default: bool,
}

impl std::default::Default for Mutability {
    fn default() -> Self {
        Mutability {
            readonly: !DEFAULT_CONTRACT_MUTABILITY,
            keeps_history: DEFAULT_CONTRACT_KEEPS_HISTORY,
            documents_keep_history_contract_default: DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
            documents_mutable_contract_default: DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
        }
    }
}
