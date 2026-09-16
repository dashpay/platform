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
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use dpp::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_shield_transition::v0::v0_methods::TokenShieldTransitionV0Methods;
use dpp::state_transition::batch_transition::token_shielded_transfer_transition::v0::v0_methods::TokenShieldedTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::token_unshield_transition::v0::v0_methods::TokenUnshieldTransitionV0Methods;
use dpp::state_transition::batch_transition::token_mint_to_pool_transition::v0::v0_methods::TokenMintToPoolTransitionV0Methods;
use dpp::state_transition::batch_transition::token_burn_from_pool_transition::v0::v0_methods::TokenBurnFromPoolTransitionV0Methods;
use dpp::state_transition::batch_transition::token_claim_to_pool_transition::v0::v0_methods::TokenClaimToPoolTransitionV0Methods;
use dpp::state_transition::batch_transition::token_direct_purchase_to_pool_transition::v0::v0_methods::TokenDirectPurchaseToPoolTransitionV0Methods;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use dpp::tokens::token_payment_info::methods::v0::TokenPaymentInfoMethodsV0;
use dpp::tokens::token_payment_info::v1::v1_accessors::TokenPaymentInfoAccessorsV1;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::{StateTransition, StateTransitionOwned};
use dpp::util::hash::hash_single;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

/// The (identity, nonce) pair CheckTx's `CheckTxProofVerifier` admits Orchard proof work under
/// for identity-signed shielded transitions, so an identity cannot start unbounded verification
/// attempts for one nonce.
///
/// `ShieldFromIdentity` keys on the identity nonce. A batch carrying token shielded transitions
/// keys on its identity CONTRACT nonce: the batch is replay-protected by that nonce, so it is the
/// counter whose committed value bounds the attempts. The two nonce spaces are unrelated, so the
/// contract-keyed form derives its own cache identity from `(identity_id, contract_id)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShieldedProofAdmissionKey {
    /// Keyed on the identity nonce (`ShieldFromIdentity`).
    Identity { identity_id: [u8; 32], nonce: u64 },
    /// Keyed on the identity contract nonce (a batch with token shielded transitions).
    IdentityContract {
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        nonce: u64,
    },
}

impl ShieldedProofAdmissionKey {
    /// The 32-byte key the verifier tracks attempts under.
    pub(crate) fn cache_key(&self) -> [u8; 32] {
        match self {
            ShieldedProofAdmissionKey::Identity { identity_id, .. } => *identity_id,
            ShieldedProofAdmissionKey::IdentityContract {
                identity_id,
                contract_id,
                ..
            } => {
                let mut preimage = Vec::with_capacity(64);
                preimage.extend_from_slice(identity_id);
                preimage.extend_from_slice(contract_id);
                hash_single(preimage)
            }
        }
    }

    /// The nonce the transition attempts.
    pub(crate) fn nonce(&self) -> u64 {
        match self {
            ShieldedProofAdmissionKey::Identity { nonce, .. }
            | ShieldedProofAdmissionKey::IdentityContract { nonce, .. } => *nonce,
        }
    }
}

/// A trait for checking whether a state transition requires shielded ZK proof validation.
pub(crate) trait StateTransitionHasShieldedProofValidationV0 {
    /// Returns true if this state transition has a ZK proof that must be verified
    /// during validation.
    fn has_shielded_proof_validation(&self) -> bool;

    /// Returns the number of Orchard actions whose proof work must be admitted.
    fn shielded_proof_action_count(&self) -> usize;

