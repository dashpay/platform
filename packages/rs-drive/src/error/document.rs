/// Document errors
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// Error
    #[error("missing document propoerty error: {0}")]
    MissingDocumentProperty(&'static str),
    /// Error
    #[error("invalid document propoerty type error: {0}")]
    InvalidDocumentPropertyType(&'static str),
}
