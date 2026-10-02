//! The coins of our own sends the network has not been seen to accept.
//!
//! A send whose broadcast outcome is unknown leaves its outputs in the wallet
//! as ordinary mempool funds — its change, or the receiving side of a transfer
//! between our own accounts. If that send never reached the network those
//! coins are phantom, and every payment built on them is an orphan no node
//! accepts: the shape behind "Transaction status unknown" followed by payments
//! that never land. So, while the manager probes unresolved broadcasts, they
//! are neither offered to a new payment nor counted as spendable until the
//! send is known to be accepted.

use std::collections::{HashMap, HashSet, VecDeque};

use dashcore::{OutPoint, Txid};
use key_wallet::managed_account::managed_core_funds_account::ManagedCoreFundsAccount;
use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;

use super::broadcast_resolver::unsettled_transactions;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// The wallet's unsettled transactions whose outputs are held: every own send
/// — one spending the wallet's coins — that is neither InstantSend-locked nor
/// mined and that the network was not last seen to accept, and every unsettled
/// transaction spending an output of one of those, transitively.
///
/// Empty while the manager does not probe unresolved broadcasts
/// ([`WalletGeneration::holds_unresolved_sends`](super::WalletGeneration::holds_unresolved_sends)):
/// without probing nothing beyond a lock or a block would ever release a held
/// send, and a record stuck in mempool context would freeze its coins for
/// good.
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
        if !info.generation.holds_unresolved_sends() {
            return Self {
                txids: HashSet::new(),
            };
        }
        let unsettled = unsettled_transactions(info);
        let unsettled_txids: HashSet<Txid> = unsettled.keys().copied().collect();

        // Each unsettled transaction's unsettled parents, and the reverse.
        let mut parents: HashMap<Txid, Vec<Txid>> = HashMap::new();
        let mut children: HashMap<Txid, Vec<Txid>> = HashMap::new();
        for (txid, (_, transaction)) in &unsettled {
            let spent: HashSet<Txid> = transaction
                .input
                .iter()
                .map(|input| input.previous_output.txid)
                .filter(|parent| unsettled.contains_key(parent))
                .collect();
            for parent in spent {
                children.entry(parent).or_default().push(*txid);
                parents.entry(*txid).or_default().push(parent);
            }
        }

        // A node that accepted a transaction has its parents: acceptance of a
        // child — an echo of the startup re-dispatch, say — is acceptance of
        // every unsettled ancestor, unless that ancestor's own record is newer
        // (found dead since).
        let facts = info.generation.acceptance_among(&unsettled_txids);
        let mut accepted: HashSet<Txid> = HashSet::new();
        let mut up: VecDeque<(Txid, u64)> = facts
            .iter()
            .filter(|(_, (was_accepted, _))| *was_accepted)
            .map(|(txid, (_, seq))| (*txid, *seq))
            .collect();
        while let Some((txid, seq)) = up.pop_front() {
            if !accepted.insert(txid) {
                continue;
            }
            for parent in parents.get(&txid).into_iter().flatten() {
                let overridden = facts
                    .get(parent)
                    .is_some_and(|(parent_accepted, parent_seq)| {
                        !parent_accepted && *parent_seq > seq
                    });
                if !overridden {
                    up.push_back((*parent, seq));
                }
            }
        }

        // A transaction built on a held output is no better than its parent:
        // a node that has not seen the parent refuses it.
        let mut held: HashSet<Txid> = HashSet::new();
        let mut queue: VecDeque<Txid> = unsettled
            .iter()
            .filter(|(txid, (own, _))| *own && !accepted.contains(*txid))
            .map(|(txid, _)| *txid)
            .collect();
        while let Some(txid) = queue.pop_front() {
            if held.insert(txid) {
                if let Some(spenders) = children.get(&txid) {
                    queue.extend(spenders.iter().copied());
                }
            }
        }
        Self { txids: held }
    }

    /// Whether the coin at `outpoint` is an output of a held transaction.
    pub(crate) fn holds(&self, outpoint: &OutPoint) -> bool {
        self.txids.contains(&outpoint.txid)
    }

    /// `managed`'s held coins.
    pub(crate) fn held_in<'a>(
        &'a self,
        managed: &'a ManagedCoreFundsAccount,
    ) -> impl Iterator<Item = OutPoint> + 'a {
        managed
            .utxos
            .keys()
            .copied()
            .filter(move |outpoint| self.holds(outpoint))
    }

    /// `builder` told never to spend `managed`'s held coins: a funding
    /// candidate is left out of selection, an input the caller seeded is
    /// refused (key-wallet's `BuilderError::ExcludedInput`). Nothing is
    /// recorded on the wallet.
    pub(crate) fn exclude_from(
        &self,
        builder: TransactionBuilder,
        managed: &ManagedCoreFundsAccount,
    ) -> TransactionBuilder {
        if self.txids.is_empty() {
            return builder;
        }
        builder.exclude_outpoints(self.held_in(managed).collect::<Vec<_>>())
    }

    /// What `managed`'s held coins could add to a payment once released: the
    /// spendable ones at `height`, each net of `input_cost`, the fee its own
    /// input adds. The test for whether a shortfall is the held coins' doing.
    pub(crate) fn held_net_value(
        &self,
        managed: &ManagedCoreFundsAccount,
        height: u32,
        input_cost: u64,
    ) -> u64 {
        if self.txids.is_empty() {
            return 0;
        }
        managed
            .spendable_utxos(height)
            .iter()
            .filter(|utxo| self.holds(&utxo.outpoint))
            .map(|utxo| utxo.value().saturating_sub(input_cost))
            .sum()
    }
}
