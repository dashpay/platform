use serde::{Deserialize, Serialize};

use crate::fee::Credits;
use crate::NonConsensusError;

use super::OperationLike;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreCalculatedOperation {
    pub storage_cost: Credits,
    pub processing_cost: Credits,
    pub fee_refunds: Vec<Refunds>,
}

impl PreCalculatedOperation {
    pub fn from_fee(fee: DummyFeesResult) -> Self {
        Self {
            fee_refunds: fee.fee_refunds,
            processing_cost: fee.processing,
            storage_cost: fee.storage,
        }
    }

    pub fn new(
        storage_cost: Credits,
        processing_cost: Credits,
        fee_refunds: impl IntoIterator<Item = Refunds>,
    ) -> Self {
        Self {
            storage_cost,
            processing_cost,
            fee_refunds: fee_refunds.into_iter().collect(),
        }
    }
}

impl OperationLike for PreCalculatedOperation {
    fn get_processing_cost(&self) -> Result<Credits, NonConsensusError> {
        Ok(self.processing_cost)
    }

    fn get_storage_cost(&self) -> Result<Credits, NonConsensusError> {
        Ok(self.storage_cost)
    }

    fn get_refunds(&self) -> Option<&Vec<Refunds>> {
        Some(&self.fee_refunds)
    }
}
