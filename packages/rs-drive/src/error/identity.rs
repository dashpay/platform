/// Identity errors
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
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
}
