use super::{OperationLike, OperationType};

pub const BASE_PROCESSING_COST: i64 = 8400;
pub const CREDIT_PER_BYTE: i64 = 12;

#[derive(Default, Debug, Clone, Copy)]
pub struct ReadOperation {
    value_size: usize,
}

impl ReadOperation {
    pub fn new(value_size: usize) -> Self {
        Self { value_size }
    }
}

impl OperationLike for ReadOperation {
    fn get_processing_cost(&self) -> i64 {
        BASE_PROCESSING_COST + (self.value_size as i64 * CREDIT_PER_BYTE)
    }

    fn get_storage_cost(&self) -> i64 {
        0
    }

    fn get_type(&self) -> OperationType {
        OperationType::Read
    }
}
