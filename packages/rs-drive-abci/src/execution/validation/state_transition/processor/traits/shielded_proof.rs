use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::reconstruct_and_verify_bundle;
use dpp::consensus::state::state_error::StateError;
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
