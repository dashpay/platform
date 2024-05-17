pub const DEFAULT_CONTRACT_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_CAN_BE_DELETED: bool = false;
pub const DEFAULT_CONTRACT_MUTABILITY: bool = true;
pub const DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY: bool = false;
pub const DEFAULT_CONTRACT_DOCUMENT_MUTABILITY: bool = true;
pub const DEFAULT_CONTRACT_DOCUMENTS_CAN_BE_DELETED: bool = true;

pub mod property {
    pub const CAN_BE_DELETED: &str = "canBeDeleted";
    pub const READONLY: &str = "readonly";
    pub const KEEPS_HISTORY: &str = "keepsHistory";
    pub const DOCUMENTS_KEEP_HISTORY_CONTRACT_DEFAULT: &str = "documentsKeepHistoryContractDefault";
    pub const DOCUMENTS_MUTABLE_CONTRACT_DEFAULT: &str = "documentsMutableContractDefault";
    pub const DOCUMENTS_CAN_BE_DELETED_CONTRACT_DEFAULT: &str =
        "documentsCanBeDeletedContractDefault";
    pub const REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY: &str =
        "requiresIdentityEncryptionBoundedKey";
    pub const REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY: &str =
        "requiresIdentityDecryptionBoundedKey";
}
