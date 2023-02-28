use thiserror::Error;

#[derive(Error, Eq, PartialEq, Debug)]
pub enum Error {
    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("structure error: {0}")]
    StructureError(String),

    #[error("path error: {0}")]
    PathError(String),

    #[error("integer out of bounds")]
    IntegerSizeError,
}
