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

    pub fn only_revision() -> Self {
        RetrieveIdentityInfo {
            query_by_key_id_key_count: 0,
            request_balance: false,
            request_revision: true,
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
    RetrieveIdentityTokenBalance,
    RetrieveIdentity(RetrieveIdentityInfo),
    RetrievePrefundedSpecializedBalance,
    RetrieveAddressNonceAndBalance(u16),
    PerformNetworkThresholdSigning,
    SingleSha256(HashBlockCount),
    DoubleSha256(HashBlockCount),
    Ripemd160(HashBlockCount),
    ValidateKeyStructure(KeyCount), // This is extremely cheap
    SignatureVerification(SignatureVerificationOperation),
    PrecalculatedOperation(FeeResult),
}

pub trait OperationLike {
    fn processing_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error>;

    #[allow(dead_code)]
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
                ValidationOperation::Ripemd160(block_count) => {
                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(
                            platform_version.fee_version.hashing.ripemd160_per_block
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
                ValidationOperation::PerformNetworkThresholdSigning => {
                    let operation_cost = platform_version
                        .fee_version
                        .processing
                        .perform_network_threshold_signing;

                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(operation_cost)
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
                ValidationOperation::RetrieveIdentityTokenBalance => {
                    let operation_cost = platform_version
                        .fee_version
                        .processing
                        .fetch_identity_token_balance_processing_cost;

                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(operation_cost)
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
                ValidationOperation::RetrieveAddressNonceAndBalance(key_count) => {
                    let operation_cost = platform_version
                        .fee_version
                        .processing
                        .fetch_key_with_type_nonce_and_balance_cost
                        * *key_count as u64;

                    fee_result.processing_fee = fee_result
                        .processing_fee
                        .checked_add(operation_cost)
                        .ok_or(ExecutionError::Overflow(
                            "execution processing fee overflow error",
                        ))?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identity::KeyType;
    use dpp::validation::operations::ProtocolValidationOperation;
    use platform_version::version::PlatformVersion;

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    #[test]
    fn retrieve_identity_info_only_balance() {
        let info = RetrieveIdentityInfo::only_balance();
        assert_eq!(info.query_by_key_id_key_count, 0);
        assert!(info.request_balance);
        assert!(!info.request_revision);
    }

    #[test]
    fn retrieve_identity_info_only_revision() {
        let info = RetrieveIdentityInfo::only_revision();
        assert_eq!(info.query_by_key_id_key_count, 0);
        assert!(!info.request_balance);
        assert!(info.request_revision);
    }

    #[test]
    fn retrieve_identity_info_one_key() {
        let info = RetrieveIdentityInfo::one_key();
        assert_eq!(info.query_by_key_id_key_count, 1);
        assert!(!info.request_balance);
        assert!(!info.request_revision);
    }

    #[test]
    fn retrieve_identity_info_one_key_and_balance_and_revision() {
        let info = RetrieveIdentityInfo::one_key_and_balance_and_revision();
        assert_eq!(info.query_by_key_id_key_count, 1);
        assert!(info.request_balance);
        assert!(info.request_revision);
    }

    #[test]
    fn retrieve_identity_info_one_key_and_balance() {
        let info = RetrieveIdentityInfo::one_key_and_balance();
        assert_eq!(info.query_by_key_id_key_count, 1);
        assert!(info.request_balance);
        assert!(!info.request_revision);
    }

    #[test]
    fn retrieve_identity_info_one_key_and_revision() {
        let info = RetrieveIdentityInfo::one_key_and_revision();
        assert_eq!(info.query_by_key_id_key_count, 1);
        assert!(!info.request_balance);
        assert!(info.request_revision);
    }

    #[test]
    fn add_many_empty_operations_is_noop() {
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&[], &mut fee_result, platform_version())
            .unwrap();
        assert_eq!(fee_result.processing_fee, 0);
        assert_eq!(fee_result.storage_fee, 0);
    }

    #[test]
    fn add_many_signature_verification() {
        let ops = vec![ValidationOperation::SignatureVerification(
            SignatureVerificationOperation::new(KeyType::ECDSA_SECP256K1),
        )];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        assert!(fee_result.processing_fee > 0);
    }

    #[test]
    fn add_many_precalculated_operation() {
        let precalc = FeeResult {
            processing_fee: 100,
            storage_fee: 50,
            ..Default::default()
        };
        let ops = vec![ValidationOperation::PrecalculatedOperation(precalc)];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        assert_eq!(fee_result.processing_fee, 100);
        assert_eq!(fee_result.storage_fee, 50);
    }

    #[test]
    fn add_many_single_sha256() {
        let ops = vec![ValidationOperation::SingleSha256(2)];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        let pv = platform_version();
        let expected =
            pv.fee_version.hashing.single_sha256_base + pv.fee_version.hashing.sha256_per_block * 2;
        assert_eq!(fee_result.processing_fee, expected);
    }

    #[test]
    fn add_many_double_sha256() {
        let ops = vec![ValidationOperation::DoubleSha256(3)];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        let pv = platform_version();
        let expected =
            pv.fee_version.hashing.single_sha256_base + pv.fee_version.hashing.sha256_per_block * 3;
        assert_eq!(fee_result.processing_fee, expected);
    }

    #[test]
    fn add_many_ripemd160() {
        let ops = vec![ValidationOperation::Ripemd160(4)];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        let pv = platform_version();
        let expected = pv.fee_version.hashing.ripemd160_per_block * 4;
        assert_eq!(fee_result.processing_fee, expected);
    }

    #[test]
    fn add_many_retrieve_identity_balance_only() {
        let ops = vec![ValidationOperation::RetrieveIdentity(
            RetrieveIdentityInfo::only_balance(),
        )];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        assert!(fee_result.processing_fee > 0);
    }

    #[test]
    fn add_many_retrieve_identity_revision_only() {
        let ops = vec![ValidationOperation::RetrieveIdentity(
            RetrieveIdentityInfo::only_revision(),
        )];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        assert!(fee_result.processing_fee > 0);
    }

    #[test]
    fn add_many_retrieve_identity_one_key_and_balance_and_revision() {
        let ops = vec![ValidationOperation::RetrieveIdentity(
            RetrieveIdentityInfo::one_key_and_balance_and_revision(),
        )];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        assert!(fee_result.processing_fee > 0);
    }

    #[test]
    fn add_many_retrieve_identity_no_balance_no_revision_no_keys() {
        // Tests the (false, false) branch with 0 keys
        let ops = vec![ValidationOperation::RetrieveIdentity(
            RetrieveIdentityInfo {
                query_by_key_id_key_count: 0,
                request_balance: false,
                request_revision: false,
            },
        )];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        // Base cost is 0 when both flags are false and no keys
        assert_eq!(fee_result.processing_fee, 0);
    }

    #[test]
    fn add_many_retrieve_prefunded_specialized_balance() {
        let ops = vec![ValidationOperation::RetrievePrefundedSpecializedBalance];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        assert!(fee_result.processing_fee > 0);
    }

    #[test]
    fn add_many_validate_key_structure() {
        let ops = vec![ValidationOperation::ValidateKeyStructure(5)];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        let pv = platform_version();
        let expected = pv.fee_version.processing.validate_key_structure * 5;
        assert_eq!(fee_result.processing_fee, expected);
    }

    #[test]
    fn add_many_protocol_operation() {
        let ops = vec![ValidationOperation::Protocol(
            ProtocolValidationOperation::DocumentTypeSchemaValidationForSize(100),
        )];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        assert!(fee_result.processing_fee > 0);
    }

    #[test]
    fn add_many_perform_network_threshold_signing() {
        let ops = vec![ValidationOperation::PerformNetworkThresholdSigning];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        assert!(fee_result.processing_fee > 0);
    }

    #[test]
    fn add_many_retrieve_identity_token_balance() {
        let ops = vec![ValidationOperation::RetrieveIdentityTokenBalance];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        assert!(fee_result.processing_fee > 0);
    }

    #[test]
    fn add_many_retrieve_address_nonce_and_balance() {
        let ops = vec![ValidationOperation::RetrieveAddressNonceAndBalance(3)];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        let pv = platform_version();
        let expected = pv
            .fee_version
            .processing
            .fetch_key_with_type_nonce_and_balance_cost
            * 3;
        assert_eq!(fee_result.processing_fee, expected);
    }

    #[test]
    fn add_many_multiple_operations_accumulate() {
        let ops = vec![
            ValidationOperation::SingleSha256(1),
            ValidationOperation::SingleSha256(1),
        ];
        let mut fee_result = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(&ops, &mut fee_result, platform_version())
            .unwrap();
        let pv = platform_version();
        let single =
            pv.fee_version.hashing.single_sha256_base + pv.fee_version.hashing.sha256_per_block;
        assert_eq!(fee_result.processing_fee, single * 2);
    }
}
