use serde::{Deserialize, Serialize};

use super::OperationLike;
use crate::{
    prelude::Fee,
    state_transition::fee::constants::{
        DELETE_BASE_PROCESSING_COST, PROCESSING_CREDIT_PER_BYTE, STORAGE_CREDIT_PER_BYTE,
    },
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
    fn get_processing_cost(&self) -> Fee {
        let current_cost = (self.key_size.saturating_add(self.value_size) as Fee)
            .saturating_mul(PROCESSING_CREDIT_PER_BYTE);

        DELETE_BASE_PROCESSING_COST.saturating_add(current_cost)
    }

    fn get_storage_cost(&self) -> Fee {
        -(self.key_size.saturating_add(self.value_size) as Fee)
            .saturating_mul(STORAGE_CREDIT_PER_BYTE)
    }
}
