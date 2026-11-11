//! Greedy note selection for shielded spending operations.
//!
//! Selects unspent notes to cover a target amount plus fee, using a largest-first
//! strategy that minimizes the number of inputs (and thus the number of Orchard
//! actions and the overall transaction fee).

use super::store::ShieldedNote;
use crate::error::PlatformWalletError;
use dpp::fee::Credits;
use dpp::shielded::{
    compute_minimum_shielded_fee, compute_private_document_create_from_shielded_pool_fee,
    compute_shielded_identity_create_fee, compute_shielded_identity_top_up_fee,
    compute_shielded_unshield_fee, compute_shielded_withdrawal_fee,
    is_allowed_identity_create_denomination,
};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use sha2::{Digest, Sha256};

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

/// Select the fewest largest notes that cover `amount` plus their own fee.
///
/// The fee depends on the number of actions, which depends on the number of selected notes, so
/// the largest-first prefixes are tried in turn and the first whose sum covers `amount` plus the
/// fee for its own action count (at least `min_actions`) is selected.
///
/// One transition spends at most `max_shielded_transition_actions` notes: an amount that only
/// more notes than that would cover is refused with
/// [`PlatformWalletError::ShieldedSpendTooManyNotes`].
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
    let sorted = spendable_largest_first(unspent)?;
    let max_notes = max_notes_per_transition(platform_version);
    let mut total = 0u64;
    let mut required = amount;
    for (index, note) in sorted.iter().enumerate() {
        // Checked: a corrupt/crafted store could otherwise overflow u64.
        total = total.checked_add(note.value).ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(
                "shielded note values sum overflows u64".to_string(),
            )
        })?;
        let count = index + 1;
        let fee = fee_kind
            .compute(count.max(min_actions), platform_version)
            .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
        required = amount.checked_add(fee).ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError("amount + fee overflows u64".to_string())
        })?;
        if total >= required {
            if count > max_notes {
                return Err(PlatformWalletError::ShieldedSpendTooManyNotes {
                    needed: count,
                    max: max_notes,
                });
            }
            let mut sorted = sorted;
            sorted.truncate(count);
            return Ok((sorted, total, fee));
        }
    }
    Err(PlatformWalletError::ShieldedInsufficientBalance {
        available: total,
        required,
    })
}

/// The unspent notes of `unspent`, largest first; [`PlatformWalletError::ShieldedNoUnspentNotes`]
/// when there are none.
fn spendable_largest_first(
    unspent: &[ShieldedNote],
) -> Result<Vec<&ShieldedNote>, PlatformWalletError> {
    // Filter out any spent notes defensively (callers pass unspent notes only).
    let mut sorted: Vec<&ShieldedNote> = unspent.iter().filter(|n| !n.is_spent).collect();
    if sorted.is_empty() {
        return Err(PlatformWalletError::ShieldedNoUnspentNotes);
    }
    sorted.sort_by_key(|note| std::cmp::Reverse(note.value));
    Ok(sorted)
}

/// How many notes one shielded transition spends at most: one action per note.
fn max_notes_per_transition(platform_version: &PlatformVersion) -> usize {
    usize::from(
        platform_version
            .system_limits
            .max_shielded_transition_actions,
    )
}

/// The most a sweep of `unspent` can send: over the largest-first prefixes one transition can
/// spend, the greatest sum minus the fee for that prefix. A note joins only when it adds more
/// than the fee for its action, and the fewest notes win a tie, so [`select_notes_with_fee`]
/// for the returned amount selects exactly that prefix.
fn sweep_amount(
    unspent: &[ShieldedNote],
    fee_kind: ShieldedFeeKind,
    platform_version: &PlatformVersion,
) -> Result<Credits, PlatformWalletError> {
    let sorted = spendable_largest_first(unspent)?;
    let max_notes = max_notes_per_transition(platform_version);
    let mut total = 0u64;
    let mut best: Option<Credits> = None;
    for (index, note) in sorted.iter().take(max_notes).enumerate() {
        total = total.checked_add(note.value).ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(
                "shielded note values sum overflows u64".to_string(),
            )
        })?;
        let fee = fee_kind
            .compute((index + 1).max(SPEND_MIN_ACTIONS), platform_version)
            .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
        if let Some(sent) = total.checked_sub(fee).filter(|sent| *sent > 0) {
            if best.is_none_or(|best| sent > best) {
                best = Some(sent);
            }
        }
    }
    match best {
        Some(amount) => Ok(amount),
        None => {
            let fee = fee_kind
                .compute(SPEND_MIN_ACTIONS, platform_version)
                .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
            Err(PlatformWalletError::ShieldedInsufficientBalance {
                available: total,
                required: fee.saturating_add(1),
            })
        }
    }
}

