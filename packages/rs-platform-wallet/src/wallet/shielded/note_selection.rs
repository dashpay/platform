//! Greedy note selection for shielded spending operations.
//!
//! Selects unspent notes to cover a target amount plus fee, using a largest-first
//! strategy that minimizes the number of inputs (and thus the number of Orchard
//! actions and the overall transaction fee).

use super::store::ShieldedNote;
use crate::error::PlatformWalletError;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::MAX_DISTRIBUTION_PARAM;
use dpp::fee::Credits;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::shielded::{
    compute_minimum_shielded_fee, compute_private_document_create_from_shielded_pool_fee,
    compute_shielded_identity_create_fee, compute_shielded_identity_top_up_fee,
    compute_shielded_unshield_fee, compute_shielded_withdrawal_fee,
    compute_token_purchase_from_shielded_pool_fee,
    compute_token_shielded_transfer_with_shielded_fee_fee,
    compute_token_unshield_with_shielded_fee_fee, is_allowed_identity_create_denomination,
    SerializedAction,
};
use dpp::state_transition::state_transitions::shielded::common_validation::ENCRYPTED_NOTE_SIZE;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::v0::TokenShieldedTransferWithShieldedFeeTransitionV0;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::v0::TokenUnshieldWithShieldedFeeTransitionV0;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use dpp::state_transition::{validate_state_transition_size, StateTransition};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb_commitment_tree::Proof;

/// Orchard actions in a `TokenPurchaseFromShieldedPool`'s token bundle: its one output (the
/// bought tokens' note) with spends disabled, which Orchard pads to its two-action minimum.
const TOKEN_PURCHASE_TOKEN_BUNDLE_ACTIONS: usize = 2;

/// The fee of a `TokenPurchaseFromShieldedPool` (type 28) whose credit pool fee bundle has
/// `fee_actions` actions, paid on top of the price: the consensus formula over the token
/// bundle's two actions and the fee bundle's.
pub fn compute_token_purchase_from_pool_fee(
    fee_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    compute_token_purchase_from_shielded_pool_fee(
        TOKEN_PURCHASE_TOKEN_BUNDLE_ACTIONS,
        fee_actions,
        platform_version,
    )
}

/// Refuse a `TokenPurchaseFromShieldedPool` token count its basic structure validation refuses
/// (DPP's `is_valid_token_count`: zero, or above [`MAX_DISTRIBUTION_PARAM`]). Store- and
/// network-free, so a purchase is refused with it before anything is reserved or asked.
pub(crate) fn ensure_token_purchase_count(
    token_count: TokenAmount,
) -> Result<(), PlatformWalletError> {
    if !TokenPurchaseFromShieldedPoolTransitionV0::is_valid_token_count(token_count) {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "a shielded token purchase buys from 1 to {MAX_DISTRIBUTION_PARAM} tokens, not \
             {token_count}"
        )));
    }
    Ok(())
}

/// The serialized size of a `TokenPurchaseFromShieldedPool` whose fee bundle has `fee_actions`
/// actions, with every variable field at its largest: the two bundles' actions and proofs (each
/// of the canonical length Orchard gives a proof over its actions, [`Proof::expected_proof_size`])
/// and the amounts at `u64::MAX`.
fn token_purchase_wire_size(fee_actions: usize) -> Result<u64, ProtocolError> {
    let action = SerializedAction {
        nullifier: [0; 32],
        rk: [0; 32],
        cmx: [0; 32],
        encrypted_note: vec![0; ENCRYPTED_NOTE_SIZE],
        cv_net: [0; 32],
        spend_auth_sig: [0; 64],
    };
    let proof = |actions: usize| vec![0u8; Proof::expected_proof_size(actions)];
    let transition = StateTransition::TokenPurchaseFromShieldedPool(
        TokenPurchaseFromShieldedPoolTransition::V0(TokenPurchaseFromShieldedPoolTransitionV0 {
            data_contract_id: Identifier::from([0xFF; 32]),
            token_contract_position: u16::MAX,
            token_id: Identifier::from([0xFF; 32]),
            token_count: u64::MAX,
            total_agreed_price: u64::MAX,
            token_actions: vec![action.clone(); TOKEN_PURCHASE_TOKEN_BUNDLE_ACTIONS],
            token_anchor: [0xFF; 32],
            token_proof: proof(TOKEN_PURCHASE_TOKEN_BUNDLE_ACTIONS),
            token_binding_signature: [0xFF; 64],
            fee_actions: vec![action; fee_actions],
            fee_anchor: [0xFF; 32],
            fee_proof: proof(fee_actions),
            fee_binding_signature: [0xFF; 64],
            credit_amount: u64::MAX,
        }),
    );
    Ok(transition.serialize_to_bytes()?.len() as u64)
}

/// The most credit pool notes a `TokenPurchaseFromShieldedPool`'s fee bundle may spend at
/// `platform_version`: the fee bundle has `max(notes, 2)` actions, each bundle at most
/// `max_shielded_transition_actions`, and the transition must fit `max_state_transition_size`,
/// which every node and DAPI enforce. The size binds first: the token bundle's two-action proof
/// and a four-action fee proof alone take 19,072 of the 20,480 bytes, so a purchase pays from
/// three notes at most. A purchase needing more is refused before anything is reserved or
/// proved.
pub fn max_token_purchase_fee_notes(
    platform_version: &PlatformVersion,
) -> Result<usize, ProtocolError> {
    let max_actions = usize::from(
        platform_version
            .system_limits
            .max_shielded_transition_actions,
    );
    let mut fitting = 0;
    for notes in 1..=max_actions {
        let size = token_purchase_wire_size(notes.max(2))?;
        if validate_state_transition_size(size, platform_version).is_err() {
            break;
        }
        fitting = notes;
    }
    Ok(fitting)
}

