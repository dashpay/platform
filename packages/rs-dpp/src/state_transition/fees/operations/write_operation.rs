use super::{
    OperationLike, OperationType, STORAGE_CREDIT_PER_BYTE, STORAGE_PROCESSING_CREDIT_PER_BYTE,
};

pub const BASE_PROCESSING_COST: i64 = 60000;

#[derive(Default, Debug, Clone, Copy)]
pub struct WriteOperation {
    key_size: usize,
    value_size: usize,
}

impl WriteOperation {
    pub fn new(key_size: usize, value_size: usize) -> Self {
        Self {
            key_size,
            value_size,
        }
    }
}

impl OperationLike for WriteOperation {
    fn get_processing_cost(&self) -> i64 {
        BASE_PROCESSING_COST
            + ((self.key_size + self.value_size) as i64 * STORAGE_PROCESSING_CREDIT_PER_BYTE)
    }

    fn get_storage_cost(&self) -> i64 {
        (self.key_size + self.value_size) as i64 * STORAGE_CREDIT_PER_BYTE
    }

    fn get_type(&self) -> OperationType {
        OperationType::Write
    }
}
