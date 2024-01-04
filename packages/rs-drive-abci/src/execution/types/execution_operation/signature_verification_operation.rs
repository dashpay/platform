use crate::error::Error;
use crate::execution::types::execution_operation::OperationLike;
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;
use dpp::identity::KeyType;
use dpp::version::PlatformVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignatureVerificationOperation {
    pub signature_type: KeyType,
}

impl SignatureVerificationOperation {
    pub fn new(signature_type: KeyType) -> Self {
        Self { signature_type }
    }
}

impl OperationLike for SignatureVerificationOperation {
    fn processing_cost(
        &self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        Ok(self
            .signature_type
            .signature_verify_cost(platform_version, epoch)?)
    }

    fn storage_cost(&self, _platform_version: &PlatformVersion) -> Result<Credits, Error> {
        Ok(0)
    }
}