/// Orchard actions in the token bundle of a token pool spend paid from the credit pool (types 26
/// and 27) that spends `token_notes` token notes: Orchard pads a bundle to two actions, and the
/// bundle has at most two outputs (the payment and the change), so the spends decide the count
/// from two up.
pub fn token_pool_spend_token_actions(token_notes: usize) -> usize {
    token_notes.max(2)
}

/// Which token pool spend paid from the credit pool a transition is: the two differ in what
/// their token bundle does, what their fee writes outside the pools, and their wire size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPoolSpendKind {
    /// `TokenShieldedTransferWithShieldedFee` (type 26): tokens move within the pool.
    ShieldedTransfer,
    /// `TokenUnshieldWithShieldedFee` (type 27): tokens leave the pool into an identity's
    /// balance.
    Unshield,
}

impl TokenPoolSpendKind {
    /// The consensus fee of this kind with `token_actions` and `fee_actions` actions.
    pub fn fee(
        self,
        token_actions: usize,
        fee_actions: usize,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        match self {
            Self::ShieldedTransfer => compute_token_shielded_transfer_with_shielded_fee_fee(
                token_actions,
                fee_actions,
                platform_version,
            ),
            Self::Unshield => compute_token_unshield_with_shielded_fee_fee(
                token_actions,
                fee_actions,
                platform_version,
            ),
        }
    }

    /// The serialized size of this kind with `token_actions` and `fee_actions` actions, every
    /// variable field at its largest: the actions, the proofs at the canonical length Orchard
    /// gives a proof over them ([`Proof::expected_proof_size`]) and the amounts at `u64::MAX`.
    fn wire_size(self, token_actions: usize, fee_actions: usize) -> Result<u64, ProtocolError> {
        let proof = |actions: usize| vec![0u8; Proof::expected_proof_size(actions)];
        let transition = match self {
            Self::ShieldedTransfer => StateTransition::TokenShieldedTransferWithShieldedFee(
                TokenShieldedTransferWithShieldedFeeTransition::V0(
                    TokenShieldedTransferWithShieldedFeeTransitionV0 {
                        data_contract_id: Identifier::from([0xFF; 32]),
                        token_contract_position: u16::MAX,
                        token_id: Identifier::from([0xFF; 32]),
                        token_actions: vec![largest_action(); token_actions],
                        token_anchor: [0xFF; 32],
                        token_proof: proof(token_actions),
                        token_binding_signature: [0xFF; 64],
                        fee_actions: vec![largest_action(); fee_actions],
                        fee_anchor: [0xFF; 32],
                        fee_proof: proof(fee_actions),
                        fee_binding_signature: [0xFF; 64],
                        credit_amount: u64::MAX,
                    },
                ),
            ),
            Self::Unshield => StateTransition::TokenUnshieldWithShieldedFee(
                TokenUnshieldWithShieldedFeeTransition::V0(
                    TokenUnshieldWithShieldedFeeTransitionV0 {
                        data_contract_id: Identifier::from([0xFF; 32]),
                        token_contract_position: u16::MAX,
                        token_id: Identifier::from([0xFF; 32]),
                        recipient_id: Identifier::from([0xFF; 32]),
                        amount: u64::MAX,
                        token_actions: vec![largest_action(); token_actions],
                        token_anchor: [0xFF; 32],
                        token_proof: proof(token_actions),
                        token_binding_signature: [0xFF; 64],
                        fee_actions: vec![largest_action(); fee_actions],
                        fee_anchor: [0xFF; 32],
                        fee_proof: proof(fee_actions),
                        fee_binding_signature: [0xFF; 64],
                        credit_amount: u64::MAX,
                    },
                ),
            ),
        };
        Ok(transition.serialize_to_bytes()?.len() as u64)
    }

