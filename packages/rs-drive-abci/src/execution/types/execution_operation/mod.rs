use crate::error::Error;
use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use dpp::fee::Credits;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use drive::drive::batch::DriveOperation;

pub mod signature_verification_operation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionOperation {
    SignatureVerification(SignatureVerificationOperation),
    PrecalculatedOperation(FeeResult)
}

pub trait OperationLike {
    fn processing_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error>;

    fn storage_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error>;
}
