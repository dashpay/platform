use serde::{Deserialize, Serialize};

use super::OperationLike;

use crate::{
    prelude::Fee,
    state_transition::fee::constants::{PROCESSING_CREDIT_PER_BYTE, READ_BASE_PROCESSING_COST},
};

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReadOperation {
    pub value_size: usize,
}

impl ReadOperation {
    pub fn new(value_size: usize) -> Self {
        Self { value_size }
    }
}

impl OperationLike for ReadOperation {
    fn get_processing_cost(&self) -> Fee {
        READ_BASE_PROCESSING_COST
            .saturating_add((self.value_size as Fee).saturating_mul(PROCESSING_CREDIT_PER_BYTE))
    }

    fn get_storage_cost(&self) -> Fee {
        0
    }
}
