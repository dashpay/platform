#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("missing document propoerty error: {0}")]
    MissingDocumentProperty(&'static str),
    #[error("invalid document propoerty type error: {0}")]
    InvalidDocumentPropertyType(&'static str),
}