/// The minimum Orchard action count of an amount-based spend: one spend plus the recipient
/// output, so a single note is still charged for two actions.
pub const SPEND_MIN_ACTIONS: usize = 2;

/// An amount-based spend from the shielded pool, whose fee depends on the notes it selects.
///
/// `IdentityCreateFromShieldedPool` is not one: its whole denomination leaves the pool and the
/// fee is metered from it at execution ([`select_notes_for_denomination`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShieldedSpendKind {
    /// ShieldedTransfer to an Orchard address.
    Transfer,
    /// Unshield to a Platform address.
    Unshield,
    /// ShieldedWithdrawal to a Core address.
    Withdrawal,
    /// IdentityTopUpFromShieldedPool.
    IdentityTopUp,
}

impl ShieldedSpendKind {
    /// The fee formula the spend reserves its notes against.
    pub fn fee_kind(self) -> ShieldedFeeKind {
        match self {
            ShieldedSpendKind::Transfer => ShieldedFeeKind::Base,
            ShieldedSpendKind::Unshield => ShieldedFeeKind::Unshield,
            ShieldedSpendKind::Withdrawal => ShieldedFeeKind::Withdrawal,
            ShieldedSpendKind::IdentityTopUp => ShieldedFeeKind::IdentityTopUp,
        }
    }

    fn tag(self) -> u8 {
        match self {
            ShieldedSpendKind::Transfer => 0,
            ShieldedSpendKind::Unshield => 1,
            ShieldedSpendKind::Withdrawal => 2,
            ShieldedSpendKind::IdentityTopUp => 3,
        }
    }
}

/// What a planned spend sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShieldedSpendAmount {
    /// Exactly this many credits reach the recipient; the fee comes on top.
    Exact(Credits),
    /// As much as the notes can send: the largest notes that add more than their action's fee,
    /// up to what one transition spends; the recipient gets their sum minus their fee.
    Sweep,
}

/// A spend's note selection made ahead of the spend, with the exact fee it pays.
///
/// Made by [`plan_spend`] from the same selection the spend runs; nothing is reserved. A spend
/// given the plan's [`digest`](Self::digest) refuses with
/// [`PlatformWalletError::ShieldedPlanChanged`] when its own selection differs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShieldedSpendPlan {
    pub kind: ShieldedSpendKind,
    /// What the recipient gets.
    pub amount: Credits,
    /// The exact fee for the selected notes.
    pub fee: Credits,
    /// The sum of the selected notes.
    pub total_input: Credits,
    /// The selected notes' nullifiers, sorted.
    pub nullifiers: Vec<[u8; 32]>,
    /// Commits to the kind, the amount, the fee and the selected notes.
    pub digest: [u8; 32],
}

impl ShieldedSpendPlan {
    fn new(
        kind: ShieldedSpendKind,
        amount: Credits,
        fee: Credits,
        total_input: Credits,
        mut nullifiers: Vec<[u8; 32]>,
    ) -> Self {
        nullifiers.sort_unstable();
        let mut hasher = Sha256::new();
        hasher.update(b"shielded-spend-plan");
        hasher.update([kind.tag()]);
        hasher.update(amount.to_le_bytes());
        hasher.update(fee.to_le_bytes());
        for nullifier in &nullifiers {
            hasher.update(nullifier);
        }
        Self {
            kind,
            amount,
            fee,
            total_input,
            nullifiers,
            digest: hasher.finalize().into(),
        }
    }

