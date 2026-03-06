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
                    | StateTransition::IdentityCreditTransferToAddresses(_) => {
                        return Ok(SimpleConsensusValidationResult::new());
                    }
                    StateTransition::Shield(_)
                    | StateTransition::ShieldedTransfer(_)
                    | StateTransition::Unshield(_)
                    | StateTransition::ShieldFromAssetLock(_)
                    | StateTransition::ShieldedWithdrawal(_) => {
                        todo!("shielded transitions not yet implemented")
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
                    | StateTransition::AddressFundingFromAssetLock(_) => true,
                    StateTransition::DataContractCreate(_)
                    | StateTransition::DataContractUpdate(_)
                    | StateTransition::Batch(_)
                    | StateTransition::IdentityCreate(_)
                    | StateTransition::IdentityTopUp(_)
                    | StateTransition::IdentityCreditWithdrawal(_)
                    | StateTransition::IdentityUpdate(_)
                    | StateTransition::IdentityCreditTransfer(_)
                    | StateTransition::MasternodeVote(_)
                    | StateTransition::IdentityCreditTransferToAddresses(_) => false,
                    StateTransition::Shield(_)
                    | StateTransition::ShieldedTransfer(_)
                    | StateTransition::Unshield(_)
                    | StateTransition::ShieldFromAssetLock(_)
                    | StateTransition::ShieldedWithdrawal(_) => {
                        todo!("shielded transitions not yet implemented")
                    }
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
