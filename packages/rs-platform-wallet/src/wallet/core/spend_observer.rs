//! Event handler that releases in-broadcast input fences when the wallet
//! OBSERVES the fenced outpoints spent.
//!
//! This is the evidence half of the broadcast fence
//! ([`WalletGeneration::pin_in_broadcast`](super::WalletGeneration::pin_in_broadcast)).
//! The dispatch side installs a fence when a transaction may have reached the
//! network; this side takes it down when the wallet can actually see that the
//! outpoints are spent.

use std::collections::BTreeMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use dash_spv::EventHandler;
use dashcore::OutPoint;

use crate::changeset::core_bridge::spent_outpoints;
use crate::events::{PlatformEventHandler, WalletEvent};
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Releases a wallet generation's in-broadcast fences as the wallet observes
/// the fenced outpoints spent.
///
/// # Why the fence needs this at all
///
/// A dispatch that returns anything but a definitive pre-send rejection leaves
/// its inputs fenced, because the broadcaster's return says "this may be on the
/// network", not "this wallet has seen the spend" — and on the
/// `DapiBroadcaster` path the two are far apart, since `sdk.execute` injects
/// nothing into local wallet state. Something has to end that fence, and three
/// earlier revisions tried to end it on elapsed `last_processed_height`. That
/// cannot work: catch-up advances the chain clock over blocks mined *before*
/// the transaction was submitted, so an ordinary historical sync retires a
/// fence without a shred of evidence about the transaction it was protecting
/// (`dashpay/platform#4309`).
///
/// So the fence ends on the observation instead, and this handler is where the
/// observation arrives.
///
/// # Which events, and why these are the right ones
///
/// The two variants that carry spend-bearing transaction records, which are
/// exactly the two [`build_core_changeset`](crate::changeset::core_bridge)
/// derives [`CoreChangeSet::spent_utxos`](crate::changeset::CoreChangeSet)
/// from:
///
/// * [`WalletEvent::TransactionDetected`] — first sighting, typically the
///   mempool relay of the transaction this very wallet just dispatched. On the
///   DAPI path this is the moment the wallet learns its own send exists.
/// * [`WalletEvent::BlockProcessed`] — the `inserted` records, i.e. spends
///   arriving in a block (including the dispatch's own transaction confirming
///   without ever having been seen in the mempool).
///
/// `TransactionInstantLocked` and `ChainLockProcessed` are deliberately not
/// handled: they promote the finality of a record the wallet already has, and
/// the spend was already observed when that record first arrived. Handling them
/// would re-derive the same outpoints for no change.
///
/// Both spend shapes release, and the fence does not care which it saw — the
/// dispatch's own transaction, or a competing transaction spending the same
/// outpoint. After either one the outpoint is out of this wallet's selectable
/// set, so there is no re-selection left to race. See
/// [`WalletGeneration::observe_spent`](super::WalletGeneration::observe_spent).
///
/// # Lock discipline
///
/// Mirrors [`BalanceUpdateHandler`](super::BalanceUpdateHandler), for the same
/// reason: `on_wallet_event` is synchronous and runs inside SPV's block
/// processing, which holds the wallet-manager WRITE lock for the whole batch.
/// Resolving the generation through *that* lock would deadlock or silently drop
/// every event during initial sync, so this handler holds an `Arc` clone of the
/// manager's `wallets` map instead. That map is an [`ArcSwap`], so the lookup
/// is wait-free and INFALLIBLE: a manager lifecycle write (wallet insert /
/// remove / load) publishes a new map without ever making a reader fail or
/// block. Releasing the fence then takes only the generation's `in_broadcast`
/// `std::sync::Mutex` for a few hash operations and never awaits.
///
/// The infallibility is what retires the deferral this handler used to need.
/// While the map was a `tokio::sync::RwLock`, a `try_read` losing to a
/// lifecycle writer had to queue the observation for the next delivered
/// event — dropping it was unacceptable, since `TransactionDetected` can be
/// the ONLY spend-bearing event a dispatch ever produces (InstantLock
/// promotions carry no record here by design, and an evicted or
/// never-confirmed transaction inserts no `BlockProcessed` record) and the
/// pending-spend fence has no deadline behind it, so one lost observation
/// fenced an input for the manager's lifetime (`dashpay/platform#4309`). With
/// a read that cannot fail there is no such window and nothing to queue: every
/// observation is applied at delivery.
///
/// One outcome stays terminal, deliberately: a wallet id that resolves to no
/// entry in the map is unregistered — a resolution, not contention, with
/// nothing to retry against.
///
pub struct SpendObservationHandler {
    wallets: Arc<ArcSwap<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
}

