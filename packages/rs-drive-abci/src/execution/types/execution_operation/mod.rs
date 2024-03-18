use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use dpp::block::epoch::Epoch;
use dpp::fee::default_costs::EpochCosts;
use dpp::fee::default_costs::KnownCostItem::DoubleSHA256;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;

pub mod signature_verification_operation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationOperation {
    DoubleSha256,
    SignatureVerification(SignatureVerificationOperation),
    PrecalculatedOperation(FeeResult),
}

pub trait OperationLike {
    fn processing_cost(
        &self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error>;

    fn storage_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error>;
}

impl ValidationOperation {
    pub fn add_many_to_fee_result(
        execution_operations: &[ValidationOperation],
        fee_result: &mut FeeResult,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for execution_operation in execution_operations {
            match execution_operation {
                ValidationOperation::SignatureVerification(signature_verification_operation) => {
                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(
                            signature_verification_operation
                                .processing_cost(epoch, platform_version)?,
                        )
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
                ValidationOperation::PrecalculatedOperation(precalculated_operation) => {
                    fee_result.checked_add_assign(precalculated_operation.clone())?;
                }
                ValidationOperation::DoubleSha256 => {
                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(epoch.cost_for_known_cost_item(DoubleSHA256))
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
            }
        }
        Ok(())
    }
}
