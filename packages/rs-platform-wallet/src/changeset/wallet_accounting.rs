//! Wallet-level accounting of a Core transaction record: the ownership
//! predicate ([`is_owned`]), the direction rule ([`wallet_direction`]) and
//! the net formula ([`wallet_accounting`]). The live projection
//! (`fold_same_txid_records`) and the SQLite repair
//! (`platform-wallet-storage`'s `core_history`) both call them, so a row
//! cannot change accounting when storage repairs it; they differ only on
//! an `i64` overflow (live saturates, repair errors).
//!
//! Upstream `key-wallet` classifies each matched account on its own, and
//! its account-local views disagree for an asset lock: the funding
//! account reads a lock with no change as `Outgoing` (the OP_RETURN burn
//! is not an owned output), while the keys account holding the credit
//! keys reads it as `Internal`. From the wallet's side an asset lock is
//! `Internal` only when its payload credits all pay the wallet's own keys;
//! credits paying anyone else leave the wallet, so the lock is `Outgoing`.
//! The net counts Core funds only, so it is the same either way.

use std::collections::BTreeSet;

use dashcore::transaction::TransactionPayload;
use dashcore::{ScriptBuf, Transaction, TxOut};

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
/// - `owns_all_credits`: the asset-lock payload has credit outputs and every
///   one pays a wallet-owned script; ignored for other transaction types.
///
/// An asset lock is `Internal` when nothing leaves the wallet: no external
/// output and every credit owned, change or not. The case table lives in
/// `platform-wallet-storage`
/// (`should_classify_repaired_direction_like_the_swift_sdk`). The rule is
/// consumed by Swift's history reconciliation (dashpay/platform#5417).
pub fn wallet_direction(
    transaction_type: TransactionType,
    spends_ours: bool,
    has_ours: bool,
    has_external: bool,
    owns_all_credits: bool,
) -> TransactionDirection {
    let stays_in_wallet = if transaction_type == TransactionType::AssetLock {
        owns_all_credits
    } else {
        has_ours
    };
    if transaction_type == TransactionType::CoinJoin {
        TransactionDirection::CoinJoin
    } else if !spends_ours {
        TransactionDirection::Incoming
    } else if !has_external && stays_in_wallet {
        TransactionDirection::Internal
    } else {
        TransactionDirection::Outgoing
    }
}

/// Recompute a record's wallet-level `net_amount` and `direction` from
/// its input and output details, saturating a net that does not fit `i64`.
///
/// A record with no details carries no accounting evidence (a keys-account
/// marker that met no funding slice), so it is left as upstream emitted
/// it. The SQLite repair skips such records for the same reason.
pub(crate) fn apply_wallet_accounting(
    record: &mut TransactionRecord,
    owned_credit_scripts: &BTreeSet<ScriptBuf>,
) {
    if record.input_details.is_empty() && record.output_details.is_empty() {
        return;
    }
    let (net, direction) = wallet_accounting(record, owned_credit_scripts);
    // Unreachable for real amounts (the whole supply fits i64 many times
    // over); saturate rather than panic on a corrupt record.
    record.net_amount = i64::try_from(net).unwrap_or(if net < 0 { i64::MIN } else { i64::MAX });
    record.direction = direction;
}

/// Wallet-level net amount and direction of `record`, from its details
/// and `owned_credit_scripts`: the payload credit scripts the caller found
/// among the wallet's keys (live) or persisted address pools (repair).
///
/// The net is `Σ owned outputs − Σ owned inputs`: the change in the
/// wallet's Core balance. A keys-account marker's `+credit` (Platform
/// credits, not Core funds) is therefore not part of it, so an asset lock
/// nets `−(credit + fee)`. The net is returned as `i128` so each caller
/// picks its own policy for a value outside `i64`.
pub fn wallet_accounting(
    record: &TransactionRecord,
    owned_credit_scripts: &BTreeSet<ScriptBuf>,
) -> (i128, TransactionDirection) {
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
    let has_ours = record.output_details.iter().any(|d| is_owned(d.role));
    // Indexed once so the per-output check below stays linear.
    let accounted: BTreeSet<usize> = record
        .output_details
        .iter()
        .filter(|d| is_owned(d.role) || d.role == OutputRole::Unspendable)
        .map(|d| d.index as usize)
        .collect();
    let has_external = record
        .transaction
        .output
        .iter()
        .enumerate()
        .any(|(index, output)| !output.script_pubkey.is_op_return() && !accounted.contains(&index));
    let credits = asset_lock_credit_outputs(&record.transaction);
    let owns_all_credits = !credits.is_empty()
        && credits
            .iter()
            .all(|output| owned_credit_scripts.contains(&output.script_pubkey));
    let direction = wallet_direction(
        record.transaction_type,
        !record.input_details.is_empty(),
        has_ours,
        has_external,
        owns_all_credits,
    );
    (owned - spent, direction)
}

/// The credit outputs of `transaction`'s asset-lock payload; empty for any
/// other transaction.
pub fn asset_lock_credit_outputs(transaction: &Transaction) -> &[TxOut] {
    match &transaction.special_transaction_payload {
        Some(TransactionPayload::AssetLockPayloadType(payload)) => &payload.credit_outputs,
        _ => &[],
    }
}

/// Whether an output with this role belongs to the wallet.
pub fn is_owned(role: OutputRole) -> bool {
    matches!(role, OutputRole::Received | OutputRole::Change)
}
