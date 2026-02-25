use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::reconstruct_and_verify_bundle;
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
        matches!(
            self,
            StateTransition::Shield(_)
                | StateTransition::ShieldedTransfer(_)
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
                            let fee = if v0.value_balance <= 0 || (v0.value_balance as u64) <= v0.amount {
                                0
                            } else {
                                (v0.value_balance as u64 - v0.amount) as i64
                            };
                            (fee, v0.actions.len())
                        }
                    },
                    // ShieldedWithdrawal: fee = value_balance - amount.
                    StateTransition::ShieldedWithdrawal(st) => match st {
                        dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition::V0(v0) => {
                            let fee = if v0.value_balance <= 0 || (v0.value_balance as u64) <= v0.amount {
                                0
                            } else {
                                (v0.value_balance as u64 - v0.amount) as i64
                            };
                            (fee, v0.actions.len())
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
                                v0.flags,
                                v0.value_balance,
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
                                v0.flags,
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
                                .extend_from_slice(&v0.amount.to_le_bytes());
                            reconstruct_and_verify_bundle(
                                &v0.actions,
                                v0.flags,
                                v0.value_balance,
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
                                .extend_from_slice(&v0.amount.to_le_bytes());
                            reconstruct_and_verify_bundle(
                                &v0.actions,
                                v0.flags,
                                v0.value_balance,
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
