use serde::{Deserialize, Serialize};

use super::OperationLike;
use crate::state_transition::fee::constants::{
    PROCESSING_CREDIT_PER_BYTE, STORAGE_CREDIT_PER_BYTE, WRITE_BASE_PROCESSING_COST,
};

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WriteOperation {
    pub key_size: usize,
    pub value_size: usize,
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
        WRITE_BASE_PROCESSING_COST
            + ((self.key_size + self.value_size) as i64 * PROCESSING_CREDIT_PER_BYTE)
    }

    fn get_storage_cost(&self) -> i64 {
        (self.key_size + self.value_size) as i64 * STORAGE_CREDIT_PER_BYTE
    }
}
