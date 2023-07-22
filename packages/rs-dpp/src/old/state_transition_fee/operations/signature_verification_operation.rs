use crate::fee::default_costs::constants::signature_verify_cost;
use crate::fee::Credits;
use crate::identity::KeyType;
use crate::NonConsensusError;
use serde::{Deserialize, Serialize};

use super::OperationLike;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SignatureVerificationOperation {
    pub signature_type: KeyType,
}

impl SignatureVerificationOperation {
    pub fn new(signature_type: KeyType) -> Self {
        Self { signature_type }
    }
}

impl OperationLike for SignatureVerificationOperation {
    fn get_processing_cost(&self) -> Result<Credits, NonConsensusError> {
        Ok(signature_verify_cost(self.signature_type))
    }

    fn get_storage_cost(&self) -> Result<Credits, NonConsensusError> {
        Ok(0)
    }

    fn get_refunds(&self) -> Option<&Vec<Refunds>> {
        None
    }
}
