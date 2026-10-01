//! Wallet-level Core transaction accounting assertions.
//!
//! Reads the in-memory `TransactionRecord`s the wallet keeps per matched
//! account (retained because `e2e` enables `keep-finalized-transactions`)
//! and pins the wallet-level `direction` / `net_amount` of a transaction
//! the wallet itself built: an asset lock is `Internal` and costs exactly
//! `lock + fee`; a plain send to a foreign address is `Outgoing` and costs
//! `amount + fee`.

use dashcore::Txid;
use key_wallet::managed_account::transaction_record::{TransactionDirection, TransactionRecord};
use key_wallet::AccountType;
use platform_wallet::PlatformWallet;

/// Every record the wallet holds for `txid`, one per matched account.
pub async fn wallet_tx_records(wallet: &PlatformWallet, txid: &Txid) -> Vec<TransactionRecord> {
    let wm = wallet.wallet_manager().read().await;
    let Some(info) = wm.get_wallet_info(&wallet.wallet_id()) else {
        return Vec::new();
    };
    info.core_wallet
        .accounts
        .all_accounts()
        .into_iter()
        .filter_map(|account| account.transactions().get(txid).cloned())
        .collect()
}

/// The record on the account that paid for `txid`: the one whose inputs
/// spend wallet UTXOs. Panics unless exactly one such record exists.
pub fn funding_record(records: &[TransactionRecord], txid: &Txid) -> TransactionRecord {
    let funding: Vec<&TransactionRecord> = records
        .iter()
        .filter(|r| !r.input_details.is_empty())
        .collect();
    assert_eq!(
        funding.len(),
        1,
        "expected exactly one funding-account record for {txid}, got {} \
         (accounts: {:?})",
        funding.len(),
        records
            .iter()
            .map(|r| r.account_type)
            .collect::<Vec<AccountType>>()
    );
    funding[0].clone()
}

/// Fee of `record`'s transaction: spent wallet inputs minus every output.
/// Only valid when all inputs are the wallet's own (true for anything the
/// wallet built itself); `None` when the record spends less than it
/// outputs, i.e. it does not carry every wallet input.
fn fee_from_inputs(record: &TransactionRecord) -> Option<u64> {
    let spent: u64 = record.input_details.iter().map(|d| d.value).sum();
    let out: u64 = record.transaction.output.iter().map(|o| o.value).sum();
    spent.checked_sub(out)
}

/// Why `record` is not an asset lock's correct wallet-level accounting
/// (`Internal`, `net_amount == -(lock_amount_duffs + fee)`), or `None`.
fn asset_lock_accounting_error(
    record: &TransactionRecord,
    lock_amount_duffs: u64,
) -> Option<String> {
    let txid = record.txid;
    let Some(fee) = fee_from_inputs(record) else {
        return Some(format!(
            "asset lock {txid}: the record's wallet inputs ({} input details) do not cover \
             its outputs, so it is not the funding wallet's view (direction {:?}, net {})",
            record.input_details.len(),
            record.direction,
            record.net_amount
        ));
    };
    if let Some(recorded_fee) = record.fee.filter(|f| *f != fee) {
        return Some(format!(
            "asset lock {txid}: recorded fee {recorded_fee} disagrees with inputs - outputs {fee}"
        ));
    }
    if record.direction != TransactionDirection::Internal {
        return Some(format!(
            "asset lock {txid}: direction must be Internal (nothing leaves to a third party), \
             got {:?}",
            record.direction
        ));
    }
    let expected = -i64::try_from(lock_amount_duffs + fee).expect("fits i64");
    if record.net_amount != expected {
        return Some(format!(
            "asset lock {txid}: net_amount must be -(lock {lock_amount_duffs} + fee {fee}) = \
             {expected}, got {}",
            record.net_amount
        ));
    }
    None
}

/// Pin an asset lock's wallet-level accounting: `Internal`, and
/// `net_amount == -(lock_amount_duffs + fee)`. The locked credit value
/// leaves the Core wallet but not to a third party, so it is internal.
pub fn assert_asset_lock_accounting(record: &TransactionRecord, lock_amount_duffs: u64) {
    if let Some(error) = asset_lock_accounting_error(record, lock_amount_duffs) {
        panic!("{error}");
    }
}

