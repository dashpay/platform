use super::{OperationLike, OperationType};

pub const BASE_PROCESSING_COST: i64 = 20000;

#[derive(Default, Debug, Clone, Copy)]
pub struct PreCalculatedOperation {
    storage_cost: i64,
    processing_cost: i64,
}

impl PreCalculatedOperation {
    pub fn new(storage_cost: i64, processing_cost: i64) -> Self {
        Self {
            storage_cost,
            processing_cost,
        }
    }
}

impl OperationLike for PreCalculatedOperation {
    fn get_processing_cost(&self) -> i64 {
        self.processing_cost
    }

    fn get_storage_cost(&self) -> i64 {
        self.storage_cost
    }

    fn get_type(&self) -> OperationType {
        OperationType::PreCalculated
    }
}