    /// The most token notes this kind's token bundle may spend at `platform_version`, with a
    /// one-note fee bundle: each bundle has at most `max_shielded_transition_actions` actions,
    /// and the transition must fit `max_state_transition_size`, which every node and DAPI
    /// enforce. A spend needing more is refused before anything is reserved.
    pub fn max_token_notes(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<usize, ProtocolError> {
        let mut fitting = 0;
        for notes in 1..=max_bundle_actions(platform_version) {
            let size = self.wire_size(token_pool_spend_token_actions(notes), 2)?;
            if validate_state_transition_size(size, platform_version).is_err() {
                break;
            }
            fitting = notes;
        }
        Ok(fitting)
    }

    /// The most credit pool notes this kind's fee bundle may spend next to a token bundle of
    /// `token_actions` actions: the fee bundle has `max(notes, 2)` actions.
    pub fn max_fee_notes(
        self,
        token_actions: usize,
        platform_version: &PlatformVersion,
    ) -> Result<usize, ProtocolError> {
        let mut fitting = 0;
        for notes in 1..=max_bundle_actions(platform_version) {
            let size = self.wire_size(token_actions, notes.max(2))?;
            if validate_state_transition_size(size, platform_version).is_err() {
                break;
            }
            fitting = notes;
        }
        Ok(fitting)
    }
}

/// A serialized action with every field at its largest.
fn largest_action() -> SerializedAction {
    SerializedAction {
        nullifier: [0; 32],
        rk: [0; 32],
        cmx: [0; 32],
        encrypted_note: vec![0; ENCRYPTED_NOTE_SIZE],
        cv_net: [0; 32],
        spend_auth_sig: [0; 64],
    }
}

/// The most actions one shielded bundle may carry at `platform_version`.
fn max_bundle_actions(platform_version: &PlatformVersion) -> usize {
    usize::from(
        platform_version
            .system_limits
            .max_shielded_transition_actions,
    )
}

/// Which consensus fee formula the wallet must reserve notes against for a spend.
///
/// The flat shielded fee charged at execution time differs by transition: ShieldedTransfer is
/// carved with [`compute_minimum_shielded_fee`] (the base), Unshield with
/// [`compute_shielded_unshield_fee`] (the base PLUS the flat `AddBalanceToAddress` output-write
/// cost), and ShieldedWithdrawal with [`compute_shielded_withdrawal_fee`] (the base PLUS the flat
/// Core withdrawal-document storage cost). Note selection must reserve against the SAME formula the
/// builder/consensus will charge, otherwise it under-funds the spend (the builder then rejects
/// it, or — in debug — the `fee_used == exact_fee` assertion fails).
///
/// `IdentityCreate` is the odd one out: it uses the EXACT-EQUALITY model (like ShieldedTransfer's
/// `value_balance`), where the whole `denomination` leaves the pool and the metered fee is taken
/// FROM the denomination at execution — so its fee is *not* added on top of the amount during note
/// selection. The variant still carries the fee formula so the offline `denomination >= fee` gate
/// (the only way the new identity ends up with a non-negative balance) can be checked. Its fee
/// additionally depends on the number of identity public keys, so the variant carries `num_keys`.
/// Use [`select_notes_for_denomination`] (not [`select_notes_with_fee`]) for this kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShieldedFeeKind {
    /// `compute_minimum_shielded_fee` — ShieldedTransfer (the base).
    Base,
    /// `compute_shielded_unshield_fee` — Unshield (adds the flat `AddBalanceToAddress` output-write cost).
    Unshield,
    /// `compute_shielded_withdrawal_fee` — ShieldedWithdrawal (adds the flat withdrawal-document cost).
    Withdrawal,
    /// `compute_shielded_identity_create_fee` — IdentityCreateFromShieldedPool. Exact-equality
    /// model: the fee is metered FROM the denomination, not added to the selection target. Carries
    /// the identity's public-key count (the fee scales with it).
    IdentityCreate {
        /// Number of public keys in the new identity (the fee scales per key).
        num_keys: usize,
    },
    /// `compute_shielded_identity_top_up_fee`: IdentityTopUpFromShieldedPool (base plus the flat
    /// identity-balance write cost, the Unshield model with an identity as the output).
    IdentityTopUp,
    /// `compute_private_document_create_from_shielded_pool_fee`:
    /// PrivateDocumentCreateFromShieldedPool (base plus, per entry, its storage and the flat
    /// processing fee). Nothing leaves the pool but the fee, so the amount selected for is 0.
    PrivateDocumentCreate {
        /// How many entries the transition appends.
        entry_count: usize,
        /// The bytes of each, the private type's `entrySize`.
        entry_size: u32,
    },
    /// [`compute_token_purchase_from_pool_fee`]: TokenPurchaseFromShieldedPool. The action count
    /// is the credit pool fee bundle's, which spends the selected notes and always carries a
    /// change output, so it is `max(selected, 2)` as for any spend with change. The price is the
    /// amount selected for, the fee comes on top.
    TokenPurchase,
    /// A token pool spend paid from the credit pool (types 26 and 27,
    /// [`TokenPoolSpendKind::fee`]). The action count is the credit pool fee bundle's,
    /// `max(selected, 2)`; nothing but the fee leaves the credit pool, so the amount selected
    /// for is 0.
    TokenPoolSpend {
        /// Which spend it is.
        kind: TokenPoolSpendKind,
        /// The token bundle's action count ([`token_pool_spend_token_actions`]).
        token_actions: usize,
    },
}

impl ShieldedFeeKind {
    /// Compute the flat shielded fee for `num_actions` under this transition's formula.
    fn compute(
        self,
        num_actions: usize,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        match self {
            ShieldedFeeKind::Base => compute_minimum_shielded_fee(num_actions, platform_version),
            ShieldedFeeKind::Unshield => {
                compute_shielded_unshield_fee(num_actions, platform_version)
            }
            ShieldedFeeKind::Withdrawal => {
                compute_shielded_withdrawal_fee(num_actions, platform_version)
            }
            ShieldedFeeKind::IdentityCreate { num_keys } => {
                compute_shielded_identity_create_fee(num_actions, num_keys, platform_version)
            }
            ShieldedFeeKind::IdentityTopUp => {
                compute_shielded_identity_top_up_fee(num_actions, platform_version)
            }
            ShieldedFeeKind::PrivateDocumentCreate {
                entry_count,
                entry_size,
            } => compute_private_document_create_from_shielded_pool_fee(
                num_actions,
                entry_count,
                entry_size,
                platform_version,
            ),
            ShieldedFeeKind::TokenPurchase => {
                compute_token_purchase_from_pool_fee(num_actions, platform_version)
            }
            ShieldedFeeKind::TokenPoolSpend {
                kind,
                token_actions,
            } => kind.fee(token_actions, num_actions, platform_version),
        }
    }
}