impl SpendObservationHandler {
    pub fn new(wallets: Arc<ArcSwap<BTreeMap<WalletId, Arc<PlatformWallet>>>>) -> Self {
        Self { wallets }
    }

    /// Apply `observation` to its wallet's generation.
    ///
    /// The wallets-map read is a wait-free `ArcSwap` load — it cannot fail or
    /// block, so the observation is never deferred and never dropped. A wallet
    /// absent from the loaded map is unregistered: resolved, not contended.
    fn observe(&self, wallet_id: WalletId, outpoints: Vec<OutPoint>) {
        // The wallets map, NOT the SPV-contended wallet_manager lock — see
        // the type docs.
        if let Some(wallet) = self.wallets.load().get(&wallet_id) {
            wallet.generation().observe_spent(outpoints);
        }
    }
}

impl EventHandler for SpendObservationHandler {
    fn on_wallet_event(&self, event: &WalletEvent) {
        let Some(wallet_id) = observing_wallet(event) else {
            return;
        };
        let outpoints = observed_spends(event);
        if outpoints.is_empty() {
            return;
        }
        self.observe(*wallet_id, outpoints);
    }
}

impl PlatformEventHandler for SpendObservationHandler {}

/// The wallet whose fences `event` can retire, or `None` for a variant that
/// carries no spend.
fn observing_wallet(event: &WalletEvent) -> Option<&WalletId> {
    match event {
        WalletEvent::TransactionDetected { wallet_id, .. }
        | WalletEvent::BlockProcessed { wallet_id, .. } => Some(wallet_id),
        WalletEvent::TransactionInstantLocked { .. }
        | WalletEvent::ChainLockProcessed { .. }
        | WalletEvent::SyncHeightAdvanced { .. }
        | WalletEvent::TransactionsSwept { .. } => None,
    }
}

