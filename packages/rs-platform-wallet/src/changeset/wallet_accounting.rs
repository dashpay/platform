//! Wallet-level accounting of a Core transaction record: the one
//! direction rule and the one net formula every Rust path applies.
//!
//! Upstream `key-wallet` classifies each matched account on its own, and
//! its account-local views disagree for an asset lock: the funding
//! account reads a lock with no change as `Outgoing` (the OP_RETURN burn
//! is not an owned output), while the keys account holding the credit
//! keys reads it as `Internal`. From the wallet's side an asset lock
//! moves Core duffs into its own Platform credits, so only the fee
//! leaves the wallet. The live projection (`fold_same_txid_records`) and
//! the SQLite repair (`platform-wallet-storage`'s `core_history`) both
//! use [`wallet_direction`], so a row cannot change direction when
//! storage repairs it.

use key_wallet::managed_account::transaction_record::{
    OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::transaction_router::TransactionType;

/// Classify a transaction from the wallet's point of view.
///
/// - `spends_ours`: at least one input spends a wallet-owned output.
/// - `has_ours`: at least one output is wallet-owned (`Received`/`Change`).
/// - `has_external`: at least one output is neither wallet-owned nor an
///   OP_RETURN burn, so value leaves the wallet.
///
/// An asset lock is `Internal` whenever nothing leaves the wallet, even
/// with no change output: its OP_RETURN value becomes the wallet's own
/// Platform credits. The Swift SDK's
/// `PersistentTransaction.reconciledAccounting` and the frozen V019
/// migration apply the same rule; keep them in step. The case table
/// shared with Swift lives in `platform-wallet-storage`
/// (`should_classify_repaired_direction_like_the_swift_sdk`).
pub fn wallet_direction(
    transaction_type: TransactionType,
    spends_ours: bool,
    has_ours: bool,
    has_external: bool,
) -> TransactionDirection {
    if transaction_type == TransactionType::CoinJoin {
        TransactionDirection::CoinJoin
    } else if !spends_ours {
        TransactionDirection::Incoming
    } else if !has_external && (has_ours || transaction_type == TransactionType::AssetLock) {
        TransactionDirection::Internal
    } else {
        TransactionDirection::Outgoing
    }
}

/// Recompute a record's wallet-level `net_amount` and `direction` from
/// its input and output details.
///
/// The net is `Σ owned outputs − Σ owned inputs`: the change in the
/// wallet's Core balance. A keys-account marker's `+credit` (Platform
/// credits, not Core funds) is therefore not part of it, so an asset lock
/// nets `−(credit + fee)`, as the SQLite repair computes.
///
/// A record with no details carries no accounting evidence (a keys-account
/// marker that met no funding slice), so it is left as upstream emitted
/// it. The SQLite repair skips such records for the same reason.
pub(crate) fn apply_wallet_accounting(record: &mut TransactionRecord) {
    if record.input_details.is_empty() && record.output_details.is_empty() {
        return;
    }
    let owned: i128 = record
        .output_details
        .iter()
        .filter(|d| is_owned(d.role))
        .map(|d| i128::from(d.value))
        .sum();
    let spent: i128 = record
        .input_details
        .iter()
        .map(|d| i128::from(d.value))
        .sum();
    // Unreachable for real amounts (the whole supply fits i64 many times
    // over); saturate rather than panic on a corrupt record.
    let net = owned - spent;
    record.net_amount = i64::try_from(net).unwrap_or(if net < 0 { i64::MIN } else { i64::MAX });

    let has_ours = record.output_details.iter().any(|d| is_owned(d.role));
    let has_external = record
        .transaction
        .output
        .iter()
        .enumerate()
        .any(|(index, output)| {
            !output.script_pubkey.is_op_return()
                && !record.output_details.iter().any(|d| {
                    d.index as usize == index
                        && (is_owned(d.role) || d.role == OutputRole::Unspendable)
                })
        });
    record.direction = wallet_direction(
        record.transaction_type,
        !record.input_details.is_empty(),
        has_ours,
        has_external,
    );
}

fn is_owned(role: OutputRole) -> bool {
    matches!(role, OutputRole::Received | OutputRole::Change)
}
