use serde::{Deserialize, Serialize};

use super::OperationLike;

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreCalculatedOperation {
    pub storage_cost: i64,
    pub processing_cost: i64,
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
}
