use crate::error::execution::ExecutionError;
use crate::error::serialization::SerializationError;
use rs_drive::error::Error as DriveError;

pub mod execution;
pub mod serialization;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("storage: {0}")]
    Drive(#[from] DriveError),
    #[error("execution: {0}")]
    Execution(#[from] ExecutionError),
    #[error("serialization: {0}")]
    Serialization(#[from] SerializationError),
}
