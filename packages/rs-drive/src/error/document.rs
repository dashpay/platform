/// Document errors
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// Error
    #[error("missing document property error: {0}")]
    MissingDocumentProperty(&'static str),
    /// Error
    #[error("invalid document property type error: {0}")]
    InvalidDocumentPropertyType(&'static str),
    /// Error
    #[error("invalid contract identifier size error")]
    InvalidContractIdSize(),
    /// Error
    #[error("contract with specified identifier is not found")]
    DataContractNotFound,
}
