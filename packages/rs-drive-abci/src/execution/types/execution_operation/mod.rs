use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::identity::KeyCount;
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::version::PlatformVersion;

pub mod signature_verification_operation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrieveIdentityInfo {
    query_by_key_id_key_count: u16,
    request_balance: bool,
    request_revision: bool,
}

impl RetrieveIdentityInfo {
    pub fn only_balance() -> Self {
        RetrieveIdentityInfo {
            query_by_key_id_key_count: 0,
            request_balance: true,
            request_revision: false,
        }
    }

    pub fn one_key() -> Self {
        RetrieveIdentityInfo {
            query_by_key_id_key_count: 1,
            request_balance: false,
            request_revision: false,
        }
    }

    pub fn one_key_and_balance_and_revision() -> Self {
        RetrieveIdentityInfo {
            query_by_key_id_key_count: 1,
            request_balance: true,
            request_revision: true,
        }
    }

    pub fn one_key_and_balance() -> Self {
        RetrieveIdentityInfo {
            query_by_key_id_key_count: 1,
            request_balance: true,
            request_revision: false,
        }
    }

    pub fn one_key_and_revision() -> Self {
        RetrieveIdentityInfo {
            query_by_key_id_key_count: 1,
            request_balance: false,
            request_revision: true,
        }
    }
}

pub type HashBlockCount = u16;

pub const SHA256_BLOCK_SIZE: u16 = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationOperation {
    Protocol(ProtocolValidationOperation),
    RetrieveIdentity(RetrieveIdentityInfo),
    RetrievePrefundedSpecializedBalance,
    SingleSha256(HashBlockCount),
    DoubleSha256(HashBlockCount),
    ValidateKeyStructure(KeyCount), // This is extremely cheap
    SignatureVerification(SignatureVerificationOperation),
    PrecalculatedOperation(FeeResult),
}

pub trait OperationLike {
    fn processing_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error>;

    fn storage_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error>;
}

impl ValidationOperation {
    pub fn add_many_to_fee_result(
        execution_operations: &[ValidationOperation],
        fee_result: &mut FeeResult,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for execution_operation in execution_operations {
            match execution_operation {
                ValidationOperation::SignatureVerification(signature_verification_operation) => {
                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(
                            signature_verification_operation.processing_cost(platform_version)?,
                        )
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
                ValidationOperation::PrecalculatedOperation(precalculated_operation) => {
                    fee_result.checked_add_assign(precalculated_operation.clone())?;
                }
                ValidationOperation::SingleSha256(block_count) => {
                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(
                            platform_version.fee_version.hashing.single_sha256_base
                                + platform_version.fee_version.hashing.sha256_per_block
                                    * (*block_count as u64),
                        )
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
                ValidationOperation::DoubleSha256(block_count) => {
                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(
                            platform_version.fee_version.hashing.single_sha256_base
                                + platform_version.fee_version.hashing.sha256_per_block
                                    * (*block_count as u64),
                        )
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
                ValidationOperation::RetrieveIdentity(RetrieveIdentityInfo {
                    query_by_key_id_key_count,
                    request_balance,
                    request_revision,
                }) => {
                    let base_cost = match (request_balance, request_revision) {
                        (true, true) => {
                            platform_version
                                .fee_version
                                .processing
                                .fetch_identity_balance_and_revision_processing_cost
                        }
                        (true, false) => {
                            platform_version
                                .fee_version
                                .processing
                                .fetch_identity_revision_processing_cost
                        }
                        (false, true) => {
                            platform_version
                                .fee_version
                                .processing
                                .fetch_identity_balance_processing_cost
                        }
                        (false, false) => 0,
                    };

                    let key_cost = platform_version
                        .fee_version
                        .processing
                        .fetch_identity_cost_per_look_up_key_by_id
                        .checked_mul(*query_by_key_id_key_count as u64)
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;

                    let operation_cost =
                        base_cost
                            .checked_add(key_cost)
                            .ok_or(ExecutionError::Overflow(
                                "execution processing fee overflow error",
                            ))?;

                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(operation_cost)
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
                ValidationOperation::RetrievePrefundedSpecializedBalance => {
                    let operation_cost = platform_version
                        .fee_version
                        .processing
                        .fetch_prefunded_specialized_balance_processing_cost;

                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(operation_cost)
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
                ValidationOperation::ValidateKeyStructure(key_count) => {
                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(
                            platform_version
                                .fee_version
                                .processing
                                .validate_key_structure
                                * (*key_count as u64),
                        )
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
                ValidationOperation::Protocol(dpp_validation_operation) => {
                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(dpp_validation_operation.processing_cost(platform_version))
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
            }
        }
        Ok(())
    }
}
