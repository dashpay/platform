use serde::{Deserialize, Serialize};

use super::OperationLike;

use crate::state_transition::fee::{
    constants::{PROCESSING_CREDIT_PER_BYTE, READ_BASE_PROCESSING_COST},
    Credits, Refunds,
};

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReadOperation {
    pub value_size: Credits,
}

impl ReadOperation {
    pub fn new(value_size: u64) -> Self {
        Self { value_size }
    }
}

impl OperationLike for ReadOperation {
    fn get_processing_cost(&self) -> Credits {
        READ_BASE_PROCESSING_COST + (self.value_size * PROCESSING_CREDIT_PER_BYTE)
    }

    fn get_storage_cost(&self) -> Credits {
        0
    }

    fn get_refunds(&self) -> Option<&Vec<Refunds>> {
        None
    }
}
