use thiserror::Error;

#[derive(Error, Eq, PartialEq, Debug)]
pub enum Error {
    #[error("Unsupported: {0}")]
    Unsupported(String),

    #[error("Structure Error: {0}")]
    StructureError(String),

    #[error("Integer not big enough to hold value")]
    IntegerSizeError,
}