    /// How many notes the spend consumes.
    pub fn note_count(&self) -> usize {
        self.nullifiers.len()
    }

    /// What leaves the pool: the amount plus the fee (any rest returns as change).
    pub fn total_debited(&self) -> Credits {
        // Selection guarantees `amount + fee <= total_input`, so this cannot overflow.
        self.amount.saturating_add(self.fee)
    }
}

/// Plan a `kind` spend of `amount` from `unspent`: the notes the spend would select
/// ([`select_notes_with_fee`] at [`SPEND_MIN_ACTIONS`]) and their exact fee.
///
/// A [`ShieldedSpendAmount::Sweep`] sends the most the notes one transition can spend send after
/// their fee ([`sweep_amount`]); the plan selects exactly those notes.
pub fn plan_spend(
    unspent: &[ShieldedNote],
    kind: ShieldedSpendKind,
    amount: ShieldedSpendAmount,
    platform_version: &PlatformVersion,
) -> Result<ShieldedSpendPlan, PlatformWalletError> {
    let fee_kind = kind.fee_kind();
    let amount = match amount {
        ShieldedSpendAmount::Exact(amount) => amount,
        ShieldedSpendAmount::Sweep => sweep_amount(unspent, fee_kind, platform_version)?,
    };
    ensure_positive_amount(amount)?;
    let (selected, total_input, fee) = select_notes_with_fee(
        unspent,
        amount,
        SPEND_MIN_ACTIONS,
        fee_kind,
        platform_version,
    )?;
    Ok(ShieldedSpendPlan::new(
        kind,
        amount,
        fee,
        total_input,
        selected.iter().map(|n| n.nullifier).collect(),
    ))
}

