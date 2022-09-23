use serde::{Deserialize, Serialize};

use super::{OperationLike, OperationType, STORAGE_PROCESSING_CREDIT_PER_BYTE};

pub const BASE_PROCESSING_COST: i64 = 20000;

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteOperation {
    pub key_size: usize,
    pub value_size: usize,
}

impl DeleteOperation {
    pub fn new(key_size: usize, value_size: usize) -> Self {
        Self {
            key_size,
            value_size,
        }
    }
}

impl OperationLike for DeleteOperation {
    fn get_processing_cost(&self) -> i64 {
        (BASE_PROCESSING_COST
            + ((self.key_size as i64 + self.value_size as i64)
                * STORAGE_PROCESSING_CREDIT_PER_BYTE)) as i64
    }

    fn get_storage_cost(&self) -> i64 {
        -((self.key_size + self.value_size) as i64 * STORAGE_PROCESSING_CREDIT_PER_BYTE)
    }

    fn get_type(&self) -> OperationType {
        OperationType::Delete
    }
}
