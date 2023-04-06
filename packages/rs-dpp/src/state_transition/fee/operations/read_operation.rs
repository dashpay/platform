use crate::credits::Credits;
use serde::{Deserialize, Serialize};

use super::OperationLike;

use crate::state_transition::fee::constants::{
    PROCESSING_CREDIT_PER_BYTE, READ_BASE_PROCESSING_COST,
};
use crate::state_transition::fee::FeeRefunds;

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
    fn processing_fee(&self) -> Credits {
        READ_BASE_PROCESSING_COST + (self.value_size * PROCESSING_CREDIT_PER_BYTE)
    }

    fn storage_fee(&self) -> Credits {
        0
    }

    fn fee_refunds(&self) -> Option<&FeeRefunds> {
        None
    }
}
