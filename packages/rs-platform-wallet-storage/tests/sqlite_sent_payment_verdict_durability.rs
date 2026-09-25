#![allow(clippy::field_reassign_with_default)]

//! A sent-payment verdict must survive a process restart.
//!
//! The wallet-event adapter is the single writer of DashPay sent-payment
//! verdicts: when a sweep proves a broadcast lost a double-spend, the adapter
//! flips the payment to `Failed` on the same `store()` round that removes the
//! loser's record. That round is the ONLY chance — upstream selects losers
//! from the live in-memory records and deletes them in the same call, so the
//! sweep never re-emits and the reconcile pass, which resolves against a
//! record that no longer exists, cannot repair it either.
//!
//! Which makes the durability question decisive rather than cosmetic. `load()`
//! rebuilds a managed identity's `dashpay_payments` from the identities
//! `entry_blob` and never reads `dashpay_payments_overlay` back (the
//! write-only overlay contract, pinned by
//! `sqlite_dashpay_overlay_contract.rs`). A round that carried the verdict
//! only on the overlay slot would therefore be undone by the next launch: the
//! payment reads `Pending` again, the transaction it names is gone, and
//! nothing can re-derive the verdict.
//!
//! So this test refuses to stop at the changeset. It stands up a real
//! `WalletManager`, seeds a `Pending` sent payment through the production
//! writer, drives the REAL adapter with a `TransactionsSwept` event against
//! this crate's SQLite persister, then closes the database and reopens it.
//! The assertion is on what `load()` hands back.

mod common;

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use common::{ensure_wallet_meta, secure_tempdir};
use dpp::identity::{Identity, IdentityV0};
use dpp::prelude::Identifier;
use key_wallet::account::account_type::StandardAccountType;
use platform_wallet::changeset::{
    spawn_wallet_event_adapter, PlatformWalletChangeSet, PlatformWalletPersistence,
    WalletMetadataEntry,
};
use platform_wallet::key_wallet_manager::WalletEvent;
use platform_wallet::test_support::funded_wallet_manager;
use platform_wallet::wallet::identity::{PaymentEntry, PaymentStatus};
use platform_wallet::wallet::persister::WalletPersister;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

const OWNER: [u8; 32] = [0xA7; 32];
const CONTACT: [u8; 32] = [0xB7; 32];

/// The transaction the sweep removes, and the key the payment entry is filed
/// under. A dummy txid is enough — the verdict path keys on the display
/// string, and nothing here inspects the transaction itself.
fn loser_txid() -> dashcore::Txid {
    use dashcore::hashes::Hash as _;
    dashcore::Txid::from_byte_array([0x5f; 32])
}

/// Read the sent payment's status out of a freshly `load()`ed start state —
/// the same path a relaunching client takes.
fn loaded_status(
    persister: &SqlitePersister,
    wallet_id: &[u8; 32],
    txid: &str,
) -> Option<PaymentStatus> {
    let state = persister.load().expect("load must succeed after reopen");
    let wallet = state.wallets.get(wallet_id)?;
    let managed = wallet
        .identity_manager
        .wallet_identities
        .get(wallet_id)?
        .get(&0)?;
    managed
        .dashpay()
        .payments
        .get(txid)
        .map(|entry| entry.status)
}

#[tokio::test]
async fn a_swept_sent_payments_failed_verdict_survives_a_reopen() {
    let tmp = secure_tempdir().expect("tempdir");
    let path = tmp.path().join("wallet.db");
    let txid = loser_txid().to_string();

    let (wallet_manager, wallet_id, _generation, _signer) =
        funded_wallet_manager(StandardAccountType::BIP44Account).await;

    // ── Session one: an identity with a Pending sent payment, on disk. ──
    {
        let persister = Arc::new(
            SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("open persister"),
        );
        ensure_wallet_meta(&persister, &wallet_id);
        let mut meta = PlatformWalletChangeSet::default();
        meta.wallet_metadata = Some(WalletMetadataEntry {
            network: key_wallet::Network::Testnet,
            wallet_group_id: wallet_id,
            birth_height: 0,
        });
        persister.store(wallet_id, meta).expect("store metadata");

        let wallet_persister = WalletPersister::new(
            wallet_id,
            Arc::clone(&persister) as Arc<dyn PlatformWalletPersistence>,
        );
        {
            let mut wm = wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            info.identity_manager
                .add_identity(
                    Identity::V0(IdentityV0 {
                        id: Identifier::from(OWNER),
                        public_keys: BTreeMap::new(),
                        balance: 0,
                        revision: 0,
                    }),
                    0,
                    wallet_id,
                    &wallet_persister,
                )
                .expect("add owner identity");
            // Seeded through the production writer, so the on-disk identity
            // blob genuinely holds `Pending` before the sweep — the state the
            // restart must NOT resurrect.
            info.identity_manager
                .managed_identity_mut(&Identifier::from(OWNER))
                .expect("managed identity")
                .record_dashpay_payment(
                    txid.clone(),
                    PaymentEntry::new_sent(Identifier::from(CONTACT), 50_000, Some("lunch".into())),
                    &wallet_persister,
                )
                .expect("record the pending payment");
        }
        persister.flush(wallet_id).expect("flush session one");
        drop(wallet_persister);

        // ── The sweep, through the real adapter. ──
        //
        // Buffered before the adapter starts and the sender dropped straight
        // after, so the drain folds the whole (one-event) backlog, commits it,
        // and exits on `Disconnected`. No sleeping, no polling.
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<WalletEvent>();
        tx.send(WalletEvent::TransactionsSwept {
            wallet_id,
            txids: vec![loser_txid()],
            superseded_by: {
                use dashcore::hashes::Hash as _;
                dashcore::Txid::from_byte_array([0x77; 32])
            },
            winner_mined_height: Some(1_499_050),
            released_outpoints: Vec::new(),
            balance: key_wallet::WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
        })
        .expect("queue the sweep");
        drop(tx);

        spawn_wallet_event_adapter(
            Arc::clone(&wallet_manager),
            Arc::downgrade(&persister),
            rx,
            Arc::new(AtomicBool::new(false)),
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .expect("the adapter exits cleanly once its backlog is committed");

        persister.flush(wallet_id).expect("flush the verdict");
        // The round reached the store at all — asserted on the overlay table,
        // NOT on `load()`. A verdict that only ever lands here is precisely
        // the failure mode the reopen below exists to catch, so this check is
        // deliberately blind to durability: it just rules out "the adapter
        // never committed" as an explanation for a red reopen.
        {
            let conn = persister.lock_conn_for_test();
            let rows: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM dashpay_payments_overlay \
                     WHERE identity_id = ?1 AND payment_id = ?2",
                    rusqlite::params![&OWNER[..], &txid],
                    |r| r.get(0),
                )
                .expect("count overlay rows");
            assert_eq!(rows, 1, "the adapter's round must have reached store()");
        }
        // The open-path registry refuses a second live persister on one file,
        // so session one must be fully released before session two opens.
        drop(persister);
    }

    // ── Session two: a fresh process's view of the same file. ──
    let reopened =
        SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen persister");
    assert_eq!(
        loaded_status(&reopened, &wallet_id, &txid),
        Some(PaymentStatus::Failed),
        "a swept payment's Failed verdict must survive a restart. If this \
         reads Pending, the adapter's round carried the verdict only on \
         `dashpay_payments_overlay` — a table `load()` never reads — while \
         the authoritative identity blob kept the pre-sweep status, and the \
         swept transaction is gone with nothing able to re-derive it."
    );
}
