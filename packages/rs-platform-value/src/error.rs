use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Structure Error: {0}")]
    StructureError(String),

    #[error("Integer not big enough to hold value")]
    IntegerSizeError,
}