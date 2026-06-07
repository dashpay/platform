//! Greedy note selection for shielded spending operations.
//!
//! Selects unspent notes to cover a target amount plus fee, using a largest-first
//! strategy that minimizes the number of inputs (and thus the number of Orchard
//! actions and the overall transaction fee).

use super::store::ShieldedNote;
use crate::error::PlatformWalletError;
use dpp::fee::Credits;
use dpp::shielded::{compute_minimum_shielded_fee, compute_shielded_withdrawal_fee};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

/// Which consensus fee formula the wallet must reserve notes against for a spend.
///
/// The flat shielded fee charged at execution time differs by transition: ShieldedTransfer and
/// Unshield are carved with [`compute_minimum_shielded_fee`], while ShieldedWithdrawal is carved
/// with [`compute_shielded_withdrawal_fee`] (the same base fee PLUS the flat Core
/// withdrawal-document storage cost). Note selection must reserve against the SAME formula the
/// builder/consensus will charge, otherwise it under-funds the spend (the builder then rejects
/// it, or — in debug — the `fee_used == exact_fee` assertion fails).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShieldedFeeKind {
    /// `compute_minimum_shielded_fee` — ShieldedTransfer and Unshield.
    Base,
    /// `compute_shielded_withdrawal_fee` — ShieldedWithdrawal (adds the flat withdrawal-document cost).
    Withdrawal,
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
            ShieldedFeeKind::Withdrawal => {
                compute_shielded_withdrawal_fee(num_actions, platform_version)
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

    let total_available: u64 = unspent_only.iter().map(|n| n.value).sum();
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
        selected.push(note);
        accumulated += note.value;
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
/// withdrawal-document cost is reserved too) and [`ShieldedFeeKind::Base`] for
/// ShieldedTransfer / Unshield. This MUST match the fee the builder/consensus will charge,
/// otherwise the spend is under-funded.
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
}
