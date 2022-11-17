use serde::{Deserialize, Serialize};

use super::OperationLike;
use crate::{identity::KeyType, state_transition::fee::constants::signature_verify_cost};

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
    fn get_processing_cost(&self) -> i64 {
        signature_verify_cost(self.signature_type)
    }

    fn get_storage_cost(&self) -> i64 {
        0
    }
}