    /// Returns the admission key that must not start repeated Orchard verification attempts
    /// in CheckTx: `ShieldFromIdentity` (identity nonce) and batches carrying token shielded
    /// transitions (identity contract nonce). Pool-paid shielded spends are already
    /// replay-protected by their nullifiers and return `None`.
    fn shielded_proof_identity_nonce_admission_key(&self) -> Option<ShieldedProofAdmissionKey>;

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
        // ShieldFromIdentity is excluded from the shared processor step for the same
        // reason: block processing verifies it in its transform and turns a failure
        // into a paid, nonce-consuming penalty. CheckTx invokes
        // `validate_shielded_proof` explicitly after full fee admission.
        matches!(
            self,
            StateTransition::Shield(_)
                | StateTransition::ShieldedTransfer(_)
                | StateTransition::IdentityTopUpFromShieldedPool(_)
                | StateTransition::Unshield(_)
                | StateTransition::ShieldedWithdrawal(_)
                | StateTransition::IdentityCreateFromShieldedPool(_)
                | StateTransition::TokenShieldedTransferWithShieldedFee(_)
                | StateTransition::TokenUnshieldWithShieldedFee(_)
                | StateTransition::TokenPurchaseFromShieldedPool(_)
        )
    }

    fn shielded_proof_action_count(&self) -> usize {
        match self {
            StateTransition::Shield(st) => match st {
                dpp::state_transition::shield_transition::ShieldTransition::V0(v0) => {
                    v0.actions.len()
                }
            },
            StateTransition::ShieldFromIdentity(st) => match st {
                ShieldFromIdentityTransition::V0(v0) => v0.actions.len(),
            },
            StateTransition::ShieldedTransfer(st) => match st {
                dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition::V0(v0) => {
                    v0.actions.len()
                }
            },
            StateTransition::Unshield(st) => match st {
                dpp::state_transition::unshield_transition::UnshieldTransition::V0(v0) => {
                    v0.actions.len()
                }
            },
            StateTransition::IdentityTopUpFromShieldedPool(st) => match st {
                IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.actions.len(),
            },
            // Two bundles are verified: both counts are admitted.
            StateTransition::TokenShieldedTransferWithShieldedFee(st) => match st {
                TokenShieldedTransferWithShieldedFeeTransition::V0(v0) => {
                    v0.token_actions.len() + v0.fee_actions.len()
                }
            },
            StateTransition::TokenUnshieldWithShieldedFee(st) => match st {
                TokenUnshieldWithShieldedFeeTransition::V0(v0) => {
                    v0.token_actions.len() + v0.fee_actions.len()
                }
            },
            StateTransition::TokenPurchaseFromShieldedPool(st) => match st {
                TokenPurchaseFromShieldedPoolTransition::V0(v0) => {
                    v0.token_actions.len() + v0.fee_actions.len()
                }
            },
            StateTransition::ShieldedWithdrawal(st) => match st {
                dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition::V0(v0) => {
                    v0.actions.len()
                }
            },
            StateTransition::IdentityCreateFromShieldedPool(st) => match st {
                IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.actions.len(),
            },
            StateTransition::ShieldFromAssetLock(st) => match st {
                dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition::V0(v0) => {
                    v0.actions.len()
                }
            },
            StateTransition::Batch(batch) => batch
                .transitions_iter()
                .map(|transition| match transition {
                    BatchedTransitionRef::Token(TokenTransition::Shield(t)) => t.actions().len(),
                    BatchedTransitionRef::Token(TokenTransition::Unshield(t)) => t.actions().len(),
                    BatchedTransitionRef::Token(TokenTransition::ShieldedTransfer(t)) => {
                        t.actions().len()
                    }
                    BatchedTransitionRef::Token(TokenTransition::MintToPool(t)) => {
                        t.actions().len()
                    }
                    BatchedTransitionRef::Token(TokenTransition::BurnFromPool(t)) => {
                        t.actions().len()
                    }
                    BatchedTransitionRef::Token(TokenTransition::ClaimToPool(t)) => {
                        t.actions().len()
                    }
                    BatchedTransitionRef::Token(TokenTransition::DirectPurchaseToPool(t)) => {
                        t.actions().len()
                    }
                    BatchedTransitionRef::Document(document_transition) => document_transition
                        .base()
                        .token_payment_info_ref()
                        .as_ref()
                        .and_then(|info| info.shielded_payment())
                        .map(|payment| payment.actions.len())
                        .unwrap_or(0),
                    _ => 0,
                })
                .sum(),
            _ => 0,
        }
    }

    fn shielded_proof_identity_nonce_admission_key(&self) -> Option<ShieldedProofAdmissionKey> {
        match self {
            StateTransition::ShieldFromIdentity(ShieldFromIdentityTransition::V0(v0)) => {
                Some(ShieldedProofAdmissionKey::Identity {
                    identity_id: v0.identity_id.to_buffer(),
                    nonce: v0.nonce,
                })
            }
            StateTransition::Batch(batch) => {
                let owner_id = batch.owner_id().to_buffer();
                batch
                    .transitions_iter()
                    .find_map(|transition| match transition {
                        BatchedTransitionRef::Token(
                            token_transition @ (TokenTransition::Shield(_)
                            | TokenTransition::Unshield(_)
                            | TokenTransition::ShieldedTransfer(_)
                            | TokenTransition::MintToPool(_)
                            | TokenTransition::BurnFromPool(_)
                            | TokenTransition::ClaimToPool(_)
                            | TokenTransition::DirectPurchaseToPool(_)),
                        ) => Some(ShieldedProofAdmissionKey::IdentityContract {
                            identity_id: owner_id,
                            contract_id: token_transition.data_contract_id().to_buffer(),
                            nonce: token_transition.identity_contract_nonce(),
                        }),
                        BatchedTransitionRef::Document(document_transition)
                            if document_transition
                                .base()
                                .token_payment_info_ref()
                                .as_ref()
                                .is_some_and(|info| info.shielded_payment().is_some()) =>
                        {
                            Some(ShieldedProofAdmissionKey::IdentityContract {
                                identity_id: owner_id,
                                contract_id: document_transition
                                    .base()
                                    .data_contract_id()
                                    .to_buffer(),
                                nonce: document_transition.base().identity_contract_nonce(),
                            })
                        }
                        _ => None,
                    })
            }
            _ => None,
        }
    }

    fn has_shielded_minimum_fee_validation(&self) -> bool {
        // Only spending transitions pay fees from the shielded pool.
        // Shield pays from address inputs; ShieldFromAssetLock pays from the asset lock.
        matches!(
            self,
            StateTransition::ShieldedTransfer(_)
                | StateTransition::IdentityTopUpFromShieldedPool(_)
                | StateTransition::Unshield(_)
                | StateTransition::ShieldedWithdrawal(_)
                | StateTransition::IdentityCreateFromShieldedPool(_)
                | StateTransition::TokenShieldedTransferWithShieldedFee(_)
                | StateTransition::TokenUnshieldWithShieldedFee(_)
                | StateTransition::TokenPurchaseFromShieldedPool(_)
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
    IdentityCreate {
        num_keys: usize,
    },
    IdentityTopUp,
    /// `compute_token_shielded_transfer_with_shielded_fee_fee` — two bundles (the token pool
    /// transfer and the credit pool fee), nothing written outside the pools.
    TokenShieldedTransfer {
        token_actions: usize,
    },
    /// `compute_token_unshield_with_shielded_fee_fee` — two bundles plus the recipient's token
    /// balance item.
    TokenUnshield {
        token_actions: usize,
    },
    /// `compute_token_purchase_from_shielded_pool_fee` — two bundles plus the contract owner's
    /// balance write and the supply item; `validated_amount` is the fee bundle's value balance
    /// minus the agreed price.
    TokenPurchase {
        token_actions: usize,
    },
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
                    StateTransition::Shield(_) | StateTransition::ShieldFromIdentity(_) => {
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
                    StateTransition::IdentityTopUpFromShieldedPool(st) => match st {
                        IdentityTopUpFromShieldedPoolTransition::V0(v0) => (
                            v0.top_up_amount as i64,
                            v0.actions.len(),
                            0,
                            u64::MAX,
                            false,
                            ShieldedMinFeeKind::IdentityTopUp,
                        ),
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
                    // The identity-less token pool transitions: `credit_amount` is the fee
                    // bundle's value balance and IS the fee (pure fee, exact), except for a
                    // purchase where the agreed price rides on top of it.
                    StateTransition::TokenShieldedTransferWithShieldedFee(st) => match st {
                        TokenShieldedTransferWithShieldedFeeTransition::V0(v0) => (
                            v0.credit_amount as i64,
                            v0.fee_actions.len(),
                            0,
                            u64::MAX,
                            true,
                            ShieldedMinFeeKind::TokenShieldedTransfer { token_actions: v0.token_actions.len() },
                        ),
                    },
                    StateTransition::TokenUnshieldWithShieldedFee(st) => match st {
                        TokenUnshieldWithShieldedFeeTransition::V0(v0) => (
                            v0.credit_amount as i64,
                            v0.fee_actions.len(),
                            0,
                            u64::MAX,
                            true,
                            ShieldedMinFeeKind::TokenUnshield { token_actions: v0.token_actions.len() },
                        ),
                    },
                    StateTransition::TokenPurchaseFromShieldedPool(st) => match st {
                        TokenPurchaseFromShieldedPoolTransition::V0(v0) => (
                            // Structure validation guarantees credit_amount >= total_agreed_price.
                            v0.credit_amount.saturating_sub(v0.total_agreed_price) as i64,
                            v0.fee_actions.len(),
                            0,
                            u64::MAX,
                            true,
                            ShieldedMinFeeKind::TokenPurchase { token_actions: v0.token_actions.len() },
                        ),
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
                    ShieldedMinFeeKind::IdentityTopUp => {
                        dpp::shielded::compute_shielded_identity_top_up_fee(
                            num_actions,
                            platform_version,
                        )?
                    }
                    ShieldedMinFeeKind::TokenShieldedTransfer { token_actions } => {
                        dpp::shielded::compute_token_shielded_transfer_with_shielded_fee_fee(
                            token_actions,
                            num_actions,
                            platform_version,
                        )?
                    }
                    ShieldedMinFeeKind::TokenUnshield { token_actions } => {
                        dpp::shielded::compute_token_unshield_with_shielded_fee_fee(
                            token_actions,
                            num_actions,
                            platform_version,
                        )?
                    }
                    ShieldedMinFeeKind::TokenPurchase { token_actions } => {
                        dpp::shielded::compute_token_purchase_from_shielded_pool_fee(
                            token_actions,
                            num_actions,
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
                // A batch carries no pool-paid bundle of its own; its token shielded
                // transitions are verified one by one against the sighash data each binds.
                // Block processing verifies them again inside state validation, where a
                // failure is a paid nonce bump; this stateless pass is CheckTx's admission
                // filter, run under the nonce-aware limiter.
                if let StateTransition::Batch(batch) = self {
                    return validate_batch_token_shielded_proofs(batch, platform_version);
                }

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
                    StateTransition::ShieldFromIdentity(st) => match st {
                        ShieldFromIdentityTransition::V0(v0) => reconstruct_and_verify_bundle(
                            &v0.actions,
                            FLAGS_OUTPUTS_ONLY,
                            -(v0.amount as i64),
                            &v0.anchor,
                            v0.proof.as_slice(),
                            &v0.binding_signature,
                            &[],
                        ),
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
                    StateTransition::TokenShieldedTransferWithShieldedFee(st) => match st {
                        TokenShieldedTransferWithShieldedFeeTransition::V0(v0) => {
                            let token_extra_sighash_data = dpp::shielded::token_shielded_transfer_with_shielded_fee_extra_sighash_data(
                &v0.token_id.to_buffer(),
                platform_version,
            )?;
                            let fee_extra_sighash_data =
                                dpp::shielded::token_pool_fee_bundle_extra_sighash_data(
                                    dpp::shielded::TOKEN_SHIELDED_TRANSFER_WITH_SHIELDED_FEE_TYPE,
                                    &v0.token_id.to_buffer(),
                                    &v0.token_actions,
                                    platform_version,
                                )?;
                            reconstruct_and_verify_bundle(
                                &v0.token_actions,
                                FLAGS_SPENDS_AND_OUTPUTS,
                                0,
                                &v0.token_anchor,
                                v0.token_proof.as_slice(),
                                &v0.token_binding_signature,
                                &token_extra_sighash_data,
                            )
                            .and_then(|_| {
                                reconstruct_and_verify_bundle(
                                    &v0.fee_actions,
                                    FLAGS_SPENDS_AND_OUTPUTS,
                                    v0.credit_amount as i64,
                                    &v0.fee_anchor,
                                    v0.fee_proof.as_slice(),
                                    &v0.fee_binding_signature,
                                    &fee_extra_sighash_data,
                                )
                            })
                        }
                    },
                    StateTransition::TokenUnshieldWithShieldedFee(st) => match st {
                        TokenUnshieldWithShieldedFeeTransition::V0(v0) => {
                            let token_extra_sighash_data = dpp::shielded::token_unshield_with_shielded_fee_extra_sighash_data(
                &v0.token_id.to_buffer(),
                &v0.recipient_id.to_buffer(),
                v0.amount,
                platform_version,
            )?;
                            let fee_extra_sighash_data =
                                dpp::shielded::token_pool_fee_bundle_extra_sighash_data(
                                    dpp::shielded::TOKEN_UNSHIELD_WITH_SHIELDED_FEE_TYPE,
                                    &v0.token_id.to_buffer(),
                                    &v0.token_actions,
                                    platform_version,
                                )?;
                            reconstruct_and_verify_bundle(
                                &v0.token_actions,
                                FLAGS_SPENDS_AND_OUTPUTS,
                                v0.amount as i64,
                                &v0.token_anchor,
                                v0.token_proof.as_slice(),
                                &v0.token_binding_signature,
                                &token_extra_sighash_data,
                            )
                            .and_then(|_| {
                                reconstruct_and_verify_bundle(
                                    &v0.fee_actions,
                                    FLAGS_SPENDS_AND_OUTPUTS,
                                    v0.credit_amount as i64,
                                    &v0.fee_anchor,
                                    v0.fee_proof.as_slice(),
                                    &v0.fee_binding_signature,
                                    &fee_extra_sighash_data,
                                )
                            })
                        }
                    },
                    StateTransition::TokenPurchaseFromShieldedPool(st) => match st {
                        TokenPurchaseFromShieldedPoolTransition::V0(v0) => {
                            let token_extra_sighash_data = dpp::shielded::token_purchase_from_shielded_pool_extra_sighash_data(
                &v0.token_id.to_buffer(),
                v0.token_count,
                v0.total_agreed_price,
                platform_version,
            )?;
                            let fee_extra_sighash_data =
                                dpp::shielded::token_pool_fee_bundle_extra_sighash_data(
                                    dpp::shielded::TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE,
                                    &v0.token_id.to_buffer(),
                                    &v0.token_actions,
                                    platform_version,
                                )?;
                            reconstruct_and_verify_bundle(
                                &v0.token_actions,
                                FLAGS_OUTPUTS_ONLY,
                                -(v0.token_count as i64),
                                &v0.token_anchor,
                                v0.token_proof.as_slice(),
                                &v0.token_binding_signature,
                                &token_extra_sighash_data,
                            )
                            .and_then(|_| {
                                reconstruct_and_verify_bundle(
                                    &v0.fee_actions,
                                    FLAGS_SPENDS_AND_OUTPUTS,
                                    v0.credit_amount as i64,
                                    &v0.fee_anchor,
                                    v0.fee_proof.as_slice(),
                                    &v0.fee_binding_signature,
                                    &fee_extra_sighash_data,
                                )
                            })
                        }
                    },
                    StateTransition::IdentityTopUpFromShieldedPool(st) => match st {
                        IdentityTopUpFromShieldedPoolTransition::V0(v0) => {
                            let extra_sighash_data =
                                dpp::shielded::identity_top_up_from_shielded_extra_sighash_data(
                                    &v0.identity_id.to_buffer(),
                                    v0.top_up_amount,
                                    platform_version,
                                )?;
                            reconstruct_and_verify_bundle(
                                &v0.actions,
                                FLAGS_SPENDS_AND_OUTPUTS,
                                v0.top_up_amount as i64,
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
                    // ShieldFromAssetLock retains proof verification in transform_into_action;
                    // its paid-failure action comes from the asset lock.
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

/// Verifies every token shielded bundle a batch carries, statelessly.
fn validate_batch_token_shielded_proofs(
    batch: &BatchTransition,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let owner_id = batch.owner_id().to_buffer();
    for transition in batch.transitions_iter() {
        let result = match transition {
            BatchedTransitionRef::Token(TokenTransition::Shield(t)) => {
                reconstruct_and_verify_bundle(
                    t.actions(),
                    FLAGS_OUTPUTS_ONLY,
                    -(t.amount() as i64),
                    t.anchor(),
                    t.proof(),
                    t.binding_signature(),
                    &[],
                )
            }
            BatchedTransitionRef::Token(TokenTransition::Unshield(t)) => {
                let extra_sighash_data = dpp::shielded::token_unshield_extra_sighash_data(
                    &t.base().token_id().to_buffer(),
                    &owner_id,
                    &t.recipient_id().to_buffer(),
                    t.amount(),
                    platform_version,
                )?;
                reconstruct_and_verify_bundle(
                    t.actions(),
                    FLAGS_SPENDS_AND_OUTPUTS,
                    t.amount() as i64,
                    t.anchor(),
                    t.proof(),
                    t.binding_signature(),
                    &extra_sighash_data,
                )
            }
            BatchedTransitionRef::Token(TokenTransition::ShieldedTransfer(t)) => {
                let extra_sighash_data = dpp::shielded::token_shielded_transfer_extra_sighash_data(
                    &t.base().token_id().to_buffer(),
                    &owner_id,
                    platform_version,
                )?;
                reconstruct_and_verify_bundle(
                    t.actions(),
                    FLAGS_SPENDS_AND_OUTPUTS,
                    0,
                    t.anchor(),
                    t.proof(),
                    t.binding_signature(),
                    &extra_sighash_data,
                )
            }
            BatchedTransitionRef::Token(TokenTransition::MintToPool(t)) => {
                reconstruct_and_verify_bundle(
                    t.actions(),
                    FLAGS_OUTPUTS_ONLY,
                    -(t.amount() as i64),
                    t.anchor(),
                    t.proof(),
                    t.binding_signature(),
                    &[],
                )
            }
            BatchedTransitionRef::Token(TokenTransition::BurnFromPool(t)) => {
                let extra_sighash_data = dpp::shielded::token_burn_from_pool_extra_sighash_data(
                    &t.base().token_id().to_buffer(),
                    &owner_id,
                    t.amount(),
                    platform_version,
                )?;
                reconstruct_and_verify_bundle(
                    t.actions(),
                    FLAGS_SPENDS_AND_OUTPUTS,
                    t.amount() as i64,
                    t.anchor(),
                    t.proof(),
                    t.binding_signature(),
                    &extra_sighash_data,
                )
            }
            // The claimable amount is only known against state, so a claim into the pool is
            // verified in block validation only; its proof is still admitted per nonce.
            BatchedTransitionRef::Token(TokenTransition::ClaimToPool(_)) => continue,
            BatchedTransitionRef::Token(TokenTransition::DirectPurchaseToPool(t)) => {
                reconstruct_and_verify_bundle(
                    t.actions(),
                    FLAGS_OUTPUTS_ONLY,
                    -(t.token_count() as i64),
                    t.anchor(),
                    t.proof(),
                    t.binding_signature(),
                    &[],
                )
            }
            // A document whose token cost is paid from the token's shielded pool: the payment
            // states the amount its bundle proves, and state validation rejects the document
            // when that is not the document type's cost, so the bundle can be checked here.
            BatchedTransitionRef::Document(document_transition) => {
                let base = document_transition.base();
                let Some(token_payment_info) = base.token_payment_info_ref() else {
                    continue;
                };
                let Some(payment) = token_payment_info.shielded_payment() else {
                    continue;
                };
                let extra_sighash_data = dpp::shielded::document_token_payment_extra_sighash_data(
                    &token_payment_info
                        .token_id(base.data_contract_id())
                        .to_buffer(),
                    &owner_id,
                    &base.data_contract_id().to_buffer(),
                    &base.id().to_buffer(),
                    payment.amount,
                    platform_version,
                )?;
                reconstruct_and_verify_bundle(
                    &payment.actions,
                    FLAGS_SPENDS_AND_OUTPUTS,
                    payment.amount as i64,
                    &payment.anchor,
                    &payment.proof,
                    &payment.binding_signature,
                    &extra_sighash_data,
                )
            }
            _ => continue,
        };
        if let Err(e) = result {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(e).into(),
            ));
        }
    }
    Ok(SimpleConsensusValidationResult::new())
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
