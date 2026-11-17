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

use dashcore::{Address, OutPoint, Transaction, TxOut};
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
/// account's coin, not this one's. A held output on an address the match
/// does not name as receive or change has no role to give it and is left
/// out, with a warning naming the output, as is a held output whose script
/// has no address. A transaction the account does not match at all gives no
/// details, with one warning naming the held outputs.
pub fn held_output_details(
    account: &ManagedCoreFundsAccount,
    tx: &Transaction,
) -> Vec<OutputDetail> {
    let txid = tx.txid();
    let held: Vec<(u32, &TxOut)> = tx
        .output
        .iter()
        .enumerate()
        .map(|(index, output)| (index as u32, output))
        .filter(|(vout, _)| account.utxos.contains_key(&OutPoint { txid, vout: *vout }))
        .collect();
    if held.is_empty() {
        return Vec::new();
    }
    let account_type = account.managed_account_type().to_account_type();
    let Some(account_match) = account.check_transaction_for_match(tx, None) else {
        // The coins' addresses are missing from the restored address pools,
        // so nothing will carry their InstantSend lock or block to the store.
        let vouts: Vec<u32> = held.iter().map(|(vout, _)| *vout).collect();
        tracing::warn!(
            %txid,
            ?vouts,
            account = ?account_type,
            "load: account holds outputs of this transaction but does not match it"
        );
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

    // A held output left out here keeps the flags the store has for it:
    // nothing will carry its InstantSend lock or block there.
    held.into_iter()
        .filter_map(|(index, output)| {
            let address = match Address::from_script(&output.script_pubkey, network) {
                Ok(address) => address,
                Err(error) => {
                    tracing::warn!(
                        %txid,
                        vout = index,
                        account = ?account_type,
                        %error,
                        "load: account holds this output but its script has no address"
                    );
                    return None;
                }
            };
            let role = if receive.contains(&&address) {
                OutputRole::Received
            } else if change.contains(&&address) {
                OutputRole::Change
            } else {
                tracing::warn!(
                    %txid,
                    vout = index,
                    account = ?account_type,
                    "load: account holds this output but the match does not classify it"
                );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::tracing_capture::{RecordedEvents, RecordingGuard};
    use key_wallet::test_utils::TestWalletContext;
    use key_wallet::Utxo;
    use tracing::Level;

    /// Hold `vout` of `tx` as a coin of `account`, as a store restores it:
    /// on the address its script pays, whether or not a pool holds it.
    fn hold(account: &mut ManagedCoreFundsAccount, tx: &Transaction, vout: u32) {
        let outpoint = OutPoint {
            txid: tx.txid(),
            vout,
        };
        let output = tx.output[vout as usize].clone();
        let address = Address::from_script(&output.script_pubkey, account.network())
            .expect("a test coin pays an address");
        account
            .utxos
            .insert(outpoint, Utxo::new(outpoint, output, address, 0, false));
    }

    fn warnings(recorder: &RecordedEvents, message: &str) -> usize {
        recorder
            .entries()
            .iter()
            .filter(|(level, msg)| *level == Level::WARN && msg == message)
            .count()
    }

    /// A held output on an address the match does not name is left out with
    /// a warning; the change next to it is still named.
    #[test]
    fn should_warn_and_leave_out_a_held_output_the_match_does_not_classify() {
        let mut ctx = TestWalletContext::new_random();
        let foreign = TestWalletContext::new_random().receive_address;
        let account = ctx
            .managed_wallet
            .first_bip44_managed_account_mut()
            .expect("bip44 managed account");
        let change = account
            .next_change_address(Some(&ctx.xpub), true)
            .expect("bip44 change address");
        let mut tx = Transaction::dummy(&change, 0..1, &[2_000]);
        tx.output.push(TxOut {
            value: 1_000,
            script_pubkey: foreign.script_pubkey(),
        });
        hold(account, &tx, 0);
        hold(account, &tx, 1);

        let recorder = RecordedEvents::default();
        let _guard = RecordingGuard::install(recorder.clone());
        let details = held_output_details(account, &tx);

        assert_eq!(details.len(), 1);
        assert_eq!(details[0].index, 0);
        assert_eq!(details[0].role, OutputRole::Change);
        assert_eq!(
            warnings(
                &recorder,
                "load: account holds this output but the match does not classify it"
            ),
            1,
            "{:?}",
            recorder.entries()
        );
    }

    /// A transaction the account does not match names no coin, and a coin of
    /// it the account holds is reported.
    #[test]
    fn should_warn_and_name_nothing_when_the_account_does_not_match_a_held_coins_transaction() {
        let mut ctx = TestWalletContext::new_random();
        let foreign = TestWalletContext::new_random().receive_address;
        let tx = Transaction::dummy(&foreign, 0..1, &[1_000]);
        let account = ctx
            .managed_wallet
            .first_bip44_managed_account_mut()
            .expect("bip44 managed account");
        hold(account, &tx, 0);

        let recorder = RecordedEvents::default();
        let _guard = RecordingGuard::install(recorder.clone());

        assert!(held_output_details(account, &tx).is_empty());
        assert_eq!(
            warnings(
                &recorder,
                "load: account holds outputs of this transaction but does not match it"
            ),
            1,
            "{:?}",
            recorder.entries()
        );
    }
}
