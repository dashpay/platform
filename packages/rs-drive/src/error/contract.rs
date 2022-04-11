#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error("missing required key: {0}")]
    MissingRequiredKey(&'static str),

    #[error("field requirement unmet: {0}")]
    FieldRequirementUnmet(&'static str),

    #[error("key wrong type error: {0}")]
    KeyWrongType(&'static str),

    #[error("value wrong type error: {0}")]
    ValueWrongType(&'static str),

    #[error("value decoding error: {0}")]
    ValueDecodingError(&'static str),

    #[error("encoding data structure not supported error: {0}")]
    EncodingDataStructureNotSupported(&'static str),

    #[error("invalid contract structure: {0}")]
    InvalidContractStructure(&'static str),

    #[error("document type not found: {0}")]
    DocumentTypeNotFound(&'static str),

    #[error("document type field not found: {0}")]
    DocumentTypeFieldNotFound(&'static str),

    #[error("reference definition not found error: {0}")]
    ReferenceDefinitionNotFound(&'static str),

    #[error("document owner id missing error: {0}")]
    DocumentOwnerIdMissing(&'static str),

    #[error("document id missing error: {0}")]
    DocumentIdMissing(&'static str),
}
