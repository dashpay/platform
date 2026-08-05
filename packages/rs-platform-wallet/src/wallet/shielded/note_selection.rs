//! Greedy note selection for shielded spending operations.
//!
//! Selects unspent notes to cover a target amount plus fee, using a largest-first
//! strategy that minimizes the number of inputs (and thus the number of Orchard
//! actions and the overall transaction fee).

use super::store::ShieldedNote;
use crate::error::PlatformWalletError;
use dpp::fee::Credits;
use dpp::shielded::{
    compute_minimum_shielded_fee, compute_shielded_identity_create_fee,
    compute_shielded_unshield_fee, compute_shielded_withdrawal_fee,
};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

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
    sorted.sort_by(|a, b| b.value.cmp(&a.value));

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
    // Reject a non-member denomination up-front: consensus only accepts the versioned exit set, so
    // an unsupported value is rejected at `validate_structure` — but that happens AFTER the
    // (expensive) Orchard build/prove in the current flow, so gating here avoids burning that work.
    let allowed = platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .shielded_identity_create_denominations;
    if !allowed.contains(&denomination) {
        return Err(PlatformWalletError::ShieldedBuildError(format!(
            "denomination {denomination} is not a member of the allowed exit-denomination set {allowed:?}"
        )));
    }

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

    /// A multi-output transfer reserves against `recipients + 1` outputs, because the bundle
    /// publishes `max(spends, recipients + 1, 2)` actions and a ShieldedTransfer's
    /// `value_balance` must equal `compute_minimum_shielded_fee(actions.len())` EXACTLY. If the
    /// reservation used the 2-action floor instead, it would under-reserve and the builder's
    /// carved fee would not match what was reserved.
    #[test]
    fn test_select_notes_with_fee_reserves_multi_output_action_floor() {
        let platform_version = PlatformVersion::latest();
        // Two recipient notes + change = 3 outputs → a 3-action floor.
        let min_actions = 3;
        let min_fee_3 = compute_minimum_shielded_fee(3, platform_version).expect("fee");
        let min_fee_2 = compute_minimum_shielded_fee(2, platform_version).expect("fee");
        assert!(
            min_fee_3 > min_fee_2,
            "a 3-action bundle must cost more than a 2-action one"
        );

        let amount = 3_000_000_000u64; // the invite denomination, split across two notes
                                       // A single note covering amount + the 3-action fee (plus change).
        let notes = vec![test_note(amount + min_fee_3 + 1, 0)];

        let (selected, _total, exact_fee) = select_notes_with_fee(
            &notes,
            amount,
            min_actions,
            ShieldedFeeKind::Base,
            platform_version,
        )
        .expect("selection ok");

        assert_eq!(selected.len(), 1);
        assert_eq!(
            exact_fee, min_fee_3,
            "one spend but three outputs must reserve the 3-action fee, not the 2-action floor"
        );
    }

    /// Two sub-denomination notes on one key are STRUCTURALLY forced to both be selected when
    /// the spend targets the full denomination: the greedy selector takes the largest note first
    /// and only stops once the accumulated value covers the target, and neither note alone can.
    /// This is what removes Orchard's padding action (and its random, unreproducible dummy
    /// nullifier) from the claim bundle.
    #[test]
    fn test_two_sub_denomination_notes_are_both_selected() {
        // The two shipped invite denominations, each split floor(D/2) + ceil(D/2).
        for denomination in [3_000_000_000u64, 25_000_000_000u64] {
            let lo = denomination / 2;
            let hi = denomination - lo;
            assert!(
                lo < denomination && hi < denomination,
                "each half must be strictly below the denomination"
            );

            let notes = vec![test_note(hi, 0), test_note(lo, 1)];
            // The claim targets the denomination exactly (fee metered from it, not added).
            let selected = select_notes(&notes, denomination, 0).expect("selection ok");
            assert_eq!(
                selected.len(),
                2,
                "both sub-denomination notes must be selected for denomination {denomination}"
            );
            let total: u64 = selected.iter().map(|n| n.value).sum();
            assert_eq!(total, denomination);
        }
    }

    /// Contrast: a SINGLE note worth the whole denomination stops the greedy selector after one
    /// note — the one-note invite shape that leaves Orchard to pad the bundle with a dummy.
    #[test]
    fn test_single_full_denomination_note_selects_alone() {
        let denomination = 3_000_000_000u64;
        let notes = vec![test_note(denomination, 0)];
        let selected = select_notes(&notes, denomination, 0).expect("selection ok");
        assert_eq!(
            selected.len(),
            1,
            "a single full-denomination note covers the target alone, so the bundle needs padding"
        );
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
