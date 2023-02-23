use thiserror::Error;

#[derive(Error, Eq, PartialEq, Debug)]
pub enum Error {
    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("structure Error: {0}")]
    StructureError(String),

    #[error("integer out of bounds")]
    IntegerSizeError,
}