/// Refuse a spend whose fresh selection (`selected`, `total_input`, `fee` for `kind` and
/// `amount`) does not match the plan whose digest is `expected`. Reads the borrowed selection
/// only, so nothing is cloned for a selection it refuses.
pub(crate) fn ensure_plan_holds(
    expected: &[u8; 32],
    kind: ShieldedSpendKind,
    amount: Credits,
    fee: Credits,
    total_input: Credits,
    selected: &[&ShieldedNote],
) -> Result<(), PlatformWalletError> {
    let fresh = ShieldedSpendPlan::new(
        kind,
        amount,
        fee,
        total_input,
        selected.iter().map(|n| n.nullifier).collect(),
    );
    if fresh.digest != *expected {
        return Err(PlatformWalletError::ShieldedPlanChanged {
            fresh_fee: fresh.fee,
            fresh_note_count: fresh.note_count(),
        });
    }
    Ok(())
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

    /// A note worth 1 DASH, so the fee never decides how many notes an amount needs.
    const NOTE: u64 = 100_000_000_000;

    /// An amount that needs five notes is planned at the fee for five actions, not two, and
    /// lists exactly those five notes.
    #[test]
    fn should_plan_a_five_note_spend_at_the_five_action_fee() {
        let platform_version = PlatformVersion::latest();
        let notes: Vec<ShieldedNote> = (0..6).map(|i| test_note(NOTE, i)).collect();
        let amount = 4 * NOTE + 1;

        let plan = plan_spend(
            &notes,
            ShieldedSpendKind::Transfer,
            ShieldedSpendAmount::Exact(amount),
            platform_version,
        )
        .expect("five notes cover the amount");

        let five_action_fee = compute_minimum_shielded_fee(5, platform_version).expect("fee");
        assert_eq!(plan.fee, five_action_fee);
        assert!(plan.fee > compute_minimum_shielded_fee(2, platform_version).expect("fee"));
        assert_eq!(plan.note_count(), 5);
        assert_eq!(plan.amount, amount);
        assert_eq!(plan.total_input, 5 * NOTE);
        assert_eq!(plan.total_debited(), amount + five_action_fee);
        // The plan is the spend's own selection.
        let (selected, total, fee) = select_notes_with_fee(
            &notes,
            amount,
            SPEND_MIN_ACTIONS,
            ShieldedFeeKind::Base,
            platform_version,
        )
        .expect("selects");
        let mut nullifiers: Vec<[u8; 32]> = selected.iter().map(|n| n.nullifier).collect();
        nullifiers.sort_unstable();
        assert_eq!(
            (plan.nullifiers, plan.total_input, plan.fee),
            (nullifiers, total, fee)
        );
    }

    /// Each kind plans against its own fee formula.
    #[test]
    fn should_plan_each_kind_against_its_spend_fee() {
        let platform_version = PlatformVersion::latest();
        let notes = vec![test_note(NOTE, 0)];
        for (kind, fee) in [
            (
                ShieldedSpendKind::Transfer,
                compute_minimum_shielded_fee(2, platform_version),
            ),
            (
                ShieldedSpendKind::Unshield,
                compute_shielded_unshield_fee(2, platform_version),
            ),
            (
                ShieldedSpendKind::Withdrawal,
                compute_shielded_withdrawal_fee(2, platform_version),
            ),
            (
                ShieldedSpendKind::IdentityTopUp,
                compute_shielded_identity_top_up_fee(2, platform_version),
            ),
        ] {
            let plan = plan_spend(
                &notes,
                kind,
                ShieldedSpendAmount::Exact(1),
                platform_version,
            )
            .expect("one note covers it");
            assert_eq!(plan.fee, fee.expect("fee"), "{kind:?}");
            assert_eq!((plan.kind, plan.note_count()), (kind, 1));
        }
    }

    /// A sweep sends the balance minus the fee for every spendable note.
    #[test]
    fn should_plan_a_sweep_as_the_balance_minus_the_fee_for_every_note() {
        let platform_version = PlatformVersion::latest();
        let mut notes: Vec<ShieldedNote> = (0..3).map(|i| test_note(NOTE + i, i)).collect();
        let mut spent = test_note(NOTE, 9);
        spent.is_spent = true;
        notes.push(spent);
        let balance = 3 * NOTE + 3;

        let plan = plan_spend(
            &notes,
            ShieldedSpendKind::Unshield,
            ShieldedSpendAmount::Sweep,
            platform_version,
        )
        .expect("the notes cover their own fee");

        let fee = compute_shielded_unshield_fee(3, platform_version).expect("fee");
        assert_eq!(plan.amount, balance - fee);
        assert_eq!(plan.fee, fee);
        assert_eq!(plan.total_debited(), balance);
        assert_eq!(plan.note_count(), 3);
    }

    /// Notes worth no more than their fee cannot be swept.
    #[test]
    fn should_refuse_a_sweep_the_fee_eats() {
        let platform_version = PlatformVersion::latest();
        let fee = compute_minimum_shielded_fee(2, platform_version).expect("fee");
        let notes = vec![test_note(fee, 0)];
        assert!(matches!(
            plan_spend(
                &notes,
                ShieldedSpendKind::Transfer,
                ShieldedSpendAmount::Sweep,
                platform_version,
            ),
            Err(PlatformWalletError::ShieldedInsufficientBalance { .. })
        ));
        assert!(matches!(
            plan_spend(
                &[],
                ShieldedSpendKind::Transfer,
                ShieldedSpendAmount::Sweep,
                platform_version,
            ),
            Err(PlatformWalletError::ShieldedNoUnspentNotes)
        ));
    }

    /// A zero amount is refused as the spend refuses it.
    #[test]
    fn should_refuse_to_plan_a_zero_amount() {
        let notes = vec![test_note(NOTE, 0)];
        assert!(matches!(
            plan_spend(
                &notes,
                ShieldedSpendKind::Transfer,
                ShieldedSpendAmount::Exact(0),
                PlatformVersion::latest(),
            ),
            Err(PlatformWalletError::ShieldedBuildError(_))
        ));
    }

    /// The digest changes with the selection, and the plan check refuses a selection that
    /// moved while accepting the one planned.
    #[test]
    fn should_hold_a_plan_only_for_its_own_selection() {
        let platform_version = PlatformVersion::latest();
        let kind = ShieldedSpendKind::Transfer;
        let notes: Vec<ShieldedNote> = (0..3).map(|i| test_note(NOTE, i)).collect();
        let amount = ShieldedSpendAmount::Exact(2 * NOTE);
        let plan = plan_spend(&notes, kind, amount, platform_version).expect("plans");
        // The plan is checked against the borrowed selection, before any note is cloned.
        fn selected<'a>(
            notes: &'a [ShieldedNote],
            kind: ShieldedSpendKind,
            platform_version: &PlatformVersion,
        ) -> (Vec<&'a ShieldedNote>, u64, u64) {
            select_notes_with_fee(notes, 2 * NOTE, 2, kind.fee_kind(), platform_version)
                .expect("selects")
        }

        let (same, total, fee) = selected(&notes, kind, platform_version);
        ensure_plan_holds(&plan.digest, kind, 2 * NOTE, fee, total, &same)
            .expect("an unchanged selection holds");

        // A larger note arrives: one note now covers it, at a different fee.
        let mut grown = notes.clone();
        grown.push(test_note(10 * NOTE, 7));
        let (moved, total, fee) = selected(&grown, kind, platform_version);
        let err = ensure_plan_holds(&plan.digest, kind, 2 * NOTE, fee, total, &moved)
            .expect_err("a moved selection must not hold");
        assert!(
            matches!(
                err,
                PlatformWalletError::ShieldedPlanChanged {
                    fresh_note_count: 1,
                    ..
                }
            ),
            "got {err:?}"
        );
        assert_ne!(
            plan_spend(&grown, kind, amount, platform_version)
                .expect("plans")
                .digest,
            plan.digest
        );
        // The same notes for another kind or amount are another plan.
        let err = ensure_plan_holds(
            &plan.digest,
            kind,
            2 * NOTE - 1,
            plan.fee,
            plan.total_input,
            &same,
        )
        .expect_err("another amount");
        assert!(matches!(
            err,
            PlatformWalletError::ShieldedPlanChanged { .. }
        ));
    }

    /// Dust that costs more than it adds stays out of a sweep: the large note alone pays its
    /// two-action fee, although the fee for every note is more than the balance.
    #[test]
    fn should_sweep_the_large_note_when_dust_costs_more_than_it_adds() {
        let platform_version = PlatformVersion::latest();
        let notes = vec![
            test_note(150_000_000, 0),
            test_note(1, 1),
            test_note(1, 2),
            test_note(1, 3),
        ];
        let two_action_fee = compute_minimum_shielded_fee(2, platform_version).expect("fee");
        assert!(
            compute_minimum_shielded_fee(4, platform_version).expect("fee") > 150_000_003,
            "the fee for every note eats the balance"
        );

        let plan = plan_spend(
            &notes,
            ShieldedSpendKind::Transfer,
            ShieldedSpendAmount::Sweep,
            platform_version,
        )
        .expect("the large note pays its own fee");

        // A second note is free: one note is already charged for two actions.
        assert_eq!(plan.note_count(), 2);
        assert!(
            plan.nullifiers.contains(&[0; 32]),
            "the large note is swept"
        );
        assert_eq!(plan.fee, two_action_fee);
        assert_eq!(plan.amount, 150_000_001 - two_action_fee);
        assert_eq!(
            plan.total_debited(),
            plan.total_input,
            "a sweep leaves no change"
        );
    }

    /// A note worth less than the fee for its action is left out of a sweep, and the amount is
    /// what the notes the spend selects send, with no change.
    #[test]
    fn should_sweep_fewer_notes_than_all_when_one_costs_more_than_it_adds() {
        let platform_version = PlatformVersion::latest();
        let dust = 1_000;
        let notes = vec![
            test_note(NOTE, 0),
            test_note(NOTE, 1),
            test_note(NOTE, 2),
            test_note(dust, 3),
        ];
        let fee = |actions| compute_minimum_shielded_fee(actions, platform_version).expect("fee");
        assert!(fee(4) - fee(3) > dust, "the dust costs more than it adds");

        let plan = plan_spend(
            &notes,
            ShieldedSpendKind::Transfer,
            ShieldedSpendAmount::Sweep,
            platform_version,
        )
        .expect("plans");

        assert_eq!(plan.note_count(), 3);
        assert_eq!(plan.fee, fee(3));
        assert_eq!(plan.amount, 3 * NOTE - fee(3));
        assert_eq!(
            plan.total_debited(),
            plan.total_input,
            "a sweep leaves no change"
        );
        // The spend run with the plan's amount selects exactly the planned notes.
        let (selected, total, selected_fee) = select_notes_with_fee(
            &notes,
            plan.amount,
            SPEND_MIN_ACTIONS,
            ShieldedFeeKind::Base,
            platform_version,
        )
        .expect("selects");
        let mut nullifiers: Vec<[u8; 32]> = selected.iter().map(|n| n.nullifier).collect();
        nullifiers.sort_unstable();
        assert_eq!(
            (nullifiers, total, selected_fee),
            (plan.nullifiers, plan.total_input, plan.fee)
        );
    }

    /// One transition spends at most `max_shielded_transition_actions` notes: a sweep of 17
    /// equal notes sweeps 16 of them.
    #[test]
    fn should_sweep_no_more_notes_than_one_transition_spends() {
        let platform_version = PlatformVersion::latest();
        let max = usize::from(
            platform_version
                .system_limits
                .max_shielded_transition_actions,
        );
        let notes: Vec<ShieldedNote> = (0..=max as u64).map(|i| test_note(NOTE, i)).collect();

        for kind in [
            ShieldedSpendKind::Transfer,
            ShieldedSpendKind::Unshield,
            ShieldedSpendKind::Withdrawal,
            ShieldedSpendKind::IdentityTopUp,
        ] {
            let plan = plan_spend(&notes, kind, ShieldedSpendAmount::Sweep, platform_version)
                .expect("plans");
            let fee = kind.fee_kind().compute(max, platform_version).expect("fee");
            assert_eq!(plan.note_count(), max, "{kind:?}");
            assert_eq!(plan.fee, fee, "{kind:?}");
            assert_eq!(plan.amount, max as u64 * NOTE - fee, "{kind:?}");
        }
    }

    /// An exact amount the notes cover only with more notes than one transition spends is
    /// refused, by the selection every spend runs and by planning alike; one that fits is not.
    #[test]
    fn should_refuse_an_amount_that_needs_more_notes_than_one_transition_spends() {
        let platform_version = PlatformVersion::latest();
        let max = usize::from(
            platform_version
                .system_limits
                .max_shielded_transition_actions,
        );
        let notes: Vec<ShieldedNote> = (0..=max as u64).map(|i| test_note(NOTE, i)).collect();
        let too_many = max as u64 * NOTE;

        for kind in [
            ShieldedSpendKind::Transfer,
            ShieldedSpendKind::Unshield,
            ShieldedSpendKind::Withdrawal,
            ShieldedSpendKind::IdentityTopUp,
        ] {
            let planned = plan_spend(
                &notes,
                kind,
                ShieldedSpendAmount::Exact(too_many),
                platform_version,
            );
            assert!(
                matches!(
                    planned,
                    Err(PlatformWalletError::ShieldedSpendTooManyNotes { needed, max: limit })
                        if needed == max + 1 && limit == max
                ),
                "{kind:?}: got {planned:?}"
            );
            let selected = select_notes_with_fee(
                &notes,
                too_many,
                SPEND_MIN_ACTIONS,
                kind.fee_kind(),
                platform_version,
            );
            assert!(
                matches!(
                    selected,
                    Err(PlatformWalletError::ShieldedSpendTooManyNotes { .. })
                ),
                "{kind:?}: got {:?}",
                selected.map(|(notes, ..)| notes.len())
            );

            let fits = plan_spend(
                &notes,
                kind,
                ShieldedSpendAmount::Exact((max as u64 - 1) * NOTE),
                platform_version,
            )
            .expect("fits one transition");
            assert_eq!(fits.note_count(), max, "{kind:?}");
        }
    }
}
