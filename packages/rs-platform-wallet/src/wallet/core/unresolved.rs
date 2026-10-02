//! The coins of our own sends the network has not been seen to accept.
//!
//! A send whose broadcast outcome is unknown leaves its outputs in the wallet
//! as ordinary mempool funds — its change, or the receiving side of a transfer
//! between our own accounts. If that send never reached the network those
//! coins are phantom, and every payment built on them is an orphan no node
//! accepts: the shape behind "Transaction status unknown" followed by payments
//! that never land. So they are neither offered to a new payment nor counted
//! as spendable until the send is known to be accepted.

use std::collections::HashSet;

use dashcore::{OutPoint, Transaction, Txid};
use key_wallet::managed_account::managed_account_collection::ManagedAccountCollection;
use key_wallet::managed_account::managed_core_funds_account::ManagedCoreFundsAccount;
use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
use key_wallet::Account;

use super::broadcast_resolver::collect_views;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// The wallet's unsettled transactions whose outputs are held: every own send
/// — one spending the wallet's coins — that is neither InstantSend-locked nor
/// mined and that the network was not last seen to accept, and every unsettled
/// transaction spending an output of one of those, transitively.
///
/// Derived from the wallet's own records on every read, so it holds the same
/// after a restart. Only the acceptance facts (a peer echoing a broadcast, a
/// probe verdict) live in memory: until one arrives again, an unsettled send
/// stays held, which only errs on the side of not spending. A send found dead
/// after it was found accepted is held again.
pub(crate) struct UnresolvedSends {
    txids: HashSet<Txid>,
}

impl UnresolvedSends {
    pub(crate) fn of(info: &PlatformWalletInfo) -> Self {
        let views = collect_views(info);
        let unsettled: HashSet<Txid> = views.iter().map(|view| view.txid).collect();
        let accepted = info.generation.accepted_among(&unsettled);
        let mut held: HashSet<Txid> = views
            .iter()
            .filter(|view| view.spends_own_coins && !accepted.contains(&view.txid))
            .map(|view| view.txid)
            .collect();
        // A transaction built on a held output is no better than its parent:
        // a node that has not seen the parent refuses it.
        loop {
            let before = held.len();
            for view in &views {
                if !held.contains(&view.txid)
                    && view
                        .transaction
                        .input
                        .iter()
                        .any(|input| held.contains(&input.previous_output.txid))
                {
                    held.insert(view.txid);
                }
            }
            if held.len() == before {
                break;
            }
        }
        Self { txids: held }
    }

    /// Whether the coin at `outpoint` is an output of a held transaction.
    pub(crate) fn holds(&self, outpoint: &OutPoint) -> bool {
        self.txids.contains(&outpoint.txid)
    }

    /// The value of `managed`'s held coins.
    pub(crate) fn held_value(&self, managed: &ManagedCoreFundsAccount) -> u64 {
        managed
            .utxos
            .values()
            .filter(|utxo| self.holds(&utxo.outpoint))
            .map(|utxo| utxo.value())
            .sum()
    }

    /// The first input of `transaction` that spends a held coin.
    pub(crate) fn held_input(&self, transaction: &Transaction) -> Option<OutPoint> {
        transaction
            .input
            .iter()
            .map(|input| input.previous_output)
            .find(|outpoint| self.holds(outpoint))
    }

    /// [`TransactionBuilder::add_funding`] with `managed`'s held coins left
    /// out of selection.
    ///
    /// `add_funding` copies the account's UTXOs into the builder as candidates,
    /// and selection drops a candidate that is locked. So the held coins are
    /// locked for that one call and unlocked right after — the copies keep the
    /// lock, the wallet does not. The caller holds the manager write guard
    /// across it, so no other reader sees the transient lock. A coin already
    /// locked for another reason is left as it is.
    pub(crate) fn add_funding(
        &self,
        builder: TransactionBuilder,
        managed: &mut ManagedCoreFundsAccount,
        account: &Account,
    ) -> TransactionBuilder {
        let locked = self.lock_in_account(managed);
        let builder = builder.add_funding(managed, account);
        unlock_in_account(managed, &locked);
        builder
    }

    /// Lock every held coin of every funds account in `accounts` that is not
    /// locked already, for a builder that selects across accounts on its own
    /// (the asset-lock builder). Returns what was locked, for
    /// [`unlock_in`](Self::unlock_in). The caller holds the manager write guard
    /// until the unlock.
    pub(crate) fn lock_in(&self, accounts: &mut ManagedAccountCollection) -> Vec<OutPoint> {
        accounts
            .all_funding_accounts_mut()
            .into_iter()
            .flat_map(|managed| self.lock_in_account(managed))
            .collect()
    }

    /// Undo [`lock_in`](Self::lock_in).
    pub(crate) fn unlock_in(accounts: &mut ManagedAccountCollection, locked: &[OutPoint]) {
        for managed in accounts.all_funding_accounts_mut() {
            unlock_in_account(managed, locked);
        }
    }

    /// The held value across every funds account in `accounts`.
    pub(crate) fn held_value_in(&self, accounts: &ManagedAccountCollection) -> u64 {
        accounts
            .all_funding_accounts()
            .into_iter()
            .map(|managed| self.held_value(managed))
            .sum()
    }

    fn lock_in_account(&self, managed: &mut ManagedCoreFundsAccount) -> Vec<OutPoint> {
        let locked: Vec<OutPoint> = managed
            .utxos
            .values()
            .filter(|utxo| !utxo.is_locked && self.holds(&utxo.outpoint))
            .map(|utxo| utxo.outpoint)
            .collect();
        for outpoint in &locked {
            if let Some(utxo) = managed.utxos.get_mut(outpoint) {
                utxo.lock();
            }
        }
        locked
    }
}

fn unlock_in_account(managed: &mut ManagedCoreFundsAccount, locked: &[OutPoint]) {
    for outpoint in locked {
        if let Some(utxo) = managed.utxos.get_mut(outpoint) {
            utxo.unlock();
        }
    }
}
