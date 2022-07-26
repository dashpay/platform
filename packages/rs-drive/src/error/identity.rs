#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("missing required key: {0}")]
    MissingRequiredKey(&'static str),

    #[error("identity key missing field: {0}")]
    IdentityKeyMissingField(&'static str),

    #[error("field requirement unmet: {0}")]
    FieldRequirementUnmet(&'static str),

    #[error("invalid identity structure: {0}")]
    InvalidIdentityStructure(&'static str),

    #[error("identity serialization error: {0}")]
    IdentitySerialization(&'static str),
}