/// Select unspent notes to cover `amount + fee` using a greedy algorithm.
///
/// Notes are sorted by value descending and accumulated until the target is met.
/// This minimizes the number of inputs, which keeps the Orchard action count low
/// and reduces proof generation time and fees.
///
/// # Errors
///
/// Returns `PlatformWalletError::ShieldedInsufficientBalance` if the total
/// unspent value is less than the required amount.
pub fn select_notes(
    unspent: &[ShieldedNote],
    amount: u64,
    fee: u64,
) -> Result<Vec<&ShieldedNote>, PlatformWalletError> {
    // Filter out any spent notes defensively (caller should pass unspent only,
    // but this prevents double-spend if called with get_all_notes()).
    let unspent_only: Vec<&ShieldedNote> = unspent.iter().filter(|n| !n.is_spent).collect();

    if unspent_only.is_empty() {
        return Err(PlatformWalletError::ShieldedNoUnspentNotes);
    }

    let required = amount.checked_add(fee).ok_or_else(|| {
        PlatformWalletError::ShieldedBuildError("amount + fee overflows u64".to_string())
    })?;

    // Checked accumulation: a corrupt/crafted store could otherwise overflow u64 (legitimate note
    // values sum to at most the bounded credit supply, but never trust the store blindly).
    let total_available = unspent_only
        .iter()
        .try_fold(0u64, |acc, n| acc.checked_add(n.value))
        .ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(
                "shielded note values sum overflows u64".to_string(),
            )
        })?;
    if total_available < required {
        return Err(PlatformWalletError::ShieldedInsufficientBalance {
            available: total_available,
            required,
        });
    }

    // Sort by value descending (largest first)
    let mut sorted = unspent_only;
    sorted.sort_by_key(|note| std::cmp::Reverse(note.value));

    let mut selected = Vec::new();
    let mut accumulated = 0u64;

    for note in sorted {
        // Cannot overflow (the full-set sum above already succeeded), but stay checked for clarity.
        accumulated = accumulated.checked_add(note.value).ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(
                "selected shielded note values sum overflows u64".to_string(),
            )
        })?;
        selected.push(note);
        if accumulated >= required {
            break;
        }
    }

    Ok(selected)
}

/// Select notes with iterative fee convergence.
///
/// The fee depends on the number of actions, which depends on the number of
/// selected notes. This function iterates:
/// 1. Estimate fee for `min_actions` (the builder's minimum action count)
/// 2. Select notes for amount + estimated fee
/// 3. Compute exact fee from actual note count
/// 4. If insufficient, re-select with exact fee; repeat (converges in 2-3 iterations)
///
/// `fee_kind` selects which consensus fee formula to reserve against: pass
/// [`ShieldedFeeKind::Withdrawal`] for a ShieldedWithdrawal (so the flat Core
/// withdrawal-document cost is reserved too), [`ShieldedFeeKind::Unshield`] for an Unshield (so the
/// flat `AddBalanceToAddress` output-write cost is reserved too), and [`ShieldedFeeKind::Base`] for
/// ShieldedTransfer. This MUST match the fee the builder/consensus will charge, otherwise the spend
/// is under-funded.
///
/// Returns the selected notes, total input value, and the exact fee.
pub fn select_notes_with_fee<'a>(
    unspent: &'a [ShieldedNote],
    amount: u64,
    min_actions: usize,
    fee_kind: ShieldedFeeKind,
    platform_version: &PlatformVersion,
) -> Result<(Vec<&'a ShieldedNote>, u64, u64), PlatformWalletError> {
    let mut fee_estimate = fee_kind
        .compute(min_actions, platform_version)
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

    for _ in 0..5 {
        let selected = select_notes(unspent, amount, fee_estimate)?;
        let total: u64 = selected.iter().map(|n| n.value).sum();
        let num_actions = selected.len().max(min_actions);
        let exact_fee = fee_kind
            .compute(num_actions, platform_version)
            .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        if total >= amount.saturating_add(exact_fee) {
            return Ok((selected, total, exact_fee));
        }

        fee_estimate = exact_fee;
    }

    // Final attempt with last computed fee
    let selected = select_notes(unspent, amount, fee_estimate)?;
    let total: u64 = selected.iter().map(|n| n.value).sum();
    let num_actions = selected.len().max(min_actions);
    let exact_fee = fee_kind
        .compute(num_actions, platform_version)
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

    if total < amount.saturating_add(exact_fee) {
        return Err(PlatformWalletError::ShieldedInsufficientBalance {
            available: total,
            required: amount.saturating_add(exact_fee),
        });
    }

    Ok((selected, total, exact_fee))
}

/// Reject a zero spend `amount`. Lock-free, so callers can run it before taking the store lock
/// or `shield_guard`.
pub(crate) fn ensure_positive_amount(amount: u64) -> Result<(), PlatformWalletError> {
    if amount == 0 {
        return Err(PlatformWalletError::ShieldedBuildError(
            "amount must be > 0".to_string(),
        ));
    }
    Ok(())
}