/// Project a [`WalletEvent`] into the outpoints of ours it reports spent.
///
/// Split out of the handler so the projection — which decides *what counts as
/// observing a spend*, the fence's entire release condition — is unit-testable
/// without standing up a `PlatformWallet` and a manager, and so the dispatch
/// tests can drive the real projection rather than a hand-rolled stand-in.
///
/// Built on [`spent_outpoints`], the same per-record input walk that produces
/// [`CoreChangeSet::spent_utxos`](crate::changeset::CoreChangeSet), so the
/// fence and the persisted spent set cannot diverge.
pub(crate) fn observed_spends(event: &WalletEvent) -> Vec<dashcore::OutPoint> {
    match event {
        // First sighting — typically the mempool relay of the transaction this
        // wallet just dispatched. On the DAPI path this is the moment the
        // wallet learns its own send exists.
        WalletEvent::TransactionDetected { record, .. } => spent_outpoints(record).collect(),
        // Spends arriving in a block, including a dispatch's own transaction
        // confirming without ever having been seen in the mempool.
        //
        // `inserted` only: `updated` and `matured` re-emit records the wallet
        // already holds, whose spends were observed when they first arrived.
        WalletEvent::BlockProcessed { inserted, .. } => {
            inserted.iter().flat_map(spent_outpoints).collect()
        }
        // Finality promotions of records the wallet already holds, and a bare
        // watermark advance. No new spend in any of them — and note that the
        // watermark is precisely the "chain moved" signal that must NOT touch
        // a fence (`dashpay/platform#4309`).
        WalletEvent::TransactionInstantLocked { .. }
        | WalletEvent::ChainLockProcessed { .. }
        | WalletEvent::SyncHeightAdvanced { .. } => Vec::new(),
        // A sweep names dead transactions, and the coins it DOES report —
        // `released_outpoints` — are the ones that came back free, the
        // opposite of a spend. The inputs it kept spent are exactly the ones
        // it does not name: the event carries txids, not records, so the held
        // set cannot be derived here at all. When the winner that settled
        // them is wallet-relevant, its own `TransactionDetected` /
        // `BlockProcessed` reports those spends and retires the fence
        // through the arms above; when it is not, this wallet never observes
        // the spend from any event, which is a gap this handler cannot close
        // without the loser's inputs travelling on the event.
        WalletEvent::TransactionsSwept { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    //! Cover the projection — which events count as observing a spend, and
    //! which outpoints they yield. That decision IS the fence's release
    //! condition (`dashpay/platform#4309`), so it is pinned here rather than
    //! only exercised end to end.

    use dashcore::hashes::Hash;
    use dashcore::{
        Address as DashAddress, BlockHash, Network, OutPoint, ScriptBuf, Transaction, TxIn, Txid,
        Witness,
    };
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::{
        InputDetail, TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::transaction_router::TransactionType;
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext};
    use key_wallet::WalletCoreBalance;

    use super::*;

    const WALLET_ID: WalletId = [3u8; 32];

    fn outpoint(byte: u8) -> OutPoint {
        OutPoint {
            txid: Txid::from_slice(&[byte; 32]).expect("valid txid"),
            vout: 0,
        }
    }

    fn spending(outpoints: &[OutPoint]) -> Transaction {
        Transaction {
            version: 2,
            lock_time: 0,
            input: outpoints
                .iter()
                .map(|previous_output| TxIn {
                    previous_output: *previous_output,
                    script_sig: ScriptBuf::new(),
                    sequence: 0xffff_ffff,
                    witness: Witness::new(),
                })
                .collect(),
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    /// A record whose `input_details` claim the given input indexes as ours —
    /// the shape upstream builds for inputs that spent our outpoints.
    fn record_claiming(tx: &Transaction, ours: &[u32]) -> TransactionRecord {
        TransactionRecord::new(
            tx.clone(),
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InBlock(BlockInfo::new(
                1_000,
                BlockHash::from_slice(&[4u8; 32]).expect("valid block hash"),
                1_234_567_890,
            )),
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            ours.iter()
                .map(|index| InputDetail {
                    index: *index,
                    value: 1_000,
                    address: DashAddress::dummy(Network::Testnet, 1),
                })
                .collect(),
            Vec::new(),
            0,
        )
    }

    fn detected(record: TransactionRecord) -> WalletEvent {
        WalletEvent::TransactionDetected {
            wallet_id: WALLET_ID,
            record: Box::new(record),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        }
    }

    fn block_processed(inserted: Vec<TransactionRecord>) -> WalletEvent {
        WalletEvent::BlockProcessed {
            wallet_id: WALLET_ID,
            height: 1_000,
            chain_lock: None,
            inserted,
            updated: Vec::new(),
            matured: Vec::new(),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        }
    }

    /// A first sighting — the mempool relay of our own DAPI-broadcast send —
    /// reports its spends. This is the event that ends the fence in the case
    /// the whole redesign exists for.
    #[test]
    fn a_detected_transaction_reports_its_spends() {
        let (a, b) = (outpoint(1), outpoint(2));
        let tx = spending(&[a, b]);

        assert_eq!(
            observed_spends(&detected(record_claiming(&tx, &[0, 1]))),
            [a, b]
        );
    }

    /// Only inputs the record claims as OURS count. A transaction that also
    /// spends someone else's coins must not retire a fence on an outpoint this
    /// wallet does not own — same rule `CoreChangeSet::spent_utxos` follows,
    /// because both walk `input_details`.
    #[test]
    fn only_our_inputs_are_reported() {
        let (ours, theirs) = (outpoint(3), outpoint(4));
        let tx = spending(&[ours, theirs]);

        assert_eq!(
            observed_spends(&detected(record_claiming(&tx, &[0]))),
            [ours],
            "an input the record does not claim is not a spend of ours"
        );
    }

    /// An `input_details` index that does not address a real input is skipped
    /// rather than panicking.
    #[test]
    fn an_out_of_range_input_index_is_skipped() {
        let tx = spending(&[outpoint(5)]);

        assert!(observed_spends(&detected(record_claiming(&tx, &[7]))).is_empty());
    }

    /// A block's `inserted` records report their spends — the dispatch's own
    /// transaction confirming without ever being seen in the mempool.
    #[test]
    fn block_processed_reports_inserted_record_spends() {
        let (a, b) = (outpoint(6), outpoint(7));
        let first = spending(&[a]);
        let second = spending(&[b]);

        let spends = observed_spends(&block_processed(vec![
            record_claiming(&first, &[0]),
            record_claiming(&second, &[0]),
        ]));

        assert_eq!(spends, [a, b]);
    }

    /// THE VARIANT THAT MUST NEVER TOUCH A FENCE.
    ///
    /// `SyncHeightAdvanced` is the bare "the chain moved" watermark, and it is
    /// precisely the signal three earlier revisions of this fix let retire a
    /// fence — via a `last_processed_height + N` bound rather than directly,
    /// but with the same effect. It reports no spend and must stay that way
    /// (`dashpay/platform#4309`).
    #[test]
    fn chain_progress_alone_reports_no_spend() {
        let event = WalletEvent::SyncHeightAdvanced {
            wallet_id: WALLET_ID,
            height: 900_000,
        };

        assert!(observed_spends(&event).is_empty());
        assert!(
            observing_wallet(&event).is_none(),
            "a bare watermark advance must not even resolve a wallet to act on"
        );
    }
}