/// Pin the accounting of EVERY wallet-level record stored for an asset
/// lock, in store order, not only the latest: a host renders each one as
/// it arrives, so a transiently wrong row is a visible bug even when a
/// later store corrects it.
///
/// A record with no input or output details carries no accounting (a
/// keys-account marker the wallet leaves as emitted) and is skipped; the
/// latest-record check covers the row a host ends up with. Panics listing
/// every failing record. Returns how many records were checked.
pub fn assert_every_asset_lock_record(
    records: &[TransactionRecord],
    lock_amount_duffs: u64,
) -> usize {
    let mut checked = 0;
    let mut failures = Vec::new();
    for (index, record) in records.iter().enumerate() {
        if record.input_details.is_empty() && record.output_details.is_empty() {
            continue;
        }
        checked += 1;
        if let Some(error) = asset_lock_accounting_error(record, lock_amount_duffs) {
            failures.push(format!(
                "  stored record #{index} of {}: {error}",
                records.len()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {checked} stored asset-lock record(s) have wrong accounting:\n{}",
        failures.len(),
        failures.join("\n")
    );
    checked
}

/// Pin a plain Core send's wallet-level accounting: `Outgoing`, and
/// `net_amount == -(sent_duffs + fee)`.
pub fn assert_send_accounting(record: &TransactionRecord, sent_duffs: u64) {
    let fee = fee_from_inputs(record).expect("wallet-built send spends at least its outputs");
    assert_eq!(
        record.direction,
        TransactionDirection::Outgoing,
        "send {}: direction must be Outgoing",
        record.txid
    );
    let expected = -i64::try_from(sent_duffs + fee).expect("fits i64");
    assert_eq!(
        record.net_amount, expected,
        "send {}: net_amount must be -(sent {sent_duffs} + fee {fee})",
        record.txid
    );
}

/// Deadline for a tracked lock's wallet-level record to reach the
/// harness persister.
const LIVE_RECORD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Pin a tracked asset lock's wallet-level accounting on both host-facing
/// observation points of the harness persister: every record the manager
/// stored for it (see [`assert_every_asset_lock_record`]) and the row
/// reread from a reopened SQLite snapshot. Each must be `Internal` with
/// `net_amount == -(lock.amount + fee)`.
pub async fn assert_tracked_lock_accounting(
    persister: &super::harness_persister::HarnessPersister,
    wallet_id: platform_wallet::wallet::platform_wallet::WalletId,
    lock: &platform_wallet::TrackedAssetLock,
) {
    let txid = lock.out_point.txid;
    let deadline = std::time::Instant::now() + LIVE_RECORD_TIMEOUT;
    let live = loop {
        if let Some(record) = persister.live_record(wallet_id, &txid) {
            break record;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "no wallet-level record for asset lock {txid} reached the persister \
             within {LIVE_RECORD_TIMEOUT:?}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    };
    assert_asset_lock_accounting(&live, lock.amount);
    let stored = persister.stored_records(wallet_id, &txid);
    let checked = assert_every_asset_lock_record(&stored, lock.amount);
    tracing::info!(
        target: "platform_wallet::e2e::tx_accounting",
        %txid,
        stored = stored.len(),
        checked,
        "every stored asset-lock record has the wallet-level accounting"
    );

    let reloaded = persister
        .reloaded_record(wallet_id, &txid)
        .unwrap_or_else(|e| panic!("reload asset lock {txid}: {e}"))
        .unwrap_or_else(|| panic!("asset lock {txid} has no row after a reload"));
    assert_asset_lock_accounting(&reloaded, lock.amount);
    assert_eq!(
        (reloaded.direction, reloaded.net_amount),
        (live.direction, live.net_amount),
        "a reload must not change asset lock {txid}'s accounting"
    );
}

#[cfg(test)]
mod tests {
    use dashcore::secp256k1::Secp256k1;
    use dashcore::{Address, Network, PrivateKey, PublicKey, Transaction};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::managed_account::transaction_record::{
        InputDetail, TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::transaction_router::TransactionType;
    use key_wallet::transaction_checking::TransactionContext;
    use key_wallet::AccountType;

    use super::assert_every_asset_lock_record;

    const LOCK: u64 = 5_000_000;
    const FEE: u64 = 1_000;

    /// A no-change asset lock's funding record with the given accounting.
    fn lock_record(direction: TransactionDirection, net_amount: i64) -> TransactionRecord {
        let secp = Secp256k1::new();
        let key = PrivateKey::from_byte_array(&[7u8; 32], Network::Testnet).expect("key");
        let address = Address::p2pkh(&PublicKey::from_private_key(&secp, &key), Network::Testnet);
        TransactionRecord::new(
            Transaction::dummy(&address, 0..1, &[LOCK]),
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            TransactionType::AssetLock,
            direction,
            vec![InputDetail {
                index: 0,
                value: LOCK + FEE,
                address,
            }],
            Vec::new(),
            net_amount,
        )
    }

    fn correct() -> TransactionRecord {
        lock_record(TransactionDirection::Internal, -((LOCK + FEE) as i64))
    }

    #[test]
    fn every_record_check_accepts_correct_rows_and_skips_bare_markers() {
        let mut marker = correct();
        marker.input_details.clear();
        marker.direction = TransactionDirection::Incoming;
        assert_eq!(
            assert_every_asset_lock_record(&[correct(), marker, correct()], LOCK),
            2
        );
    }

    #[test]
    #[should_panic(expected = "stored record #0 of 2")]
    fn every_record_check_rejects_a_transient_wrong_row() {
        // The #5150 shape: an Outgoing row first, corrected by a later store.
        let transient = lock_record(TransactionDirection::Outgoing, -((LOCK + FEE) as i64));
        assert_every_asset_lock_record(&[transient, correct()], LOCK);
    }

    #[test]
    #[should_panic(expected = "net_amount must be")]
    fn every_record_check_rejects_a_fee_only_net() {
        // The folded keys-account marker shape: Internal but net -fee.
        let fee_only = lock_record(TransactionDirection::Internal, -(FEE as i64));
        assert_every_asset_lock_record(&[fee_only], LOCK);
    }
}
