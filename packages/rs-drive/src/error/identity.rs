/// Identity errors
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// Identity already exists error
    #[error("identity already exists error: {0}")]
    IdentityAlreadyExists(&'static str),

    /// Identity already exists error
    #[error("identity key already exists for user error: {0}")]
    IdentityKeyAlreadyExists(&'static str),

    /// A user is requesting an unknown key error
    #[error("identity public key not found: {0}")]
    IdentityPublicKeyNotFound(String),

    /// A unique key with that hash already exists
    #[error("a unique key with that hash already exists: {0}")]
    UniqueKeyAlreadyExists(&'static str),

    /// Missing required key error
    #[error("missing required key: {0}")]
    MissingRequiredKey(&'static str),

    /// Identity key missing field error
    #[error("identity key missing field: {0}")]
    IdentityKeyMissingField(&'static str),

    /// Field requirement unmet error
    #[error("field requirement unmet: {0}")]
    FieldRequirementUnmet(&'static str),

    /// Invalid identity structure error
    #[error("invalid identity structure: {0}")]
    InvalidIdentityStructure(&'static str),

    /// Identity serialization error
    #[error("identity serialization error: {0}")]
    IdentitySerialization(&'static str),

    /// Identity insufficient balance error
    #[error("identity insufficient balance error: {0}")]
    IdentityInsufficientBalance(&'static str),

    /// Critical balance overflow error
    #[error("critical balance overflow error: {0}")]
    CriticalBalanceOverflow(&'static str),

    /// Identity key incorrect query missing information error
    #[error("identity key incorrect query missing information error: {0}")]
    IdentityKeyIncorrectQueryMissingInformation(&'static str),
}
