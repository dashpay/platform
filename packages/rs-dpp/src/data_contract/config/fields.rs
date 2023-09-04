pub const DEFAULT_CONTRACT_KEEPS_HISTORY: bool = false;
pub const DEFAULT_ALLOW_CONTRACT_DELETION: bool = false;
pub const DEFAULT_ALLOW_CONTRACT_UPDATE: bool = true;
pub const DEFAULT_DOCUMENTS_KEEP_HISTORY: bool = false;
pub const DEFAULT_DOCUMENTS_MUTABILITY: bool = true;

pub mod property {
    pub const CAN_BE_DELETED: &str = "canBeDeleted";
    pub const READONLY: &str = "readonly";
    pub const KEEPS_HISTORY: &str = "keepsHistory";
    pub const DOCUMENTS_KEEP_HISTORY_CONTRACT_DEFAULT: &str = "documentsKeepHistoryContractDefault";
    pub const DOCUMENTS_MUTABLE_CONTRACT_DEFAULT: &str = "documentsMutableContractDefault";
}
