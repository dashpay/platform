use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::address_funds::AddressWitnessVerificationOperations;
use dpp::serialization::Signable;
use dpp::state_transition::{StateTransition, StateTransitionWitnessValidation};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

/// A trait for validating address witnesses within a state transition.
pub(crate) trait StateTransitionAddressWitnessValidationV0 {
    /// Validates the address witnesses for this state transition.
    ///
    /// # Arguments
    ///
    /// * `execution_context` – The execution context to track operations for fee calculation.
    /// * `platform_version` – The active platform version.
    ///
    /// # Returns
    ///
    /// Returns a [`SimpleConsensusValidationResult`] on success
    /// or an [`Error`] if validation fails.
    fn validate_address_witnesses(
        &self,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

pub(crate) trait StateTransitionHasAddressWitnessValidationV0 {
    /// True if the state transition has address witness validation.
    fn has_address_witness_validation(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error>;
}

impl StateTransitionAddressWitnessValidationV0 for StateTransition {
    fn validate_address_witnesses(
        &self,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .validate_address_witnesses
        {
            0 => {
                let signable_bytes = self.signable_bytes()?;

                let witness_result = match self {
                    StateTransition::AddressFundsTransfer(st) => {
                        st.validate_witnesses(&signable_bytes)
                    }
                    StateTransition::IdentityCreateFromAddresses(st) => {
                        st.validate_witnesses(&signable_bytes)
                    }
                    StateTransition::IdentityTopUpFromAddresses(st) => {
                        st.validate_witnesses(&signable_bytes)
                    }
                    StateTransition::AddressCreditWithdrawal(st) => {
                        st.validate_witnesses(&signable_bytes)
                    }
                    StateTransition::AddressFundingFromAssetLock(st) => {
                        st.validate_witnesses(&signable_bytes)
                    }
                    StateTransition::Shield(st) => st.validate_witnesses(&signable_bytes),
                    // These state transitions don't have address witness validation
                    StateTransition::DataContractCreate(_)
                    | StateTransition::DataContractUpdate(_)
                    | StateTransition::Batch(_)
                    | StateTransition::IdentityCreate(_)
                    | StateTransition::IdentityTopUp(_)
                    | StateTransition::IdentityCreditWithdrawal(_)
                    | StateTransition::IdentityUpdate(_)
                    | StateTransition::IdentityCreditTransfer(_)
                    | StateTransition::MasternodeVote(_)
                    | StateTransition::IdentityCreditTransferToAddresses(_)
                    | StateTransition::ShieldedTransfer(_)
                    | StateTransition::Unshield(_)
                    | StateTransition::ShieldFromAssetLock(_)
                    | StateTransition::ShieldedWithdrawal(_) => {
                        return Ok(SimpleConsensusValidationResult::new());
                    }
                };

                // Add operations to execution context for fee calculation
                add_witness_verification_operations_to_context(
                    &witness_result.operations,
                    execution_context,
                    platform_version,
                );

                Ok(witness_result.validation_result)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::validate_address_witnesses".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

/// Converts address witness verification operations into fee operations and adds them to the context.
fn add_witness_verification_operations_to_context(
    operations: &AddressWitnessVerificationOperations,
    execution_context: &mut StateTransitionExecutionContext,
    _platform_version: &PlatformVersion,
) {
    // Add ECDSA signature verification operations
    // Each verification uses the ECDSA_SECP256K1 key type cost
    if operations.ecdsa_signature_verifications > 0 {
        for _ in 0..operations.ecdsa_signature_verifications {
            execution_context.add_operation(ValidationOperation::SignatureVerification(
                crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation::new(
                    dpp::identity::KeyType::ECDSA_SECP256K1,
                ),
            ));
        }
    }

    // Add hash operations for message digest (sha256d of signable bytes)
    // This is separate from signature verifications because with P2SH optimization,
    // the hash is computed once and reused for all signature verifications
    if operations.message_hash_count > 0 {
        // SHA256 has 64-byte blocks. For double_sha256:
        // - First SHA256: ceil((len + 9) / 64) blocks (9 bytes for length + padding)
        // - Second SHA256: 1 block (32-byte input from first hash)
        let first_sha256_blocks = (operations.signable_bytes_len + 9).div_ceil(64) as u16;
        let second_sha256_blocks = 1u16;
        let blocks_per_double_sha256 = first_sha256_blocks + second_sha256_blocks;
        execution_context.add_operation(ValidationOperation::DoubleSha256(
            operations.message_hash_count * blocks_per_double_sha256,
        ));
    }

    // Add hash operations for pubkey hash verifications
    // Hash160 = RIPEMD160(SHA256(pubkey))
    // - SHA256 of 33-byte compressed pubkey = 1 block
    // - RIPEMD160 of 32-byte SHA256 output = 1 block
    if operations.pubkey_hash_verifications > 0 {
        execution_context.add_operation(ValidationOperation::SingleSha256(
            operations.pubkey_hash_verifications,
        ));
        execution_context.add_operation(ValidationOperation::Ripemd160(
            operations.pubkey_hash_verifications,
        ));
    }

    // Add hash operations for script hash verifications
    // Hash160 = RIPEMD160(SHA256(script))
    // - SHA256 of script (typically ~105 bytes for 2-of-3 multisig) = 2 blocks
    // - RIPEMD160 of 32-byte SHA256 output = 1 block
    if operations.script_hash_verifications > 0 {
        // Script is typically 2 blocks for SHA256 (105 bytes for 2-of-3 multisig)
        execution_context.add_operation(ValidationOperation::SingleSha256(
            operations.script_hash_verifications * 2,
        ));
        execution_context.add_operation(ValidationOperation::Ripemd160(
            operations.script_hash_verifications,
        ));
    }
}

impl StateTransitionHasAddressWitnessValidationV0 for StateTransition {
    fn has_address_witness_validation(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .has_address_witness_validation
        {
            0 => {
                // Preferably use match without wildcard arm to avoid missing cases
                // in the future when new state transitions are added
                let has_address_witness_validation = match self {
                    StateTransition::AddressFundsTransfer(_)
                    | StateTransition::IdentityCreateFromAddresses(_)
                    | StateTransition::IdentityTopUpFromAddresses(_)
                    | StateTransition::AddressCreditWithdrawal(_)
                    | StateTransition::AddressFundingFromAssetLock(_)
                    | StateTransition::Shield(_) => true,
                    StateTransition::DataContractCreate(_)
                    | StateTransition::DataContractUpdate(_)
                    | StateTransition::Batch(_)
                    | StateTransition::IdentityCreate(_)
                    | StateTransition::IdentityTopUp(_)
                    | StateTransition::IdentityCreditWithdrawal(_)
                    | StateTransition::IdentityUpdate(_)
                    | StateTransition::IdentityCreditTransfer(_)
                    | StateTransition::MasternodeVote(_)
                    | StateTransition::IdentityCreditTransferToAddresses(_)
                    | StateTransition::ShieldedTransfer(_)
                    | StateTransition::Unshield(_)
                    | StateTransition::ShieldFromAssetLock(_)
                    | StateTransition::ShieldedWithdrawal(_) => false,
                };

                Ok(has_address_witness_validation)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::has_address_witness_validation".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_data_contract_create_st() -> StateTransition {
        use dpp::tests::fixtures::get_data_contract_fixture;
        use platform_version::TryIntoPlatformVersioned;
        let platform_version = platform_version::version::PlatformVersion::latest();
        let created_data_contract =
            get_data_contract_fixture(None, 1, platform_version.protocol_version);
        let transition: dpp::state_transition::data_contract_create_transition::DataContractCreateTransition =
            created_data_contract.try_into_platform_versioned(platform_version).unwrap();
        transition.into()
    }

    use dpp::version::DefaultForPlatformVersion;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
    use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
    use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
    use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
    use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::AddressFundsTransferTransition;
    use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
    use dpp::state_transition::state_transitions::address_funds::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
    use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
    use dpp::state_transition::shield_transition::ShieldTransition;
    use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;

    mod has_address_witness_validation {
        use super::*;

        #[test]
        fn should_return_true_for_address_based_transitions() {
            let platform_version = &platform_version::version::v8::PLATFORM_V8;
            let transitions: Vec<(&str, StateTransition)> = vec![
                (
                    "AddressFundsTransfer",
                    StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                        AddressFundsTransferTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityCreateFromAddresses",
                    StateTransition::IdentityCreateFromAddresses(
                        IdentityCreateFromAddressesTransition::V0(
                            IdentityCreateFromAddressesTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "IdentityTopUpFromAddresses",
                    StateTransition::IdentityTopUpFromAddresses(
                        IdentityTopUpFromAddressesTransition::V0(
                            IdentityTopUpFromAddressesTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "AddressCreditWithdrawal",
                    StateTransition::AddressCreditWithdrawal(
                        AddressCreditWithdrawalTransition::V0(
                            AddressCreditWithdrawalTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "AddressFundingFromAssetLock",
                    StateTransition::AddressFundingFromAssetLock(
                        AddressFundingFromAssetLockTransition::V0(
                            AddressFundingFromAssetLockTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "Shield",
                    StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
                        inputs: Default::default(),
                        actions: vec![],
                        amount: 0,
                        anchor: [0u8; 32],
                        proof: vec![],
                        binding_signature: [0u8; 64],
                        fee_strategy: vec![],
                        user_fee_increase: 0,
                        input_witnesses: vec![],
                    })),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    st.has_address_witness_validation(platform_version).unwrap(),
                    "expected true for {}",
                    name
                );
            }
        }

        #[test]
        fn should_return_false_for_identity_based_transitions() {
            let platform_version = &platform_version::version::v8::PLATFORM_V8;
            let transitions: Vec<(&str, StateTransition)> = vec![
                ("DataContractCreate", make_data_contract_create_st()),
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
                ),
                (
                    "IdentityCreate",
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityCreditTransfer",
                    StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
                        IdentityCreditTransferTransitionV0::default(),
                    )),
                ),
                (
                    "MasternodeVote",
                    StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                        MasternodeVoteTransitionV0::default(),
                    )),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    !st.has_address_witness_validation(platform_version).unwrap(),
                    "expected false for {}",
                    name
                );
            }
        }
    }

    mod validate_address_witnesses {
        use super::*;

        #[test]
        fn should_return_valid_for_non_address_transition() {
            let platform_version = &platform_version::version::v8::PLATFORM_V8;
            let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default()));
            let mut exec_ctx = StateTransitionExecutionContext::default_for_platform_version(
                platform_version::version::PlatformVersion::latest(),
            )
            .unwrap();
            let result = st
                .validate_address_witnesses(&mut exec_ctx, platform_version)
                .expect("should not error");
            assert!(result.is_valid());
        }
    }
}
