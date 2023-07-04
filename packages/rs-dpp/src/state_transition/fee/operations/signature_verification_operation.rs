use serde::{Deserialize, Serialize};

use super::OperationLike;
use crate::{
    identity::KeyType,
    NonConsensusError,
    state_transition::fee::Refunds,
};
use crate::fee::Credits;
use crate::fee::default_costs::constants::signature_verify_cost;

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
