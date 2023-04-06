use serde::{Deserialize, Serialize};

use super::OperationLike;
use crate::credits::Credits;
use crate::state_transition::fee::FeeRefunds;
use crate::{identity::KeyType, state_transition::fee::constants::signature_verify_cost};

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
    fn processing_fee(&self) -> Credits {
        signature_verify_cost(self.signature_type)
    }

    fn storage_fee(&self) -> Credits {
        0
    }

    fn fee_refunds(&self) -> Option<FeeRefunds> {
        None
    }
}
