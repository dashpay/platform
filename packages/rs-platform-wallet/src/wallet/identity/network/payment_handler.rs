//! Event handler that drives the DashPay payment hooks off upstream
//! `WalletEvent`s.
//!
//! Registered as one of the [`PlatformEventHandler`]s in
//! [`PlatformEventManager`](crate::events::PlatformEventManager), this
//! keeps the DashPay-payment domain logic out of the generic
//! core-changeset bridge ([`spawn_wallet_event_adapter`]): the bridge
//! projects every event into a `CoreChangeSet` and persists it, while
//! this handler independently records incoming payments and confirms
//! sent ones.
//!
//! # Why it spawns
//!
//! [`PlatformEventHandler::on_wallet_event`] is synchronous and is
//! dispatched from dash-spv's wallet-event broadcast monitor, which can
//! fire while SPV holds the wallet-manager write lock. The payment hooks
//! are async and take that same write lock, so they cannot run inline.
//! The handler therefore captures an owned copy of the event and spawns
//! a task that queues on the write lock and runs once SPV releases it.
//! Every hook path is idempotent per txid (re-detections converge and
//! the recurring reconcile sweep backfills anything a lagged broadcast
//! dropped), so running off the core-store bridge's ordering is safe —
//! a payment row's only foreign key is to its `identities` parent, never
//! to a core transaction row.

use std::sync::Arc;

use dash_spv::EventHandler;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::RwLock;

use crate::changeset::traits::PlatformWalletPersistence;
use crate::events::PlatformEventHandler;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Records incoming DashPay payments and confirms sent ones in response
/// to upstream `WalletEvent`s.
///
/// Holds the manager's `wallet_manager` (for the in-memory identity /
/// payment state the hooks mutate) and an `Arc<dyn PlatformWalletPersistence>`
/// (to persist the resulting payment entries). Both are cheap `Arc`
/// clones taken at manager construction.
pub(crate) struct DashPayPaymentHandler {
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: Arc<dyn PlatformWalletPersistence>,
}

impl DashPayPaymentHandler {
    pub(crate) fn new(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        persister: Arc<dyn PlatformWalletPersistence>,
    ) -> Self {
        Self {
            wallet_manager,
            persister,
        }
    }
}

impl EventHandler for DashPayPaymentHandler {
    fn on_wallet_event(&self, event: &WalletEvent) {
        if !drives_payment_hooks(event) {
            return;
        }
        // Capture owned clones so the async hooks can outlive this
        // synchronous dispatch. See the module docs for why this runs on
        // its own task rather than inline.
        let wallet_manager = Arc::clone(&self.wallet_manager);
        let persister = Arc::clone(&self.persister);
        let event = event.clone();
        tokio::spawn(async move {
            let wallet_id = event.wallet_id();
            let wallet_persister =
                crate::wallet::persister::WalletPersister::new(wallet_id, persister);
            run_dashpay_payment_hooks(&wallet_manager, &wallet_id, &wallet_persister, &event).await;
        });
    }
}

impl PlatformEventHandler for DashPayPaymentHandler {}

/// Transaction records carried by `event` that should drive the DashPay
/// payment hooks (live incoming-record recording + sent-payment confirm).
///
/// [`WalletEvent::TransactionDetected`] is the first off-chain sighting of
/// a transaction — mempool, or a direct InstantSend lock — so its
/// `record.context` is not yet block-confirmed.
/// [`WalletEvent::BlockProcessed`] carries the records a block changed:
/// `inserted` (first stored in this block) and `updated`
/// (previously-known records that this block confirmed). A wallet sees its
/// *own* broadcast in the mempool first, so that transaction reaches a
/// confirmed context only via `BlockProcessed.updated` — routing solely
/// `TransactionDetected` is the gap that left sent payments stuck
/// `Pending`: the confirm hook early-returns on the unconfirmed mempool
/// sighting and never sees the confirming block. `matured` is
/// coinbase-maturity only — never a DashPay payment — so it is excluded.
fn dashpay_payment_records(event: &WalletEvent) -> Vec<&TransactionRecord> {
    // Exhaustive on purpose (no `_` arm): a new upstream `WalletEvent`
    // variant that carries transaction records must fail to compile here
    // rather than be silently dropped — routing only `TransactionDetected`
    // is exactly the gap that left sent payments stuck `Pending`.
    match event {
        WalletEvent::TransactionDetected { record, .. } => vec![record.as_ref()],
        WalletEvent::BlockProcessed {
            inserted, updated, ..
        } => inserted.iter().chain(updated.iter()).collect(),
        WalletEvent::TransactionInstantLocked { .. }
        | WalletEvent::SyncHeightAdvanced { .. }
        | WalletEvent::ChainLockProcessed { .. } => Vec::new(),
    }
}

/// Whether `event` is worth spawning a payment-hook task for.
///
/// Covers the record-bearing events ([`dashpay_payment_records`]) plus
/// [`WalletEvent::TransactionInstantLocked`], which drives the sent-payment
/// confirm by txid alone (no record). A `BlockProcessed` that changed no
/// records — the common case while syncing past empty blocks — has no
/// payment work, so it is skipped rather than spawning a task that would
/// only take and release the wallet-manager write lock for nothing.
/// Allocation-free.
fn drives_payment_hooks(event: &WalletEvent) -> bool {
    match event {
        WalletEvent::TransactionDetected { .. } | WalletEvent::TransactionInstantLocked { .. } => {
            true
        }
        WalletEvent::BlockProcessed {
            inserted, updated, ..
        } => !inserted.is_empty() || !updated.is_empty(),
        WalletEvent::SyncHeightAdvanced { .. } | WalletEvent::ChainLockProcessed { .. } => false,
    }
}