/// Reject a `denomination` outside the versioned `IdentityCreateFromShieldedPool` exit set.
///
/// Consensus only accepts members of that set, but rejects others at `validate_structure` — AFTER
/// the (expensive) Orchard build/prove — so callers gate here first. Every member is positive, so
/// this also rejects zero. Store-free, so callers can run it before taking the store lock.
pub(crate) fn ensure_allowed_denomination(
    denomination: u64,
    platform_version: &PlatformVersion,
) -> Result<(), PlatformWalletError> {
    if !is_allowed_identity_create_denomination(denomination, platform_version) {
        return Err(PlatformWalletError::ShieldedBuildError(format!(
            "denomination {denomination} is not a member of the allowed exit-denomination set \
             of protocol version {}",
            platform_version.protocol_version
        )));
    }
    Ok(())
}

/// Select notes for an `IdentityCreateFromShieldedPool` exit of exactly `denomination` credits.
///
/// Unlike [`select_notes_with_fee`], this uses the EXACT-EQUALITY model: the whole `denomination`
/// leaves the pool as the bundle's `value_balance`, and the metered fee is taken FROM the
/// denomination at execution (the new identity is created holding `denomination - fee`). So the
/// selection target is `denomination` itself — the fee is NOT added on top.
///
/// The function still computes the predicted `compute_shielded_identity_create_fee` (using the
/// resulting action count and `num_keys`) and rejects up-front if `denomination < predicted_fee`,
/// since that would create an identity with a negative/zero balance (consensus rejects
/// `total_fee >= denomination`). The predicted fee is informational — the authoritative fee is
/// metered at consensus — so a small drift between the predictor and the metered amount does not
/// affect selection (the full denomination is reserved regardless).
///
/// Returns the selected notes, total input value, and the predicted fee.
pub fn select_notes_for_denomination<'a>(
    unspent: &'a [ShieldedNote],
    denomination: u64,
    min_actions: usize,
    num_keys: usize,
    platform_version: &PlatformVersion,
) -> Result<(Vec<&'a ShieldedNote>, u64, u64), PlatformWalletError> {
    ensure_allowed_denomination(denomination, platform_version)?;

    // Target the denomination exactly — no fee added on top (exact-equality model).
    let selected = select_notes(unspent, denomination, 0)?;
    let total: u64 = selected.iter().map(|n| n.value).sum();
    let num_actions = selected.len().max(min_actions);
    let predicted_fee = ShieldedFeeKind::IdentityCreate { num_keys }
        .compute(num_actions, platform_version)
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

    // The denomination must cover the metered fee, or the new identity would be created with a
    // non-positive balance (consensus rejects `total_fee >= denomination`). Gate on the predictor.
    if denomination <= predicted_fee {
        return Err(PlatformWalletError::ShieldedBuildError(format!(
            "denomination {denomination} does not exceed the predicted identity-create fee \
             {predicted_fee}; the new identity would be created with a non-positive balance"
        )));
    }

    Ok((selected, total, predicted_fee))
}

#[cfg(test)]
mod tests {

    /// Types 26 and 27 spend at least one token note with a one-note fee
    /// bundle, and their fee kind charges the consensus formula for both
    /// bundles' action counts.
    #[test]
    fn should_bound_token_pool_spends_and_price_them_as_consensus_does() {
        let platform_version = PlatformVersion::latest();
        for kind in [
            TokenPoolSpendKind::ShieldedTransfer,
            TokenPoolSpendKind::Unshield,
        ] {
            let max_token_notes = kind.max_token_notes(platform_version).expect("limit");
            assert!(max_token_notes >= 1, "{kind:?}");
            let token_actions = token_pool_spend_token_actions(max_token_notes);
            assert!(
                kind.max_fee_notes(token_actions, platform_version)
                    .expect("fee notes")
                    >= 1,
                "{kind:?}: the largest token bundle still leaves room for a fee note"
            );
            assert_eq!(
                ShieldedFeeKind::TokenPoolSpend {
                    kind,
                    token_actions: 3
                }
                .compute(2, platform_version)
                .expect("fee"),
                kind.fee(3, 2, platform_version).expect("consensus fee")
            );
        }
        assert_eq!(token_pool_spend_token_actions(1), 2);
        assert_eq!(token_pool_spend_token_actions(3), 3);
        assert_eq!(
            TokenPoolSpendKind::Unshield
                .fee(2, 2, platform_version)
                .expect("fee"),
            compute_token_unshield_with_shielded_fee_fee(2, 2, platform_version).expect("fee")
        );
        assert_eq!(
            TokenPoolSpendKind::ShieldedTransfer
                .fee(2, 2, platform_version)
                .expect("fee"),
            compute_token_shielded_transfer_with_shielded_fee_fee(2, 2, platform_version)
                .expect("fee")
        );
    }
    use super::*;

    /// Create a test ShieldedNote with the given value.
    fn test_note(value: u64, position: u64) -> ShieldedNote {
        ShieldedNote {
            // We use a dummy note field -- in tests the orchard::Note is not needed
            // for note selection, only value and is_spent matter.
            note_data: Vec::new(),
            position,
            cmx: [0u8; 32],
            nullifier: [position as u8; 32],
            block_height: 0,
            is_spent: false,
            value,
        }
    }

