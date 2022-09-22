mod delete_operation;

pub use delete_operation::*;

mod precalculated_operation;
pub use precalculated_operation::*;

mod read_operation;
pub use read_operation::*;

mod write_operation;
pub use write_operation::*;

pub const STORAGE_CREDIT_PER_BYTE: i64 = 5000;
pub const STORAGE_PROCESSING_CREDIT_PER_BYTE: i64 = 5000;

pub enum Operation {
    Read(ReadOperation),
    Write(WriteOperation),
    Delete(DeleteOperation),
}

pub trait OperationLike {
    /// Get CPU cost of the operation
    fn get_processing_cost(&self) -> i64;
    /// Get storage cost of the operation
    fn get_storage_cost(&self) -> i64;
    /// Get operation type
    fn get_type(&self) -> OperationType;
}

#[derive(Debug, Clone, Copy)]
pub enum OperationType {
    Write,
    Read,
    Delete,
    PreCalculated,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Write => write!(f, "write"),
            Self::Read => write!(f, "read"),
            Self::Delete => write!(f, "delete"),
            Self::PreCalculated => write!(f, "preCalculated"),
        }
    }
}

macro_rules! call_method {
    ($operation_type:expr, $method:ident ) => {
        match $operation_type {
            Operation::Read(op) => op.$method(),
            Operation::Write(op) => op.$method(),
            Operation::Delete(op) => op.$method(),
        }
    };
}

impl OperationLike for Operation {
    fn get_processing_cost(&self) -> i64 {
        call_method!(self, get_processing_cost)
    }

    fn get_storage_cost(&self) -> i64 {
        call_method!(self, get_storage_cost)
    }

    fn get_type(&self) -> OperationType {
        call_method!(self, get_type)
    }
}
