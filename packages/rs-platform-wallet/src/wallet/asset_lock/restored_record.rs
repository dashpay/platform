//! Output details for an asset-lock funding transaction re-inserted at load.
//!
//! A persister that restores an unresolved asset lock re-inserts its funding
//! transaction as a record without the per-account classification the live
//! checker computes. When that transaction's InstantSend lock or block later
//! arrives, key-wallet finds the record already present: it updates the
//! record's context and its own UTXOs, and re-emits the record as it stands.
//! A store that projects coin flags from the emitted record's
//! `output_details` (the FFI persisters do) learns nothing from an empty
//! list, so the funding transaction's change keeps the flags it had when it
//! was first seen — unconfirmed and not InstantSend-locked — after the
//! engine has confirmed it.

use dashcore::{Address, OutPoint, Transaction};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::managed_core_funds_account::ManagedCoreFundsAccount;
use key_wallet::managed_account::transaction_record::{OutputDetail, OutputRole};

/// The outputs of `tx` that `account` holds as coins at restore time, with
/// the role the live checker gives them.
///
/// Roles come from key-wallet's own read-only match of `tx` against the
/// account, in the order `record_transaction` applies them: a receive
/// address is [`OutputRole::Received`], a change address
/// [`OutputRole::Change`].
///
/// Only outputs present in the account's `utxos` at restore time are returned.
/// An output already spent then is left out: a store whose row for it is gone
/// would insert it again as an unspent coin. A coin spent after restore is the
/// re-emit's credit verdict to settle, as for any live record. An output
/// paying another of the wallet's accounts is left out too: it is that
/// account's coin, not this one's.
pub fn held_output_details(
    account: &ManagedCoreFundsAccount,
    tx: &Transaction,
) -> Vec<OutputDetail> {
    let txid = tx.txid();
    let holds_a_coin = (0..tx.output.len() as u32)
        .any(|vout| account.utxos.contains_key(&OutPoint { txid, vout }));
    let Some(account_match) = account.check_transaction_for_match(tx, None) else {
        if holds_a_coin {
            // The coin's address is missing from the restored address pool,
            // so nothing will carry its InstantSend lock or block to the store.
            tracing::warn!(
                %txid,
                "restore: account holds a coin of this transaction but does not match it"
            );
        }
        return Vec::new();
    };
    let receive: Vec<&Address> = account_match
        .account_type_match
        .involved_receive_addresses()
        .iter()
        .map(|info| &info.address)
        .collect();
    let change: Vec<&Address> = account_match
        .account_type_match
        .involved_change_addresses()
        .iter()
        .map(|info| &info.address)
        .collect();
    let network = account.network();

    tx.output
        .iter()
        .enumerate()
        .filter_map(|(index, output)| {
            let index = index as u32;
            if !account.utxos.contains_key(&OutPoint { txid, vout: index }) {
                return None;
            }
            let address = Address::from_script(&output.script_pubkey, network).ok()?;
            let role = if receive.contains(&&address) {
                OutputRole::Received
            } else if change.contains(&&address) {
                OutputRole::Change
            } else {
                return None;
            };
            Some(OutputDetail {
                index,
                role,
                address: Some(address),
                value: output.value,
            })
        })
        .collect()
}
