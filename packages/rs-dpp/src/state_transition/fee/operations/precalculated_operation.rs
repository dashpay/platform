use crate::credits::Credits;
use crate::state_transition::fee::{ExecutionFees, FeeRefunds};
use serde::{Deserialize, Serialize};

use super::OperationLike;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreCalculatedOperation(ExecutionFees);

impl OperationLike for PreCalculatedOperation {
    fn processing_fee(&self) -> Credits {
        self.0.processing_fee
    }

    fn storage_fee(&self) -> Credits {
        self.0.storage_fee
    }

    fn fee_refunds(&self) -> Option<&FeeRefunds> {
        Some(&self.0.fee_refunds)
    }
}