    #[test]
    fn test_select_exact_amount() {
        let notes = vec![test_note(100, 0), test_note(200, 1), test_note(300, 2)];
        let result = select_notes(&notes, 300, 0).unwrap();
        // Largest-first: should pick 300 alone
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, 300);
    }

    #[test]
    fn test_select_for_denomination_rejects_non_member_before_proof() {
        // A non-member denomination must fail fast (before any note selection / Orchard prove),
        // even when the wallet holds more than enough value — consensus would reject it anyway at
        // validate_structure, so burning the prove path on it is pure waste.
        let platform_version = PlatformVersion::latest();
        let notes = vec![test_note(u64::MAX / 2, 0)];
        let err = select_notes_for_denomination(&notes, 12_345, 2, 1, platform_version)
            .expect_err("a non-member denomination must be rejected");
        assert!(
            matches!(err, PlatformWalletError::ShieldedBuildError(ref m) if m.contains("not a member")),
            "expected a not-a-member ShieldedBuildError, got: {err:?}"
        );
    }

    #[test]
    fn test_select_for_denomination_accepts_member() {
        // A member denomination with enough value selects successfully.
        let platform_version = PlatformVersion::latest();
        let denomination = 10_000_000_000u64; // 0.1 DASH — a member of both the v12 and v13 sets.
        let notes = vec![test_note(denomination + 1, 0)];
        let (selected, total, _fee) =
            select_notes_for_denomination(&notes, denomination, 2, 1, platform_version)
                .expect("a member denomination with enough value must select");
        assert_eq!(selected.len(), 1);
        assert!(total >= denomination);
    }

    /// A private document create selects notes for its fee alone, the fee consensus pins for
    /// its action count and its entries' number and size: one DashPay entry from two actions is
    /// the 114,140,000-credit base plus the entry's storage and its processing fee.
    #[test]
    fn test_select_for_a_private_document_create_covers_its_fee_alone() {
        let platform_version = PlatformVersion::latest();
        let kind = ShieldedFeeKind::PrivateDocumentCreate {
            entry_count: 1,
            entry_size: 589,
        };
        let fee = kind.compute(2, platform_version).expect("the fee");
        assert_eq!(
            fee,
            compute_private_document_create_from_shielded_pool_fee(2, 1, 589, platform_version)
                .expect("the consensus fee")
        );
        assert_eq!(fee, 114_140_000 + (589 + 32) * 27_400 + 10_000_000);

        let notes = vec![test_note(fee, 0), test_note(fee - 1, 1)];
        let (selected, total, exact_fee) =
            select_notes_with_fee(&notes, 0, 2, kind, platform_version).expect("covers the fee");
        assert_eq!((selected.len(), total, exact_fee), (1, fee, fee));
        assert!(matches!(
            select_notes_with_fee(&notes[1..], 0, 2, kind, platform_version),
            Err(PlatformWalletError::ShieldedInsufficientBalance { .. })
        ));
    }

    /// A token purchase reserves the price plus the fee the builder charges: the consensus
    /// formula over the token bundle's two actions and the fee bundle's `max(selected, 2)`. A
    /// note one credit short of price plus fee does not cover it.
    #[test]
    fn should_reserve_a_token_purchase_price_plus_the_fee_the_builder_charges() {
        let platform_version = PlatformVersion::latest();
        let kind = ShieldedFeeKind::TokenPurchase;
        for fee_actions in [2usize, 3, 7] {
            assert_eq!(
                kind.compute(fee_actions, platform_version)
                    .expect("the fee"),
                compute_token_purchase_from_shielded_pool_fee(2, fee_actions, platform_version)
                    .expect("the consensus fee"),
                "{fee_actions} fee actions"
            );
        }
        let fee = compute_token_purchase_from_pool_fee(2, platform_version).expect("the fee");
        let price = 5_000_000u64;

        let notes = vec![test_note(price + fee, 0), test_note(price + fee - 1, 1)];
        let (selected, total, exact_fee) =
            select_notes_with_fee(&notes, price, 2, kind, platform_version)
                .expect("covers price plus fee");
        assert_eq!((selected.len(), total, exact_fee), (1, price + fee, fee));
        assert!(matches!(
            select_notes_with_fee(&notes[1..], price, 2, kind, platform_version),
            Err(PlatformWalletError::ShieldedInsufficientBalance { .. })
        ));
    }

    /// A token purchase pays from three notes at most: with the token bundle's two-action proof,
    /// a fee bundle of three actions fits the 20,480-byte transition limit and one of four does
    /// not, at every protocol version that has the transition.
    #[test]
    fn should_bound_a_token_purchase_to_the_notes_that_fit_the_size_limit() {
        for protocol_version in [14, PlatformVersion::latest().protocol_version] {
            let platform_version = PlatformVersion::get(protocol_version).expect("the version");
            let max = platform_version.system_limits.max_state_transition_size;
            assert_eq!(
                max_token_purchase_fee_notes(platform_version).expect("the bound"),
                3,
                "protocol version {protocol_version}"
            );
            assert!(token_purchase_wire_size(3).expect("the size") <= max);
            assert!(token_purchase_wire_size(4).expect("the size") > max);
        }
        // The proof share the bound rests on: a two-action and a four-action proof alone.
        assert_eq!(
            Proof::expected_proof_size(2) + Proof::expected_proof_size(4),
            19_072
        );
    }

    /// Basic structure accepts token counts from 1 to `MAX_DISTRIBUTION_PARAM`, and so does the
    /// local check.
    #[test]
    fn should_check_a_token_purchase_count_as_basic_structure_does() {
        assert!(ensure_token_purchase_count(1).is_ok());
        assert!(ensure_token_purchase_count(MAX_DISTRIBUTION_PARAM).is_ok());
        for count in [0, MAX_DISTRIBUTION_PARAM + 1, u64::MAX] {
            assert!(matches!(
                ensure_token_purchase_count(count),
                Err(PlatformWalletError::InvalidParameter(_))
            ));
        }
    }

    /// When several notes pay a token purchase, the reserved fee is the one for that many fee
    /// bundle actions, as the builder computes it from `max(spends, 2)`.
    #[test]
    fn should_price_a_multi_note_token_purchase_by_its_selected_note_count() {
        let platform_version = PlatformVersion::latest();
        let price = 1_000_000u64;
        let notes: Vec<ShieldedNote> = (0..20).map(|i| test_note(60_000_000, i)).collect();
        let (selected, total, exact_fee) = select_notes_with_fee(
            &notes,
            price,
            2,
            ShieldedFeeKind::TokenPurchase,
            platform_version,
        )
        .expect("selection ok");
        assert!(selected.len() > 2, "expected more than two notes selected");
        assert_eq!(
            exact_fee,
            compute_token_purchase_from_shielded_pool_fee(2, selected.len(), platform_version)
                .expect("the consensus fee")
        );
        assert!(total >= price + exact_fee);
    }

    #[test]
    fn test_select_needs_multiple() {
        let notes = vec![test_note(100, 0), test_note(200, 1), test_note(150, 2)];
        let result = select_notes(&notes, 300, 0).unwrap();
        // Largest-first: 200 + 150 = 350 >= 300
        assert_eq!(result.len(), 2);
        let total: u64 = result.iter().map(|n| n.value).sum();
        assert!(total >= 300);
    }

    #[test]
    fn test_select_with_fee() {
        let notes = vec![test_note(500, 0), test_note(300, 1)];
        let result = select_notes(&notes, 400, 50).unwrap();
        // Need 450 total. 500 >= 450, so just one note.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, 500);
    }

    #[test]
    fn test_select_insufficient_balance() {
        let notes = vec![test_note(100, 0), test_note(200, 1)];
        let result = select_notes(&notes, 400, 0);
        assert!(result.is_err());
        match result.unwrap_err() {
            PlatformWalletError::ShieldedInsufficientBalance {
                available,
                required,
            } => {
                assert_eq!(available, 300);
                assert_eq!(required, 400);
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn test_select_empty_notes() {
        let notes: Vec<ShieldedNote> = vec![];
        let result = select_notes(&notes, 100, 0);
        assert!(matches!(
            result.unwrap_err(),
            PlatformWalletError::ShieldedNoUnspentNotes
        ));
    }

    #[test]
    fn test_select_overflow_protection() {
        let notes = vec![test_note(100, 0)];
        let result = select_notes(&notes, u64::MAX, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_select_notes_with_fee_floors_to_min_actions() {
        let platform_version = PlatformVersion::latest();
        let min_fee_2 = compute_minimum_shielded_fee(2, platform_version)
            .expect("fee computation should not overflow");
        let amount = 1_000_000u64;
        // A single note covering amount + the 2-action fee.
        let notes = vec![test_note(amount + min_fee_2 + 5, 0)];

        let (selected, total, exact_fee) =
            select_notes_with_fee(&notes, amount, 2, ShieldedFeeKind::Base, platform_version)
                .expect("selection ok");

        assert_eq!(selected.len(), 1);
        assert_eq!(total, amount + min_fee_2 + 5);
        // One selected note → num_actions = max(1, min_actions=2) = 2, so the fee is the
        // 2-action minimum even though only one note is spent (the Orchard bundle pads to 2).
        assert_eq!(exact_fee, min_fee_2);
    }

    #[test]
    fn test_select_notes_with_fee_uses_actual_action_count() {
        let platform_version = PlatformVersion::latest();
        let amount = 1_000_000u64;
        // Many equal mid-size notes so several are needed; the convergence loop must settle on
        // a fee that matches the actual selected-note (action) count, not the min_actions floor.
        let note_val = 60_000_000u64;
        let notes: Vec<ShieldedNote> = (0..20).map(|i| test_note(note_val, i)).collect();

        let (selected, total, exact_fee) =
            select_notes_with_fee(&notes, amount, 2, ShieldedFeeKind::Base, platform_version)
                .expect("selection ok");

        let expected_fee =
            compute_minimum_shielded_fee(selected.len().max(2), platform_version).unwrap();
        assert_eq!(
            exact_fee, expected_fee,
            "fee must match the selected action count"
        );
        assert!(
            total >= amount.saturating_add(exact_fee),
            "selection must cover amount + fee"
        );
        assert!(selected.len() >= 2, "expected multiple notes selected");
    }

    #[test]
    fn test_select_notes_with_fee_withdrawal_reserves_document_cost() {
        // A ShieldedWithdrawal must reserve against `compute_shielded_withdrawal_fee` (base +
        // flat document cost), which is strictly larger than `compute_minimum_shielded_fee`.
        // Reserving with `ShieldedFeeKind::Withdrawal` must therefore return the higher fee.
        let platform_version = PlatformVersion::latest();
        let base_fee_2 = compute_minimum_shielded_fee(2, platform_version)
            .expect("fee computation should not overflow");
        let withdrawal_fee_2 = compute_shielded_withdrawal_fee(2, platform_version)
            .expect("fee computation should not overflow");
        assert!(
            withdrawal_fee_2 > base_fee_2,
            "withdrawal fee must exceed the base shielded fee (it includes the document cost)"
        );

        let amount = 1_000_000u64;
        // A single note that covers amount + the 2-action WITHDRAWAL fee exactly.
        let notes = vec![test_note(amount + withdrawal_fee_2, 0)];

        let (selected, total, exact_fee) = select_notes_with_fee(
            &notes,
            amount,
            2,
            ShieldedFeeKind::Withdrawal,
            platform_version,
        )
        .expect("selection ok");

        assert_eq!(selected.len(), 1);
        assert_eq!(total, amount + withdrawal_fee_2);
        assert_eq!(
            exact_fee, withdrawal_fee_2,
            "withdrawal note selection must reserve the withdrawal-inclusive fee"
        );
    }

    #[test]
    fn test_select_notes_with_fee_unshield_reserves_address_write_cost() {
        // An Unshield must reserve against `compute_shielded_unshield_fee` (base + flat
        // `AddBalanceToAddress` output-write cost), which is strictly larger than
        // `compute_minimum_shielded_fee`. Reserving with `ShieldedFeeKind::Unshield` must therefore
        // return the higher fee.
        let platform_version = PlatformVersion::latest();
        let base_fee_2 = compute_minimum_shielded_fee(2, platform_version)
            .expect("fee computation should not overflow");
        let unshield_fee_2 = compute_shielded_unshield_fee(2, platform_version)
            .expect("fee computation should not overflow");
        assert!(
            unshield_fee_2 > base_fee_2,
            "unshield fee must exceed the base shielded fee (it includes the address-write cost)"
        );

        let amount = 1_000_000u64;
        // A single note that covers amount + the 2-action UNSHIELD fee exactly.
        let notes = vec![test_note(amount + unshield_fee_2, 0)];

        let (selected, total, exact_fee) = select_notes_with_fee(
            &notes,
            amount,
            2,
            ShieldedFeeKind::Unshield,
            platform_version,
        )
        .expect("selection ok");

        assert_eq!(selected.len(), 1);
        assert_eq!(total, amount + unshield_fee_2);
        assert_eq!(
            exact_fee, unshield_fee_2,
            "unshield note selection must reserve the unshield-inclusive fee"
        );
    }

    #[test]
    fn test_select_notes_for_denomination_targets_denomination_only() {
        // IdentityCreateFromShieldedPool uses the exact-equality model: the whole denomination
        // leaves the pool and the fee is metered FROM it, so selection targets the denomination
        // itself (NOT denomination + fee). A single note worth exactly the denomination must
        // therefore satisfy the selection.
        let platform_version = PlatformVersion::latest();
        let denomination = 10_000_000_000u64; // 0.1 DASH in credits (a member of the set)
        let notes = vec![test_note(denomination, 0)];

        let (selected, total, predicted_fee) =
            select_notes_for_denomination(&notes, denomination, 2, 1, platform_version)
                .expect("selection ok");

        assert_eq!(selected.len(), 1);
        assert_eq!(total, denomination);
        // The predicted fee is informational and must be strictly below the denomination (otherwise
        // the gate rejects). It equals the consensus identity-create fee for this action/key count.
        let expected_fee = compute_shielded_identity_create_fee(2, 1, platform_version)
            .expect("fee computation should not overflow");
        assert_eq!(predicted_fee, expected_fee);
        assert!(
            predicted_fee < denomination,
            "predicted fee must be below the denomination"
        );
    }

    #[test]
    fn test_select_notes_for_denomination_fee_scales_with_keys() {
        // The identity-create fee scales with the number of keys, so a larger key set yields a
        // larger predicted fee for the same denomination + action count.
        let platform_version = PlatformVersion::latest();
        let denomination = 100_000_000_000u64; // 1 DASH in credits
        let notes = vec![test_note(denomination, 0)];

        let (_, _, fee_1_key) =
            select_notes_for_denomination(&notes, denomination, 2, 1, platform_version)
                .expect("selection ok");
        let (_, _, fee_5_keys) =
            select_notes_for_denomination(&notes, denomination, 2, 5, platform_version)
                .expect("selection ok");

        assert!(
            fee_5_keys > fee_1_key,
            "more identity keys must predict a higher fee ({fee_5_keys} > {fee_1_key})"
        );
    }

    #[test]
    fn test_select_notes_for_denomination_rejects_denomination_below_fee() {
        // If the denomination doesn't exceed the predicted fee, the new identity would be created
        // with a non-positive balance — the selection must reject up-front.
        let platform_version = PlatformVersion::latest();
        let predicted_fee = compute_shielded_identity_create_fee(2, 1, platform_version)
            .expect("fee computation should not overflow");
        // A denomination equal to the fee (the boundary) must be rejected (`denomination <= fee`).
        let denomination = predicted_fee;
        let notes = vec![test_note(denomination, 0)];

        let result = select_notes_for_denomination(&notes, denomination, 2, 1, platform_version);
        assert!(
            matches!(result, Err(PlatformWalletError::ShieldedBuildError(_))),
            "denomination == fee must reject (non-positive resulting balance)"
        );
    }

    #[test]
    fn test_select_notes_for_denomination_insufficient_balance() {
        // Not enough unspent value to cover the denomination → insufficient-balance error
        // (the denomination is the selection target).
        let platform_version = PlatformVersion::latest();
        let denomination = 10_000_000_000u64;
        let notes = vec![test_note(denomination - 1, 0)];

        let result = select_notes_for_denomination(&notes, denomination, 2, 1, platform_version);
        assert!(matches!(
            result,
            Err(PlatformWalletError::ShieldedInsufficientBalance { .. })
        ));
    }
}
