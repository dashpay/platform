use serde::{Deserialize, Serialize};

use super::OperationLike;

use crate::fee::default_costs::constants::{PROCESSING_CREDIT_PER_BYTE, READ_BASE_PROCESSING_COST};
use crate::fee::Credits;
use crate::NonConsensusError;

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
    fn get_processing_cost(&self) -> Result<Credits, NonConsensusError> {
        let value_byte_processing_cost = self
            .value_size
            .checked_mul(PROCESSING_CREDIT_PER_BYTE)
            .ok_or(NonConsensusError::Overflow(
                "value processing cost is too big",
            ))?;

        READ_BASE_PROCESSING_COST
            .checked_add(value_byte_processing_cost)
            .ok_or(NonConsensusError::Overflow(
                "can't add read base processing cost",
            ))
    }

    fn get_storage_cost(&self) -> Result<Credits, NonConsensusError> {
        Ok(0)
    }

    fn get_refunds(&self) -> Option<&Vec<Refunds>> {
        None
    }
}
