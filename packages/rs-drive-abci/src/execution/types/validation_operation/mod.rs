use crate::error::Error;
use crate::execution::types::validation_operation::signature_verification_operation::SignatureVerificationOperation;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;

pub mod signature_verification_operation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationOperation {
    SignatureVerification(SignatureVerificationOperation),
}

pub trait OperationLike {
    fn processing_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error>;

    fn storage_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error>;
}

impl OperationLike for ValidationOperation {
    fn processing_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error> {
        match self {
            ValidationOperation::SignatureVerification(op) => op.processing_cost(platform_version),
        }
    }

    fn storage_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error> {
        match self {
            ValidationOperation::SignatureVerification(op) => op.storage_cost(platform_version),
        }
    }
}
