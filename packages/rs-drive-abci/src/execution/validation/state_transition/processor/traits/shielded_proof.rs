use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    reconstruct_and_verify_bundle, FLAGS_OUTPUTS_ONLY, FLAGS_SPENDS_AND_OUTPUTS,
};
use dpp::consensus::basic::identity::InvalidIdentityCreditWithdrawalTransitionAmountError;
use dpp::consensus::basic::state_transition::{
    ShieldedInvalidValueBalanceError, WithdrawalBelowMinAmountError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::shielded::insufficient_shielded_fee_error::InsufficientShieldedFeeError;
use dpp::consensus::state::state_error::StateError;
use dpp::serialization::{PlatformMessageSignable, Signable};
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
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
                | StateTransition::IdentityCreateFromShieldedPool(_)
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
                | StateTransition::IdentityCreateFromShieldedPool(_)
        )
    }
}

/// A trait for validating that a shielded state transition includes sufficient fees.
///
/// The minimum fee is computed dynamically based on the number of actions:
///   min_fee = proof_verification_fee + num_actions × (processing_fee + storage_fee)
///
/// The amount checked against `min_fee` is derived from public fields (no ZK proof
/// execution needed):
/// - ShieldedTransfer: the whole `value_balance` is the fee, so we require
///   `value_balance == min_fee` *exactly*. There is no recipient to absorb an excess, so a
///   variable fee would only burn credits and leak a distinguishing fee fingerprint that
///   breaks shielded uniformity — overpayment is rejected. (Unshield/Withdrawal use `>=`
///   because their excess is the recipient/net amount.)
/// - Unshield / ShieldedWithdrawal: `unshielding_amount` is the TOTAL value leaving the
///   shielded pool (recipient/net amount + fee). The fee actually charged at execution time
///   is carved out of `unshielding_amount` — `compute_shielded_unshield_fee` (the base fee plus
///   the flat `AddBalanceToAddress` output-write storage cost) for Unshield, and
///   `compute_shielded_withdrawal_fee` (the base fee plus the flat Core withdrawal-document
///   storage cost) for ShieldedWithdrawal — and the recipient/net receives
///   `unshielding_amount - fee`. Requiring `unshielding_amount >= min_fee` guarantees that net
///   amount is non-negative.
/// - ShieldedWithdrawal additionally requires the net (`unshielding_amount - min_fee`) to
///   fall within `[min_withdrawal_amount, max_withdrawal_amount]` — the same two-sided range
///   the transparent withdrawal paths enforce — because that net becomes a Core `TxOut`: the
///   dust floor stops a zero/sub-dust queue entry, and the per-transition policy cap stops a
///   single withdrawal from exceeding the protocol's maximum. Unshield has no such range: its
///   net is credited to a platform address, not Core.
/// - Shield: fee paid by transparent address inputs (skipped here).
pub(crate) trait StateTransitionShieldedMinimumFeeValidationV0 {
    fn validate_minimum_shielded_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

/// Which flat shielded fee formula this transition's minimum is computed with.
///
/// The three pool-paid transitions price the same per-action note/nullifier storage and per-bundle
/// ZK compute, but two of them add one flat per-transition storage component on top: Unshield prices
/// its single `AddBalanceToAddress` output write, and ShieldedWithdrawal prices its Core withdrawal
/// document insert. The gate MUST use the SAME formula the SDK builder and the transformer use to
/// carve the fee from the pool, otherwise the validation threshold would drift from the fee actually
/// charged.
#[derive(Clone, Copy)]
enum ShieldedMinFeeKind {
    /// `compute_minimum_shielded_fee` — ShieldedTransfer (the base; no extra write).
    Base,
    /// `compute_shielded_unshield_fee` — Unshield (base + the flat `AddBalanceToAddress` cost).
    Unshield,
    /// `compute_shielded_withdrawal_fee` — ShieldedWithdrawal (base + the flat withdrawal-document cost).
    Withdrawal,
    /// `compute_shielded_identity_create_fee` — IdentityCreateFromShieldedPool (base + the consensus
    /// identity-create floor `identity_create_base_cost + num_keys × identity_key_in_creation_cost`,
    /// the same constants the non-shielded `IdentityCreate` predictor uses, which grows with the key
    /// count). Carries `num_keys` because the fee scales with it, unlike the other (fixed)
    /// per-transition components.
    IdentityCreate { num_keys: usize },
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
                // Destructure per shielded transition type:
                // - `validated_amount`: the amount field this type carries. For ShieldedTransfer
                //   it IS the fee (`value_balance`); for Unshield/ShieldedWithdrawal it is the
                //   GROSS leaving the pool (`unshielding_amount` = recipient/net + fee).
                // - `amount_is_pure_fee`: true only when `validated_amount` is the fee itself
                //   (ShieldedTransfer), so it must equal the minimum exactly — there is no
                //   recipient amount and overpaying is disallowed. False for the gross-carrying
                //   types, where the excess over the fee IS the recipient/net amount.
                // - `min_net_amount`/`max_net_amount`: the allowed range for the derived net
                //   (`validated_amount - min_fee`). `[0, u64::MAX]` (a no-op) for every type
                //   except ShieldedWithdrawal, whose net becomes a Core `TxOut` and must clear
                //   the same `[min_withdrawal_amount, max_withdrawal_amount]` range the
                //   transparent withdrawal paths enforce.
                // - `fee_kind`: which flat fee formula this transition's minimum uses. `Base` for
                //   ShieldedTransfer, `Unshield` for Unshield (adds the `AddBalanceToAddress` output
                //   write cost), `Withdrawal` for ShieldedWithdrawal (adds the Core withdrawal
                //   document cost). MUST match what the builder/transformer carve from the pool.
                let (validated_amount, num_actions, min_net_amount, max_net_amount, amount_is_pure_fee, fee_kind): (i64, usize, u64, u64, bool, ShieldedMinFeeKind) = match self {
                    // Shield: fee is paid from transparent address inputs, not from value_balance.
                    StateTransition::Shield(_) => {
                        return Ok(SimpleConsensusValidationResult::new())
                    }
                    // ShieldedTransfer: value_balance (u64) IS the fee. It writes no extra
                    // per-transition output, so its fee is the `Base` shielded minimum.
                    StateTransition::ShieldedTransfer(st) => match st {
                        dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition::V0(v0) => {
                            (v0.value_balance as i64, v0.actions.len(), 0, u64::MAX, true, ShieldedMinFeeKind::Base)
                        }
                    },
                    // Unshield: `unshielding_amount` is the TOTAL leaving the pool
                    // (recipient/net + fee). We check it against `min_fee` so the net
                    // (`unshielding_amount - compute_shielded_unshield_fee`) credited to
                    // the recipient at execution time is non-negative. The net is credited
                    // to a platform address (not Core), so no withdrawal range applies. It
                    // writes a single `AddBalanceToAddress` output, so its fee is the `Unshield`
                    // flavor (base + that write's flat storage cost).
                    StateTransition::Unshield(st) => match st {
                        dpp::state_transition::unshield_transition::UnshieldTransition::V0(
                            v0,
                        ) => (v0.unshielding_amount as i64, v0.actions.len(), 0, u64::MAX, false, ShieldedMinFeeKind::Unshield),
                    },
                    // ShieldedWithdrawal: the net (`unshielding_amount - min_fee`) becomes a
                    // Core `TxOut`, so it must fall within the same
                    // `[min_withdrawal_amount, max_withdrawal_amount]` range the transparent
                    // withdrawal paths enforce (dust floor and the per-transition policy cap).
                    // It is the ONLY transition that also inserts a Core withdrawal document, so
                    // its fee is the `Withdrawal` flavor: it must price that document write (via
                    // `compute_shielded_withdrawal_fee`), not just the base shielded fee.
                    StateTransition::ShieldedWithdrawal(st) => match st {
                        dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition::V0(v0) => {
                            (
                                v0.unshielding_amount as i64,
                                v0.actions.len(),
                                platform_version.system_limits.min_withdrawal_amount,
                                platform_version.system_limits.max_withdrawal_amount,
                                false,
                                ShieldedMinFeeKind::Withdrawal,
                            )
                        }
                    },
                    // IdentityCreateFromShieldedPool: `denomination` is the TOTAL leaving the pool
                    // (new-identity balance + fee). This is a CHEAP early floor — it rejects a
                    // denomination that cannot even cover the consensus identity-create minimum
                    // (`identity_create_base_cost + num_keys × identity_key_in_creation_cost`, the
                    // same constants the non-shielded `IdentityCreate` predictor uses) plus the
                    // shielded compute fee, before the expensive proof verification + metering. The
                    // AUTHORITATIVE non-negative-balance check (`denomination >= metered + compute`)
                    // runs later in `validate_fees_of_event`. It is NOT pure fee (`>=` model); the
                    // exact `value_balance == denomination` equality is enforced by the proof verifier
                    // (which passes `value_balance = denomination`). The fee scales with the key
                    // count, so the `IdentityCreate` flavor carries `num_keys`.
                    StateTransition::IdentityCreateFromShieldedPool(st) => match st {
                        dpp::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition::V0(v0) => {
                            (
                                v0.denomination as i64,
                                v0.actions.len(),
                                0,
                                u64::MAX,
                                false,
                                ShieldedMinFeeKind::IdentityCreate { num_keys: v0.public_keys.len() },
                            )
                        }
                    },
                    // Other transitions don't go through shielded fee validation.
                    _ => return Ok(SimpleConsensusValidationResult::new()),
                };