/// Run the DashPay payment hooks for `event`: record any incoming DashPay
/// payment, then advance a matching sent payment from `Pending` to
/// `Confirmed` once its transaction reaches finality (mined or
/// InstantSend-locked). All paths are idempotent per txid, so re-detections
/// and repeated block-processing rounds converge without duplicating
/// entries.
pub(crate) async fn run_dashpay_payment_hooks(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    event: &WalletEvent,
) {
    // An InstantSend lock applied to a previously-seen transaction carries
    // no record — only a txid — and is final for DashPay display, so
    // confirm the matching sent payment directly.
    if let WalletEvent::TransactionInstantLocked { txid, .. } = event {
        crate::wallet::identity::network::confirm_sent_dashpay_payment_by_txid(
            wallet_manager,
            wallet_id,
            persister,
            txid,
        )
        .await;
        return;
    }
    for record in dashpay_payment_records(event) {
        crate::wallet::identity::network::record_incoming_dashpay_payments(
            wallet_manager,
            wallet_id,
            persister,
            record,
        )
        .await;
        crate::wallet::identity::network::confirm_sent_dashpay_payment(
            wallet_manager,
            wallet_id,
            persister,
            record,
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::blockdata::transaction::Transaction;
    use dashcore::TxIn;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::transaction_record::TransactionDirection;
    use key_wallet::transaction_checking::{TransactionContext, TransactionType};
    use key_wallet::WalletCoreBalance;

    /// A `TransactionRecord` whose txid is uniquely seeded by `seed` (via a
    /// distinct input outpoint). Context is irrelevant to the routing under
    /// test, so it stays `Mempool`.
    fn record(seed: u8) -> TransactionRecord {
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(dashcore::Txid::from([seed; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    fn block_processed(
        inserted: Vec<TransactionRecord>,
        updated: Vec<TransactionRecord>,
        matured: Vec<TransactionRecord>,
    ) -> WalletEvent {
        WalletEvent::BlockProcessed {
            wallet_id: [0u8; 32],
            height: 1,
            chain_lock: None,
            inserted,
            updated,
            matured,
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        }
    }

    /// `BlockProcessed` is the path by which a wallet's own broadcast
    /// confirms (`updated`), and the path by which a payment first seen in a
    /// block lands (`inserted`); both must drive the DashPay payment hooks.
    /// `matured` is coinbase-maturity only and carries no DashPay payment, so
    /// it is excluded. A regression that re-narrows routing to
    /// `TransactionDetected` — the original sent-payment-stuck-`Pending` bug —
    /// drops the `updated` record and fails this test.
    #[test]
    fn dashpay_payment_records_covers_block_processed_inserted_and_updated() {
        let event = block_processed(vec![record(0x01)], vec![record(0x02)], vec![record(0x03)]);
        let txids: Vec<_> = dashpay_payment_records(&event)
            .iter()
            .map(|r| r.txid)
            .collect();
        assert!(
            txids.contains(&record(0x01).txid),
            "inserted record must drive the payment hooks"
        );
        assert!(
            txids.contains(&record(0x02).txid),
            "updated (just-confirmed) record must drive the payment hooks — \
             this is how a sent payment flips Pending → Confirmed"
        );
        assert!(
            !txids.contains(&record(0x03).txid),
            "matured coinbase is not a DashPay payment and must be excluded"
        );
        assert_eq!(txids.len(), 2, "exactly inserted ∪ updated");
    }

    /// The first mempool sighting still routes its single record (incoming
    /// recording + the early-returning confirm probe).
    #[test]
    fn dashpay_payment_records_covers_transaction_detected() {
        let event = WalletEvent::TransactionDetected {
            wallet_id: [0u8; 32],
            record: Box::new(record(0x07)),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        let txids: Vec<_> = dashpay_payment_records(&event)
            .iter()
            .map(|r| r.txid)
            .collect();
        assert_eq!(txids, vec![record(0x07).txid]);
    }

    /// Events with no transaction records contribute nothing, and a
    /// record-less, non-IS event does not drive the payment hooks.
    #[test]
    fn dashpay_payment_records_empty_for_non_record_events() {
        let event = WalletEvent::SyncHeightAdvanced {
            wallet_id: [0u8; 32],
            height: 42,
        };
        assert!(dashpay_payment_records(&event).is_empty());
        assert!(!drives_payment_hooks(&event));
    }

    /// `TransactionInstantLocked` carries no record but DOES drive the
    /// payment hooks — it confirms a sent payment by txid alone (an
    /// InstantSend lock is final for DashPay display).
    #[test]
    fn instant_locked_drives_payment_hooks_without_a_record() {
        use dashcore::ephemerealdata::instant_lock::InstantLock;
        let event = WalletEvent::TransactionInstantLocked {
            wallet_id: [0u8; 32],
            txid: dashcore::Txid::from([0x11; 32]),
            instant_lock: InstantLock::default(),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
        };
        // No record to route, but the event must still drive the hooks.
        assert!(dashpay_payment_records(&event).is_empty());
        assert!(drives_payment_hooks(&event));
    }

    /// A `BlockProcessed` that changed no records (syncing past an empty
    /// block) has no payment work, so it must not spawn a hook task. Pins
    /// the spawn-skip that keeps initial sync from taking the wallet-manager
    /// write lock once per empty block.
    #[test]
    fn empty_block_processed_does_not_drive_payment_hooks() {
        let event = block_processed(Vec::new(), Vec::new(), vec![record(0x05)]);
        // `matured`-only blocks carry no DashPay payment and no inserted/
        // updated records, so there is nothing to route and nothing to spawn.
        assert!(dashpay_payment_records(&event).is_empty());
        assert!(!drives_payment_hooks(&event));
    }
}
