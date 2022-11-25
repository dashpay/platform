use serde::{Deserialize, Serialize};

use super::OperationLike;
use crate::state_transition::fee::constants::{
    DELETE_BASE_PROCESSING_COST, PROCESSING_CREDIT_PER_BYTE, STORAGE_CREDIT_PER_BYTE,
};

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
        (DELETE_BASE_PROCESSING_COST
            + ((self.key_size as i64 + self.value_size as i64) * PROCESSING_CREDIT_PER_BYTE))
            as i64
    }

    fn get_storage_cost(&self) -> i64 {
        -((self.key_size + self.value_size) as i64 * STORAGE_CREDIT_PER_BYTE)
    }
}