                // Defensive overflow guard. `value_balance`/`unshielding_amount` are bounded to
                // `<= i64::MAX` by basic structure validation, which runs (with a hard
                // early-return) before this stage, so the `as i64` casts above are non-negative
                // for validated input. Re-check here so a future reordering or a direct caller
                // cannot slip a value `> i64::MAX` past as a wrapped-negative `i64` (which would
                // then wrap back to a huge `u64` and sail past the min-fee check below).
                if validated_amount < 0 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        BasicError::ShieldedInvalidValueBalanceError(
                            ShieldedInvalidValueBalanceError::new(
                                "shielded value_balance/unshielding_amount exceeds the maximum \
                                 allowed value (i64::MAX)"
                                    .to_string(),
                            ),
                        )
                        .into(),
                    ));
                }

                let constants = &platform_version
                    .drive_abci
                    .validation_and_processing
                    .event_constants;

                // Single source of truth for the consensus fee formula. Each pool-paid transition's
                // SDK builder and transformer carve the fee with the SAME function this gate uses,
                // so the validation threshold here can never drift from the fee actually charged:
                // - ShieldedTransfer carves `compute_minimum_shielded_fee` (the base).
                // - Unshield carves `compute_shielded_unshield_fee` (= the base PLUS the flat
                //   `AddBalanceToAddress` output-write storage cost).
                // - ShieldedWithdrawal carves `compute_shielded_withdrawal_fee` (= the base PLUS the
                //   flat Core withdrawal-document storage cost).
                // All three use the same checked computations the carving sites use. See
                // `dpp::shielded::compute_minimum_shielded_fee` /
                // `dpp::shielded::compute_shielded_unshield_fee` /
                // `dpp::shielded::compute_shielded_withdrawal_fee`.
                let minimum_shielded_fee = match fee_kind {
                    ShieldedMinFeeKind::Base => {
                        dpp::shielded::compute_minimum_shielded_fee(num_actions, platform_version)?
                    }
                    ShieldedMinFeeKind::Unshield => {
                        dpp::shielded::compute_shielded_unshield_fee(num_actions, platform_version)?
                    }
                    ShieldedMinFeeKind::Withdrawal => {
                        dpp::shielded::compute_shielded_withdrawal_fee(
                            num_actions,
                            platform_version,
                        )?
                    }
                    ShieldedMinFeeKind::IdentityCreate { num_keys } => {
                        dpp::shielded::compute_shielded_identity_create_fee(
                            num_actions,
                            num_keys,
                            platform_version,
                        )?
                    }
                };

                if (validated_amount as u64) < minimum_shielded_fee {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        StateError::InsufficientShieldedFeeError(
                            InsufficientShieldedFeeError::new(format!(
                                "shielded transition amount {} is below the minimum required fee \
                                 {} ({} proof-verification + {} actions)",
                                validated_amount,
                                minimum_shielded_fee,
                                constants.shielded_proof_verification_fee,
                                num_actions,
                            )),
                        )
                        .into(),
                    ));
                }

                // ShieldedTransfer: `value_balance` IS the entire fee (there is no recipient
                // amount), so it must equal the minimum exactly. Allowing `value_balance >
                // min_fee` would let a transfer overpay for no benefit and leak a fee
                // fingerprint that breaks shielded uniformity — reject it. Unshield/Withdrawal
                // are exempt: their excess over `min_fee` is the recipient/net amount.
                if amount_is_pure_fee && (validated_amount as u64) > minimum_shielded_fee {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        BasicError::ShieldedInvalidValueBalanceError(
                            ShieldedInvalidValueBalanceError::new(format!(
                                "shielded transfer value_balance {} must equal the minimum \
                                 shielded fee {} exactly ({} proof-verification + {} actions); \
                                 overpayment is not allowed",
                                validated_amount,
                                minimum_shielded_fee,
                                constants.shielded_proof_verification_fee,
                                num_actions,
                            )),
                        )
                        .into(),
                    ));
                }

                // For ShieldedWithdrawal, the net value leaving to Core
                // (`unshielding_amount - min_fee`) must fall within the same
                // `[min_withdrawal_amount, max_withdrawal_amount]` range the transparent
                // withdrawal paths enforce — the dust floor AND the per-transition policy
                // cap. For the other shielded paths the range is `[0, u64::MAX]`, so both
                // checks are no-ops.
                // Safe: `validated_amount as u64 >= minimum_shielded_fee` was just checked above.
                let net_amount = (validated_amount as u64) - minimum_shielded_fee;
                if net_amount < min_net_amount {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        BasicError::WithdrawalBelowMinAmountError(
                            WithdrawalBelowMinAmountError::new(
                                net_amount,
                                min_net_amount,
                                max_net_amount,
                            ),
                        )
                        .into(),
                    ));
                }
                if net_amount > max_net_amount {
                    // Over the per-transition withdrawal cap. Use the same range error the
                    // transparent withdrawal path uses so the rejection reason is accurate
                    // (an over-max amount is not "below min").
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidIdentityCreditWithdrawalTransitionAmountError::new(
                            net_amount,
                            min_net_amount,
                            max_net_amount,
                        )
                        .into(),
                    ));
                }

                Ok(SimpleConsensusValidationResult::new())
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
                // `IdentityCreateFromShieldedPool` is the only shielded transition carrying separate
                // per-key proof-of-possession signatures that are NOT covered by the Orchard proof
                // (they sign the platform signable bytes, and only id+denomination+keys — not the PoP
                // sigs — are bound into `extra_sighash_data`). Validate the CHEAP key structure +
                // per-key PoP here, BEFORE the expensive Halo 2 bundle verification, so a relayer
                // who flips a PoP byte on an observed transition is rejected without the node paying
                // for proof verification (DoS hardening). Same `signable_bytes` the transformer uses.
                if let StateTransition::IdentityCreateFromShieldedPool(st) = self {
                    let IdentityCreateFromShieldedPoolTransition::V0(v0) = st;

                    let key_structure_result =
                        IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                            &v0.public_keys,
                            true,
                            platform_version,
                        )?;
                    if !key_structure_result.is_valid() {
                        return Ok(key_structure_result);
                    }

                    let signable_bytes = self.signable_bytes()?;
                    for key in v0.public_keys.iter() {
                        let pop_result = signable_bytes.as_slice().verify_signature(
                            key.key_type(),
                            key.data().as_slice(),
                            key.signature().as_slice(),
                        );
                        if !pop_result.is_valid() {
                            return Ok(pop_result);
                        }
                    }
                }

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
                            let extra_sighash_data = dpp::shielded::unshield_extra_sighash_data(
                                &v0.output_address.to_bytes(),
                                v0.unshielding_amount,
                                platform_version,
                            )?;
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
                            let extra_sighash_data =
                                dpp::shielded::shielded_withdrawal_extra_sighash_data(
                                    v0.output_script.as_bytes(),
                                    v0.unshielding_amount,
                                    v0.core_fee_per_byte,
                                    v0.pooling,
                                    platform_version,
                                )?;
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
                    StateTransition::IdentityCreateFromShieldedPool(st) => match st {
                        dpp::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition::V0(v0) => {
                            // Bind the new identity id + denomination + FULL public-key set into the
                            // Orchard sighash so the bundle cannot be redirected to a different
                            // identity/keys (the surplus_output binding analog). The id is re-derived
                            // from the spend nullifiers — the canonical value — so the binding holds
                            // regardless of any (separately-validated) wire `identity_id`.
                            let identity_id =
                                dpp::state_transition::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions(&v0.actions)
                                    .to_buffer();
                            let extra_sighash_data =
                                dpp::shielded::identity_create_from_shielded_extra_sighash_data(
                                    &identity_id,
                                    v0.denomination,
                                    &v0.send_to_address_on_creation_failure,
                                    &v0.public_keys,
                                    platform_version,
                                )?;
                            // value_balance = denomination EXACTLY (the ShieldedTransfer exact-equality
                            // model): the binding signature proves the value commitments sum to exactly
                            // the denomination leaving the pool.
                            reconstruct_and_verify_bundle(
                                &v0.actions,
                                FLAGS_SPENDS_AND_OUTPUTS,
                                v0.denomination as i64,
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
                surplus_output: None,
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
    fn make_identity_create_from_shielded_pool() -> StateTransition {
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
        StateTransition::IdentityCreateFromShieldedPool(
            IdentityCreateFromShieldedPoolTransition::V0(
                IdentityCreateFromShieldedPoolTransitionV0 {
                    public_keys: vec![],
                    denomination: 0,
                    actions: vec![],
                    anchor: [0u8; 32],
                    proof: vec![],
                    binding_signature: [0u8; 64],
                    send_to_address_on_creation_failure: dpp::address_funds::PlatformAddress::P2pkh(
                        [0u8; 20],
                    ),
                    identity_id: Default::default(),
                },
            ),
        )
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
                (
                    "IdentityCreateFromShieldedPool",
                    make_identity_create_from_shielded_pool(),
                ),
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
                (
                    "IdentityCreateFromShieldedPool",
                    make_identity_create_from_shielded_pool(),
                ),
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
        use dpp::consensus::ConsensusError;

        /// A ShieldedWithdrawal transition (no actions) with the given gross amount.
        fn shielded_withdrawal_with_amount(unshielding_amount: u64) -> StateTransition {
            StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
                ShieldedWithdrawalTransitionV0 {
                    actions: vec![],
                    unshielding_amount,
                    anchor: [0u8; 32],
                    proof: vec![],
                    binding_signature: [0u8; 64],
                    core_fee_per_byte: 0,
                    pooling: Default::default(),
                    output_script: Default::default(),
                },
            ))
        }

        #[test]
        fn should_pass_for_non_shielded_transition() {
            let platform_version = &platform_version::version::v9::PLATFORM_V9;
            let st = make_data_contract_create_st();
            let result = st
                .validate_minimum_shielded_fee(platform_version)
                .expect("should not error");
            assert!(result.is_valid());
        }

        #[test]
        fn should_reject_shielded_withdrawal_with_net_below_min_withdrawal_amount() {
            let platform_version = PlatformVersion::latest();
            // ShieldedWithdrawal validation carves `compute_shielded_withdrawal_fee` (base +
            // document cost), so the net the gate checks is `unshielding_amount - withdrawal_fee`.
            // Use the SAME fee here so the constructed gross lands the net exactly at the
            // withdrawal-range boundary under test.
            let min_fee = dpp::shielded::compute_shielded_withdrawal_fee(0, platform_version)
                .expect("fee computation should not overflow");
            let min_withdrawal = platform_version.system_limits.min_withdrawal_amount;
            // net = min_withdrawal_amount - 1 → just below the Core dust floor.
            let st = shielded_withdrawal_with_amount(min_fee + min_withdrawal - 1);
            let result = st
                .validate_minimum_shielded_fee(platform_version)
                .expect("should not error");
            assert!(!result.is_valid());
            assert!(
                matches!(
                    result.errors.first(),
                    Some(ConsensusError::BasicError(
                        BasicError::WithdrawalBelowMinAmountError(_)
                    ))
                ),
                "below-min must reject with WithdrawalBelowMinAmountError; got {:?}",
                result.errors
            );
        }

        #[test]
        fn should_accept_shielded_withdrawal_with_net_at_min_withdrawal_amount() {
            let platform_version = PlatformVersion::latest();
            // ShieldedWithdrawal validation carves `compute_shielded_withdrawal_fee` (base +
            // document cost), so the net the gate checks is `unshielding_amount - withdrawal_fee`.
            // Use the SAME fee here so the constructed gross lands the net exactly at the
            // withdrawal-range boundary under test.
            let min_fee = dpp::shielded::compute_shielded_withdrawal_fee(0, platform_version)
                .expect("fee computation should not overflow");
            let min_withdrawal = platform_version.system_limits.min_withdrawal_amount;
            // net = min_withdrawal_amount exactly → at the floor, accepted.
            let st = shielded_withdrawal_with_amount(min_fee + min_withdrawal);
            let result = st
                .validate_minimum_shielded_fee(platform_version)
                .expect("should not error");
            assert!(
                result.is_valid(),
                "a withdrawal whose net equals min_withdrawal_amount must be accepted"
            );
        }

        #[test]
        fn should_reject_shielded_withdrawal_with_net_above_max_withdrawal_amount() {
            let platform_version = PlatformVersion::latest();
            // ShieldedWithdrawal validation carves `compute_shielded_withdrawal_fee` (base +
            // document cost), so the net the gate checks is `unshielding_amount - withdrawal_fee`.
            // Use the SAME fee here so the constructed gross lands the net exactly at the
            // withdrawal-range boundary under test.
            let min_fee = dpp::shielded::compute_shielded_withdrawal_fee(0, platform_version)
                .expect("fee computation should not overflow");
            let max = platform_version.system_limits.max_withdrawal_amount;
            // net = max_withdrawal_amount + 1 → just over the per-transition policy cap.
            let st = shielded_withdrawal_with_amount(min_fee + max + 1);
            let result = st
                .validate_minimum_shielded_fee(platform_version)
                .expect("should not error");
            assert!(!result.is_valid());
            // Must reject with the amount-RANGE error, not the below-min error — locks in the
            // "over-max is not below-min" reason accuracy the cap introduced.
            assert!(
                matches!(
                    result.errors.first(),
                    Some(ConsensusError::BasicError(
                        BasicError::InvalidIdentityCreditWithdrawalTransitionAmountError(_)
                    ))
                ),
                "over-max must reject with InvalidIdentityCreditWithdrawalTransitionAmountError; got {:?}",
                result.errors
            );
        }

        #[test]
        fn should_accept_shielded_withdrawal_with_net_at_max_withdrawal_amount() {
            let platform_version = PlatformVersion::latest();
            // ShieldedWithdrawal validation carves `compute_shielded_withdrawal_fee` (base +
            // document cost), so the net the gate checks is `unshielding_amount - withdrawal_fee`.
            // Use the SAME fee here so the constructed gross lands the net exactly at the
            // withdrawal-range boundary under test.
            let min_fee = dpp::shielded::compute_shielded_withdrawal_fee(0, platform_version)
                .expect("fee computation should not overflow");
            let max = platform_version.system_limits.max_withdrawal_amount;
            // net = max_withdrawal_amount exactly → at the cap, accepted.
            let st = shielded_withdrawal_with_amount(min_fee + max);
            let result = st
                .validate_minimum_shielded_fee(platform_version)
                .expect("should not error");
            assert!(
                result.is_valid(),
                "a withdrawal whose net equals max_withdrawal_amount must be accepted"
            );
        }

        #[test]
        fn should_reject_amount_exceeding_i64_max_via_guard() {
            let platform_version = PlatformVersion::latest();
            // `unshielding_amount > i64::MAX` wraps to a negative i64 in the validator's cast;
            // the defensive `fee < 0` guard must reject it (rather than wrapping back to a huge
            // u64 and sailing past the min-fee check).
            let st = shielded_withdrawal_with_amount((i64::MAX as u64) + 1);
            let result = st
                .validate_minimum_shielded_fee(platform_version)
                .expect("should not error");
            assert!(!result.is_valid());
            assert!(
                matches!(
                    result.errors.first(),
                    Some(ConsensusError::BasicError(
                        BasicError::ShieldedInvalidValueBalanceError(_)
                    ))
                ),
                "amount > i64::MAX must be rejected by the fee<0 guard; got {:?}",
                result.errors
            );
        }

        /// Build an `IdentityCreateFromShieldedPool` with `num_actions` actions, `num_keys` keys, and
        /// the given `denomination` (the min-fee gate only reads those three).
        fn identity_create_from_shielded_pool(
            denomination: u64,
            num_actions: usize,
            num_keys: usize,
        ) -> StateTransition {
            use dpp::identity::{KeyType, Purpose, SecurityLevel};
            use dpp::platform_value::BinaryData;
            use dpp::shielded::SerializedAction;
            use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
            use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
            use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
            use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
            let actions = (0..num_actions as u8)
                .map(|i| SerializedAction {
                    nullifier: [i; 32],
                    rk: [0u8; 32],
                    cmx: [0u8; 32],
                    encrypted_note: vec![0u8; 216],
                    cv_net: [0u8; 32],
                    spend_auth_sig: [0u8; 64],
                })
                .collect();
            let public_keys = (0..num_keys as u32)
                .map(|i| {
                    IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
                        id: i,
                        key_type: KeyType::ECDSA_SECP256K1,
                        purpose: Purpose::AUTHENTICATION,
                        security_level: SecurityLevel::MASTER,
                        contract_bounds: None,
                        read_only: false,
                        data: BinaryData::new(vec![i as u8; 33]),
                        signature: BinaryData::new(vec![]),
                    })
                })
                .collect();
            StateTransition::IdentityCreateFromShieldedPool(
                IdentityCreateFromShieldedPoolTransition::V0(
                    IdentityCreateFromShieldedPoolTransitionV0 {
                        public_keys,
                        denomination,
                        actions,
                        anchor: [0u8; 32],
                        proof: vec![],
                        binding_signature: [0u8; 64],
                        send_to_address_on_creation_failure:
                            dpp::address_funds::PlatformAddress::P2pkh([0u8; 20]),
                        identity_id: Default::default(),
                    },
                ),
            )
        }

        #[test]
        fn should_reject_identity_create_denomination_below_min_fee() {
            let platform_version = PlatformVersion::latest();
            let (num_actions, num_keys) = (2usize, 1usize);
            let min_fee = dpp::shielded::compute_shielded_identity_create_fee(
                num_actions,
                num_keys,
                platform_version,
            )
            .expect("fee");
            let st = identity_create_from_shielded_pool(min_fee - 1, num_actions, num_keys);
            let result = st
                .validate_minimum_shielded_fee(platform_version)
                .expect("no error");
            assert!(!result.is_valid());
            assert!(
                matches!(
                    result.errors.first(),
                    Some(ConsensusError::StateError(
                        StateError::InsufficientShieldedFeeError(_)
                    ))
                ),
                "a denomination below the min fee must reject with InsufficientShieldedFeeError; got {:?}",
                result.errors
            );
        }

        #[test]
        fn should_accept_identity_create_denomination_at_min_fee() {
            let platform_version = PlatformVersion::latest();
            let (num_actions, num_keys) = (2usize, 1usize);
            let min_fee = dpp::shielded::compute_shielded_identity_create_fee(
                num_actions,
                num_keys,
                platform_version,
            )
            .expect("fee");
            let st = identity_create_from_shielded_pool(min_fee, num_actions, num_keys);
            assert!(
                st.validate_minimum_shielded_fee(platform_version)
                    .expect("no error")
                    .is_valid(),
                "a denomination equal to the min fee must be accepted"
            );
        }

        #[test]
        fn should_scale_identity_create_min_fee_with_key_count() {
            let platform_version = PlatformVersion::latest();
            let num_actions = 2usize;
            let one_key_fee = dpp::shielded::compute_shielded_identity_create_fee(
                num_actions,
                1,
                platform_version,
            )
            .expect("fee");
            let five_key_fee = dpp::shielded::compute_shielded_identity_create_fee(
                num_actions,
                5,
                platform_version,
            )
            .expect("fee");
            assert!(five_key_fee > one_key_fee, "more keys must cost more");
            // A 1-key-sized denomination must be REJECTED for a 5-key identity (the fee scaled up).
            let st = identity_create_from_shielded_pool(one_key_fee, num_actions, 5);
            assert!(
                !st.validate_minimum_shielded_fee(platform_version)
                    .expect("no error")
                    .is_valid(),
                "a 1-key-sized denomination must be rejected once the identity has 5 keys"
            );
            // ...and accepted once the denomination covers the scaled fee.
            let st_ok = identity_create_from_shielded_pool(five_key_fee, num_actions, 5);
            assert!(st_ok
                .validate_minimum_shielded_fee(platform_version)
                .expect("no error")
                .is_valid());
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
