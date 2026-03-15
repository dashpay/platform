use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    reconstruct_and_verify_bundle, FLAGS_OUTPUTS_ONLY, FLAGS_SPENDS_AND_OUTPUTS,
};
use dpp::consensus::state::shielded::insufficient_shielded_fee_error::InsufficientShieldedFeeError;
use dpp::consensus::state::state_error::StateError;
use dpp::shielded::SHIELDED_STORAGE_BYTES_PER_ACTION;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

/// A trait for checking whether a state transition requires shielded ZK proof validation.
pub(crate) trait StateTransitionHasShieldedProofValidationV0 {
    /// Returns true if this state transition has a ZK proof that must be verified
    /// before any state reads.
    fn has_shielded_proof_validation(&self) -> bool;

    /// Returns true if this state transition pays fees from the shielded pool's
    /// value_balance and requires minimum fee validation.
    ///
    /// Shield pays fees from transparent address inputs, and ShieldFromAssetLock
    /// pays from the asset lock, so neither goes through shielded fee validation.
    fn has_shielded_minimum_fee_validation(&self) -> bool;
}

/// A trait for validating the ZK proof of a shielded state transition.
///
/// This is a stateless check — it only uses data from the transition itself
/// (actions, flags, value_balance, anchor bytes, proof, binding_signature).
/// No GroveDB reads are needed.
pub(crate) trait StateTransitionShieldedProofValidationV0 {
    fn validate_shielded_proof(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionHasShieldedProofValidationV0 for StateTransition {
    fn has_shielded_proof_validation(&self) -> bool {
        // Note: ShieldFromAssetLock is intentionally excluded. Its proof verification
        // is done inside transform_into_action because a failed proof must penalize
        // the asset lock (via PartiallyUseAssetLockAction). Moving it here would let
        // attackers spam bad proofs without burning their asset lock.
        matches!(
            self,
            StateTransition::Shield(_)
                | StateTransition::ShieldedTransfer(_)
                | StateTransition::Unshield(_)
                | StateTransition::ShieldedWithdrawal(_)
        )
    }

    fn has_shielded_minimum_fee_validation(&self) -> bool {
        // Only spending transitions pay fees from the shielded pool.
        // Shield pays from address inputs; ShieldFromAssetLock pays from the asset lock.
        matches!(
            self,
            StateTransition::ShieldedTransfer(_)
                | StateTransition::Unshield(_)
                | StateTransition::ShieldedWithdrawal(_)
        )
    }
}

/// A trait for validating that a shielded state transition includes sufficient fees.
///
/// The minimum fee is computed dynamically based on the number of actions:
///   min_fee = proof_verification_fee + num_actions × (processing_fee + storage_fee)
///
/// The fee is derived from the public `value_balance` field (no ZK proof execution needed):
/// - ShieldedTransfer: fee = value_balance
/// - Unshield: fee = value_balance - amount
/// - ShieldedWithdrawal: fee = value_balance - amount
/// - Shield: fee paid by transparent address inputs (skipped here)
pub(crate) trait StateTransitionShieldedMinimumFeeValidationV0 {
    fn validate_minimum_shielded_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionShieldedMinimumFeeValidationV0 for StateTransition {
    fn validate_minimum_shielded_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .validate_minimum_shielded_fee
        {
            0 => {
                // Extract the fee and action count from the transition.
                let (fee, num_actions): (i64, usize) = match self {
                    // Shield: fee is paid from transparent address inputs, not from value_balance.
                    StateTransition::Shield(_) => {
                        return Ok(SimpleConsensusValidationResult::new())
                    }
                    // ShieldedTransfer: value_balance (u64) IS the fee.
                    StateTransition::ShieldedTransfer(st) => match st {
                        dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition::V0(v0) => {
                            (v0.value_balance as i64, v0.actions.len())
                        }
                    },
                    // Unshield: fee = value_balance - amount.
                    StateTransition::Unshield(st) => match st {
                        dpp::state_transition::unshield_transition::UnshieldTransition::V0(
                            v0,
                        ) => {
                            // unshielding_amount is the total leaving the pool (fee is validated separately)
                            (v0.unshielding_amount as i64, v0.actions.len())
                        }
                    },
                    StateTransition::ShieldedWithdrawal(st) => match st {
                        dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition::V0(v0) => {
                            (v0.unshielding_amount as i64, v0.actions.len())
                        }
                    },
                    // Other transitions don't go through shielded fee validation.
                    _ => return Ok(SimpleConsensusValidationResult::new()),
                };

                let constants = &platform_version
                    .drive_abci
                    .validation_and_processing
                    .event_constants;

                // Storage fee per action: 312 bytes (280 BulkAppendTree + 32 nullifier)
                // × (storage_disk_usage_credit_per_byte + storage_processing_credit_per_byte)
                let storage_costs = &platform_version.fee_version.storage;
                let storage_fee_per_action = SHIELDED_STORAGE_BYTES_PER_ACTION
                    * (storage_costs.storage_disk_usage_credit_per_byte
                        + storage_costs.storage_processing_credit_per_byte);

                // min_fee = proof_verification_fee + num_actions × (processing_fee + storage_fee)
                let per_action_fee =
                    constants.shielded_per_action_processing_fee + storage_fee_per_action;
                let minimum_shielded_fee =
                    constants.shielded_proof_verification_fee + num_actions as u64 * per_action_fee;

                if (fee as u64) < minimum_shielded_fee {
                    Ok(SimpleConsensusValidationResult::new_with_error(
                        StateError::InsufficientShieldedFeeError(
                            InsufficientShieldedFeeError::new(format!(
                                "shielded transition fee {} is below minimum required fee {} \
                                 ({} proof + {} actions × {} per-action)",
                                fee,
                                minimum_shielded_fee,
                                constants.shielded_proof_verification_fee,
                                num_actions,
                                per_action_fee,
                            )),
                        )
                        .into(),
                    ))
                } else {
                    Ok(SimpleConsensusValidationResult::new())
                }
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::validate_minimum_shielded_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionShieldedProofValidationV0 for StateTransition {
    fn validate_shielded_proof(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .validate_shielded_proof
        {
            0 => {
                let result = match self {
                    StateTransition::Shield(st) => match st {
                        dpp::state_transition::shield_transition::ShieldTransition::V0(v0) => {
                            reconstruct_and_verify_bundle(
                                &v0.actions,
                                FLAGS_OUTPUTS_ONLY,
                                -(v0.amount as i64),
                                &v0.anchor,
                                v0.proof.as_slice(),
                                &v0.binding_signature,
                                &[], // No transparent fields for shield
                            )
                        }
                    },
                    StateTransition::ShieldedTransfer(st) => match st {
                        dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition::V0(v0) => {
                            reconstruct_and_verify_bundle(
                                &v0.actions,
                                FLAGS_SPENDS_AND_OUTPUTS,
                                v0.value_balance as i64,
                                &v0.anchor,
                                v0.proof.as_slice(),
                                &v0.binding_signature,
                                &[], // No transparent fields for shielded transfer
                            )
                        }
                    },
                    StateTransition::Unshield(st) => match st {
                        dpp::state_transition::unshield_transition::UnshieldTransition::V0(v0) => {
                            let mut extra_sighash_data = v0.output_address.to_bytes();
                            extra_sighash_data
                                .extend_from_slice(&v0.unshielding_amount.to_le_bytes());
                            reconstruct_and_verify_bundle(
                                &v0.actions,
                                FLAGS_SPENDS_AND_OUTPUTS,
                                v0.unshielding_amount as i64,
                                &v0.anchor,
                                v0.proof.as_slice(),
                                &v0.binding_signature,
                                &extra_sighash_data,
                            )
                        }
                    },
                    StateTransition::ShieldedWithdrawal(st) => match st {
                        dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition::V0(v0) => {
                            let mut extra_sighash_data =
                                v0.output_script.as_bytes().to_vec();
                            extra_sighash_data
                                .extend_from_slice(&v0.unshielding_amount.to_le_bytes());
                            reconstruct_and_verify_bundle(
                                &v0.actions,
                                FLAGS_SPENDS_AND_OUTPUTS,
                                v0.unshielding_amount as i64,
                                &v0.anchor,
                                v0.proof.as_slice(),
                                &v0.binding_signature,
                                &extra_sighash_data,
                            )
                        }
                    },
                    // ShieldFromAssetLock retains proof verification in transform_into_action
                    // (penalty comes from the asset lock, which is safe)
                    _ => return Ok(SimpleConsensusValidationResult::new()),
                };

                match result {
                    Ok(()) => Ok(SimpleConsensusValidationResult::new()),
                    Err(e) => Ok(SimpleConsensusValidationResult::new_with_error(
                        StateError::InvalidShieldedProofError(e).into(),
                    )),
                }
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::validate_shielded_proof".to_string(),
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

    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
    use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
    use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
    use dpp::state_transition::shield_transition::ShieldTransition;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use dpp::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
    use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
    use dpp::state_transition::unshield_transition::v0::UnshieldTransitionV0;
    use dpp::state_transition::unshield_transition::UnshieldTransition;

    fn make_shield() -> StateTransition {
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
        }))
    }
    fn make_shielded_transfer() -> StateTransition {
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions: vec![],
                value_balance: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
            },
        ))
    }
    fn make_unshield() -> StateTransition {
        StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
            output_address: Default::default(),
            actions: vec![],
            unshielding_amount: 0,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
        }))
    }
    fn make_shield_from_asset_lock() -> StateTransition {
        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
            ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: Default::default(),
                actions: vec![],
                value_balance: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                signature: Default::default(),
            },
        ))
    }
    fn make_shielded_withdrawal() -> StateTransition {
        StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
            ShieldedWithdrawalTransitionV0 {
                actions: vec![],
                unshielding_amount: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                core_fee_per_byte: 0,
                pooling: Default::default(),
                output_script: Default::default(),
            },
        ))
    }

    mod has_shielded_proof_validation {
        use super::*;

        #[test]
        fn should_return_true_for_shield_shielded_transfer_unshield_shielded_withdrawal() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                ("Shield", make_shield()),
                ("ShieldedTransfer", make_shielded_transfer()),
                ("Unshield", make_unshield()),
                ("ShieldedWithdrawal", make_shielded_withdrawal()),
            ];
            for (name, st) in transitions {
                assert!(
                    st.has_shielded_proof_validation(),
                    "expected true for {}",
                    name
                );
            }
        }

        #[test]
        fn should_return_false_for_shield_from_asset_lock_and_non_shielded() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                ("ShieldFromAssetLock", make_shield_from_asset_lock()),
                ("DataContractCreate", make_data_contract_create_st()),
                (
                    "IdentityCreate",
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0::default(),
                    )),
                ),
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    !st.has_shielded_proof_validation(),
                    "expected false for {}",
                    name
                );
            }
        }
    }

    mod has_shielded_minimum_fee_validation {
        use super::*;

        #[test]
        fn should_return_true_for_spending_transitions() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                ("ShieldedTransfer", make_shielded_transfer()),
                ("Unshield", make_unshield()),
                ("ShieldedWithdrawal", make_shielded_withdrawal()),
            ];
            for (name, st) in transitions {
                assert!(
                    st.has_shielded_minimum_fee_validation(),
                    "expected true for {}",
                    name
                );
            }
        }

        #[test]
        fn should_return_false_for_shield_and_non_shielded() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                ("Shield", make_shield()),
                ("ShieldFromAssetLock", make_shield_from_asset_lock()),
                ("DataContractCreate", make_data_contract_create_st()),
            ];
            for (name, st) in transitions {
                assert!(
                    !st.has_shielded_minimum_fee_validation(),
                    "expected false for {}",
                    name
                );
            }
        }
    }

    mod validate_minimum_shielded_fee {
        use super::*;

        #[test]
        fn should_pass_for_non_shielded_transition() {
            let platform_version = &platform_version::version::v9::PLATFORM_V9;
            let st = make_data_contract_create_st();
            let result = st
                .validate_minimum_shielded_fee(platform_version)
                .expect("should not error");
            assert!(result.is_valid());
        }
    }

    mod validate_shielded_proof {
        use super::*;

        #[test]
        fn should_pass_for_non_shielded_transition() {
            let platform_version = &platform_version::version::v9::PLATFORM_V9;
            let st = make_data_contract_create_st();
            let result = st
                .validate_shielded_proof(platform_version)
                .expect("should not error");
            assert!(result.is_valid());
        }
    }
}
