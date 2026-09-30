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
/// wallet built itself).
fn fee_from_inputs(record: &TransactionRecord) -> u64 {
    let spent: u64 = record.input_details.iter().map(|d| d.value).sum();
    let out: u64 = record.transaction.output.iter().map(|o| o.value).sum();
    spent
        .checked_sub(out)
        .expect("wallet-built transaction spends at least its outputs")
}

/// Pin an asset lock's wallet-level accounting: `Internal`, and
/// `net_amount == -(lock_amount_duffs + fee)`. The locked credit value
/// leaves the Core wallet but not to a third party, so it is internal.
pub fn assert_asset_lock_accounting(record: &TransactionRecord, lock_amount_duffs: u64) {
    let fee = fee_from_inputs(record);
    if let Some(recorded_fee) = record.fee {
        assert_eq!(
            recorded_fee, fee,
            "asset lock {}: recorded fee disagrees with inputs - outputs",
            record.txid
        );
    }
    assert_eq!(
        record.direction,
        TransactionDirection::Internal,
        "asset lock {}: direction must be Internal (nothing leaves to a third party)",
        record.txid
    );
    let expected = -i64::try_from(lock_amount_duffs + fee).expect("fits i64");
    assert_eq!(
        record.net_amount, expected,
        "asset lock {}: net_amount must be -(lock {lock_amount_duffs} + fee {fee})",
        record.txid
    );
}

/// Pin a plain Core send's wallet-level accounting: `Outgoing`, and
/// `net_amount == -(sent_duffs + fee)`.
pub fn assert_send_accounting(record: &TransactionRecord, sent_duffs: u64) {
    let fee = fee_from_inputs(record);
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
