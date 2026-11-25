//! An InstantSend lock that dash-spv has not validated never makes a payment
//! final: not in memory, not in the store, and not after a restart.
//!
//! Everything below the network is real: dash-spv's `DashSpvClient` with its
//! InstantSend and mempool managers runs over `MockNetworkManager`, the
//! `PlatformWalletManager` owns the wallet manager and the wallet-event
//! adapter (`changeset/core_bridge.rs`), and the adapter writes to a
//! `SqlitePersister` in an owner-only directory. The test injects a payment
//! to the wallet and an `islock` for it that cannot be validated (the client
//! holds no masternode list, so no quorum of the lock's cycle), in three
//! orders, then checks:
//!
//! - the client held the lock back: it is pending and no `InstantLockReceived`
//!   was emitted for it;
//! - (i) the store has no `core_instant_locks` row with `verified = 1`. The
//!   writer stores every lock that reaches it as verified, so this means no
//!   lock reached the store, not that the store refused one;
//! - (ii) in memory, the record is Mempool and its coin is not InstantSend
//!   locked, confirmed, or counted in the confirmed balance;
//! - (iii) after everything is dropped and a fresh manager loads the same
//!   file, the same holds.
//!
//! The positive case runs against a regtest masternode network (dashd plus
//! rust-dashcore's pre-generated `regtest-mn` data): a lock the client
//! validates against a live quorum is final in memory, is stored with
//! `verified = 1`, and is final again after the reopen.

use crate::common::{ro_conn, secure_tempdir};

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dash_spv::error::NetworkResult;
use dash_spv::network::PeerNetworkManager;
use dash_spv::network::{Inbound, MessageType, NetworkEvent, NetworkManager, RequestKey};
use dash_spv::test_utils::{
    test_socket_address, MasternodeTestContext, MockNetworkManager, TestEventHandler,
};
use dash_spv::{ClientConfig, DashSpvClient, MempoolStrategy, SyncEvent, SyncState};
use dashcore::bls_sig_utils::BLSSignature;
use dashcore::ephemerealdata::instant_lock::InstantLock;
use dashcore::network::message::NetworkMessage;
use dashcore::{Address, Amount, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::Network;
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet::key_wallet_manager::WalletManager;
use platform_wallet::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use platform_wallet::PlatformWalletManager;
use platform_wallet_storage::{FlushMode, SqlitePersister, SqlitePersisterConfig};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{broadcast, RwLock};

const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const AMOUNT: u64 = 25_000_000;
const BARRIER: Duration = Duration::from_secs(60);
const SETTLE: Duration = Duration::from_secs(3);

/// The mock network, shared: the client owns its network by value, and the
/// test still needs a handle to inject messages through it.
struct SharedNetwork(Arc<MockNetworkManager>);

#[async_trait::async_trait]
impl NetworkManager for SharedNetwork {
    async fn start(&self) {
        self.0.start().await
    }
    async fn stop(&self) {
        self.0.stop().await
    }
    async fn enqueue(&self, height: u32, msg: NetworkMessage) {
        self.0.enqueue(height, msg).await
    }
    async fn send_to(&self, addr: SocketAddr, msg: NetworkMessage) -> bool {
        self.0.send_to(addr, msg).await
    }
    async fn broadcast(&self, msg: NetworkMessage) -> NetworkResult<()> {
        self.0.broadcast(msg).await
    }
    async fn dispatch_local(&self, msg: NetworkMessage) {
        self.0.dispatch_local(msg).await
    }
    async fn release_request(&self, key: RequestKey) {
        self.0.release_request(key).await
    }
    async fn subscribe(&self, kinds: &[MessageType]) -> UnboundedReceiver<Inbound> {
        self.0.subscribe(kinds).await
    }
    fn events(&self) -> broadcast::Receiver<NetworkEvent> {
        self.0.events()
    }
    fn tip(&self) -> u32 {
        self.0.tip()
    }
    async fn connected_count(&self) -> u32 {
        self.0.connected_count().await
    }
    async fn ban_peer(&self, addr: SocketAddr) {
        self.0.ban_peer(addr).await
    }
}

#[derive(Clone, Copy, Debug)]
enum Order {
    /// The lock is injected right behind the transaction, with no wait.
    Together,
    /// The InstantSend manager has handled the lock before the transaction is
    /// injected. Had it forwarded the lock, the mempool manager would get it
    /// on a separate event subscription, so which of its two paths (lock held
    /// for the transaction, or lock applied to it) a regression takes is not
    /// fixed.
    LockFirst,
    /// The transaction is in the wallet and the store before the lock arrives.
    TxFirst,
}

/// The `InstantLockReceived` events the client emits, as `(txid, validated)`.
#[derive(Default)]
struct LockEvents(std::sync::Mutex<Vec<(Txid, bool)>>);

impl dash_spv::EventHandler for LockEvents {
    fn on_sync_event(&self, event: &SyncEvent) {
        if let SyncEvent::InstantLockReceived {
            instant_lock,
            validated,
        } = event
        {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push((instant_lock.txid, *validated));
        }
    }
}

impl LockEvents {
    fn of(&self, txid: &Txid) -> Vec<bool> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|(t, _)| t == txid)
            .map(|(_, validated)| *validated)
            .collect()
    }
}

struct NoopHandler;
impl platform_wallet::events::EventHandler for NoopHandler {}
impl platform_wallet::PlatformEventHandler for NoopHandler {}

type Manager = PlatformWalletManager<SqlitePersister>;
type Client = DashSpvClient<WalletManager<PlatformWalletInfo>, SharedNetwork>;

fn open_persister(path: &Path) -> Arc<SqlitePersister> {
    Arc::new(
        SqlitePersister::open(
            SqlitePersisterConfig::new(path).with_flush_mode(FlushMode::Immediate),
        )
        .expect("open persister"),
    )
}

fn new_manager(persister: Arc<SqlitePersister>, network: Network) -> Manager {
    let sdk = Arc::new(
        dash_sdk::SdkBuilder::new_mock()
            .with_network(network)
            .build()
            .expect("mock sdk"),
    );
    PlatformWalletManager::new(sdk, persister, Arc::new(NoopHandler))
}

/// A payment of `AMOUNT` to `address` from an outpoint the wallet does not own.
fn payment_to(address: &Address) -> Transaction {
    Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: OutPoint::new(Txid::from_byte_array([0x42; 32]), 3),
            script_sig: ScriptBuf::new(),
            sequence: 0xffff_ffff,
            witness: Witness::default(),
        }],
        output: vec![TxOut {
            value: AMOUNT,
            script_pubkey: address.script_pubkey(),
        }],
        special_transaction_payload: None,
    }
}

/// A well-formed lock for `tx` whose cycle no quorum the client holds belongs
/// to: it passes the structure check and cannot be validated.
fn unvalidatable_lock(tx: &Transaction) -> InstantLock {
    InstantLock {
        inputs: vec![tx.input[0].previous_output],
        txid: tx.txid(),
        signature: BLSSignature::from([0x11; 96]),
        ..InstantLock::default()
    }
}

fn verified_lock_rows(path: &Path, txid: &Txid) -> i64 {
    ro_conn(path)
        .query_row(
            "SELECT COUNT(*) FROM core_instant_locks WHERE txid = ?1 AND verified = 1",
            rusqlite::params![txid.as_byte_array().as_slice()],
            |r| r.get(0),
        )
        .expect("count verified lock rows")
}

fn stored_lock_rows(path: &Path, txid: &Txid) -> Vec<i64> {
    let conn = ro_conn(path);
    let mut stmt = conn
        .prepare("SELECT verified FROM core_instant_locks WHERE txid = ?1")
        .expect("prepare");
    let rows = stmt
        .query_map(rusqlite::params![txid.as_byte_array().as_slice()], |r| {
            r.get::<_, i64>(0)
        })
        .expect("query lock rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("read lock rows");
    rows
}

fn stored_record_rows(path: &Path, txid: &Txid) -> i64 {
    ro_conn(path)
        .query_row(
            "SELECT COUNT(*) FROM core_transactions WHERE txid = ?1",
            rusqlite::params![txid.as_byte_array().as_slice()],
            |r| r.get(0),
        )
        .expect("count record rows")
}

/// What the wallet holds for the payment: its record's context, its coins'
/// `(is_instantlocked, is_confirmed)` flags, and the confirmed and
/// unconfirmed balance.
#[derive(Debug)]
struct WalletView {
    context: Option<TransactionContext>,
    coins: Vec<(bool, bool)>,
    confirmed: u64,
    unconfirmed: u64,
}

async fn wallet_view(
    wm: &RwLock<WalletManager<PlatformWalletInfo>>,
    wallet_id: &WalletId,
    txid: &Txid,
) -> WalletView {
    let wm = wm.read().await;
    let info = wm.get_wallet_info(wallet_id).expect("wallet info");
    let account = info
        .core_wallet
        .first_bip44_managed_account()
        .expect("bip44 account");
    let balance = WalletInfoInterface::balance(&info.core_wallet);
    WalletView {
        context: account.transactions().get(txid).map(|r| r.context.clone()),
        coins: account
            .utxos
            .values()
            .filter(|u| u.outpoint.txid == *txid)
            .map(|u| (u.is_instantlocked, u.is_confirmed))
            .collect(),
        confirmed: balance.confirmed(),
        unconfirmed: balance.unconfirmed(),
    }
}

/// The view after a reopen. The load restores coins and balances but does
/// not replay history into the accounts, so the record is the one the store
/// serves (`get_core_tx_record`, the read the asset-lock proof flow and
/// sent-payment reconciliation use).
async fn reopened_view(
    persister: &SqlitePersister,
    wm: &RwLock<WalletManager<PlatformWalletInfo>>,
    wallet_id: &WalletId,
    txid: &Txid,
) -> WalletView {
    let mut view = wallet_view(wm, wallet_id, txid).await;
    assert!(
        view.context.is_none(),
        "the load replayed history: {view:?}"
    );
    view.context = persister
        .get_core_tx_record(*wallet_id, txid)
        .expect("read the stored record")
        .map(|record| record.context);
    view
}

/// What makes the payment final, if anything: it must be in the wallet,
/// unconfirmed, and not final.
fn not_final_violation(view: &WalletView) -> Option<String> {
    let not_final = matches!(view.context, Some(TransactionContext::Mempool))
        && view.coins == [(false, false)]
        && view.confirmed == 0
        && view.unconfirmed == AMOUNT;
    (!not_final).then(|| {
        format!(
            "want a Mempool record, one coin neither InstantSend-locked nor \
             confirmed, confirmed balance 0 and unconfirmed {AMOUNT}; got {view:?}"
        )
    })
}

async fn wait_until<F, Fut>(what: &str, mut ready: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = Instant::now();
    while !ready().await {
        assert!(start.elapsed() < BARRIER, "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Shut the manager down and unregister the wallet, which is what lets the
/// manager release the persister: a registered wallet's handle and the
/// manager's handlers refer to each other.
async fn close(manager: &Manager, wallet: Arc<platform_wallet::PlatformWallet>) {
    let wallet_id = wallet.wallet_id();
    drop(wallet);
    assert!(
        manager.shutdown().await.all_clean(),
        "clean manager shutdown"
    );
    drop(
        manager
            .remove_wallet(&wallet_id)
            .await
            .expect("unregister the wallet"),
    );
}

/// Wait for every other holder of the persister to let go, so the file can
/// be opened again.
async fn release(
    persister: Arc<SqlitePersister>,
    wm: std::sync::Weak<RwLock<WalletManager<PlatformWalletInfo>>>,
) {
    let start = Instant::now();
    while Arc::strong_count(&persister) > 1 {
        assert!(
            start.elapsed() < BARRIER,
            "the persister is still held: {} strong references, wallet manager {} strong",
            Arc::strong_count(&persister),
            wm.strong_count()
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn instantsend_pending(client: &Client) -> usize {
    client
        .progress()
        .await
        .instantsend()
        .map(|p| p.pending())
        .unwrap_or(0)
}

async fn run(order: Order) {
    let tmp = secure_tempdir().expect("owner-only tempdir");
    let db = tmp.path().join("wallet.db");
    let spv_dir = tmp.path().join("spv");

    let persister = open_persister(&db);
    let manager = new_manager(Arc::clone(&persister), Network::Testnet);
    let wallet = manager
        .create_wallet_from_mnemonic(
            MNEMONIC,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
            None,
        )
        .await
        .expect("create wallet");
    let wallet_id = wallet.wallet_id();
    let address = wallet
        .core()
        .next_receive_address_for_account(0)
        .await
        .expect("receive address");
    let wm = manager.wallet_manager_arc();

    let net = Arc::new(MockNetworkManager::new());
    let config = ClientConfig::testnet()
        .without_filters()
        .with_mempool_tracking(MempoolStrategy::FetchAll)
        .with_storage_path(&spv_dir);
    assert!(
        config.enable_masternodes,
        "the InstantSend manager needs masternodes on"
    );
    let lock_events = Arc::new(LockEvents::default());
    let client: Client = DashSpvClient::new(
        config,
        SharedNetwork(Arc::clone(&net)),
        Arc::clone(&wm),
        vec![Arc::clone(&lock_events) as Arc<dyn dash_spv::EventHandler>],
    )
    .await
    .expect("spv client");
    client.run().await.expect("run spv client");

    let tx = payment_to(&address);
    let txid = tx.txid();
    let lock = unvalidatable_lock(&tx);
    let peer = test_socket_address(1);
    let tx_msg = || NetworkMessage::Tx(Box::new(tx.clone()));
    let lock_msg = || NetworkMessage::ISLock(lock.clone());

    let (wm_ref, db_ref, client_ref) = (&*wm, db.as_path(), &client);
    let tx_seen = || async move {
        wallet_view(wm_ref, &wallet_id, &txid)
            .await
            .context
            .is_some()
            && stored_record_rows(db_ref, &txid) == 1
    };
    // The InstantSend manager has handled the lock: it holds it pending
    // (deferred, or kept for re-validation) or has emitted it.
    let events_ref = &*lock_events;
    let lock_seen = || async move {
        instantsend_pending(client_ref).await >= 1 || !events_ref.of(&txid).is_empty()
    };

    match order {
        Order::Together => {
            net.inject(peer, tx_msg()).await;
            net.inject(peer, lock_msg()).await;
            wait_until("the payment in the wallet and the store", tx_seen).await;
            wait_until("the lock at the InstantSend manager", lock_seen).await;
        }
        Order::LockFirst => {
            net.inject(peer, lock_msg()).await;
            wait_until("the lock at the InstantSend manager", lock_seen).await;
            net.inject(peer, tx_msg()).await;
            wait_until("the payment in the wallet and the store", tx_seen).await;
        }
        Order::TxFirst => {
            net.inject(peer, tx_msg()).await;
            wait_until("the payment in the wallet and the store", tx_seen).await;
            net.inject(peer, lock_msg()).await;
            wait_until("the lock at the InstantSend manager", lock_seen).await;
        }
    }

    // A forwarded lock reaches the wallet and the store within milliseconds;
    // give it SETTLE to do so. Each check records its first violation, so a
    // failing run reports all of them.
    let mut store_violation = None;
    let mut memory_violation = None;
    let start = Instant::now();
    while start.elapsed() < SETTLE && (store_violation.is_none() || memory_violation.is_none()) {
        let rows = verified_lock_rows(&db, &txid);
        if store_violation.is_none() && rows != 0 {
            store_violation = Some(format!(
                "{rows} core_instant_locks row(s) with verified = 1"
            ));
        }
        if memory_violation.is_none() {
            memory_violation = not_final_violation(&wallet_view(&wm, &wallet_id, &txid).await);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // Why it holds: the client kept the lock pending and emitted nothing.
    let events = lock_events.of(&txid);
    let pending = instantsend_pending(&client).await;
    let spv_violation = (!events.is_empty() || pending != 1).then(|| {
        format!("InstantLockReceived events (validated) {events:?}, {pending} lock(s) pending")
    });

    client.stop().await;
    drop(client);
    close(&manager, wallet).await;
    let wm_probe = Arc::downgrade(&wm);
    drop(wm);
    drop(manager);
    release(persister, wm_probe).await;

    // A fresh process: the same file, a new persister and manager.
    let persister = open_persister(&db);
    let rows = verified_lock_rows(&db, &txid);
    let reopened_store_violation =
        (rows != 0).then(|| format!("{rows} core_instant_locks row(s) with verified = 1"));
    let manager = new_manager(Arc::clone(&persister), Network::Testnet);
    manager
        .load_from_persistor()
        .await
        .expect("load from the store");
    let wm = manager.wallet_manager_arc();
    let reopen_violation =
        not_final_violation(&reopened_view(&persister, &wm, &wallet_id, &txid).await);
    drop(wm);
    assert!(
        manager.shutdown().await.all_clean(),
        "clean manager shutdown"
    );

    eprintln!(
        "{order:?}: InstantLockReceived events for the payment (validated): {events:?}; \
         lock rows after reopen (verified column): {:?}",
        stored_lock_rows(&db, &txid)
    );
    let violations: Vec<String> = [
        ("SPV held the lock back", spv_violation),
        ("(i) store, live", store_violation),
        ("(ii) memory, live", memory_violation),
        ("(i) store, after reopen", reopened_store_violation),
        ("(iii) after reopen", reopen_violation),
    ]
    .into_iter()
    .filter_map(|(check, violation)| violation.map(|v| format!("{check}: {v}")))
    .collect();
    assert!(
        violations.is_empty(),
        "{order:?}: an unvalidated lock made the payment final:\n{}",
        violations.join("\n")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn g1_unvalidated_lock_with_its_transaction_is_never_final() {
    run(Order::Together).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn g1_unvalidated_lock_before_its_transaction_is_never_final() {
    run(Order::LockFirst).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn g1_unvalidated_lock_after_its_transaction_is_never_final() {
    run(Order::TxFirst).await;
}

/// A lock the client validates against a live regtest quorum.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs dashd and rust-dashcore's regtest-mn data (DASHD_PATH, DASHD_MN_DATADIR)"]
async fn g1_validated_lock_is_final_and_survives_a_reopen() {
    let mut mn = MasternodeTestContext::new(false)
        .await
        .expect("a dashd masternode network (unset SKIP_DASHD_TESTS)");
    let tmp = secure_tempdir().expect("owner-only tempdir");
    let db = tmp.path().join("wallet.db");
    let spv_dir = tmp.path().join("spv");

    let persister = open_persister(&db);
    let manager = new_manager(Arc::clone(&persister), Network::Regtest);
    let wallet = manager
        .create_wallet_from_mnemonic(
            MNEMONIC,
            Network::Regtest,
            WalletAccountCreationOptions::Default,
            None,
        )
        .await
        .expect("create wallet");
    let wallet_id = wallet.wallet_id();
    let wm = manager.wallet_manager_arc();

    let mut config = ClientConfig::regtest().with_storage_path(&spv_dir);
    config.peers.clear();
    config.add_peer(mn.controller_addr);
    let handler = Arc::new(TestEventHandler::new());
    let mut sync_events = handler.subscribe_sync_events();
    let lock_events = Arc::new(LockEvents::default());
    let client = DashSpvClient::new(
        config.clone(),
        PeerNetworkManager::new(&config).await,
        Arc::clone(&wm),
        vec![
            handler.clone(),
            Arc::clone(&lock_events) as Arc<dyn dash_spv::EventHandler>,
        ],
    )
    .await
    .expect("spv client");
    client.run().await.expect("run spv client");
    let client_ref = &client;

    wait_until("masternode sync", || async move {
        client_ref
            .progress()
            .await
            .masternodes()
            .is_ok_and(|p| p.state() == SyncState::Synced)
    })
    .await;
    let baseline = client
        .progress()
        .await
        .masternodes()
        .expect("masternode progress")
        .current_height();

    // The pre-generated quorums are in the past; a DKG cycle forms one that
    // signs locks now.
    tokio::task::block_in_place(|| mn.mine_dkg_cycle()).expect("DKG cycle");
    let start = Instant::now();
    loop {
        let remaining = BARRIER.saturating_sub(start.elapsed());
        match tokio::time::timeout(remaining, sync_events.recv()).await {
            Err(_) => panic!("no masternode state above height {baseline}"),
            Ok(Ok(SyncEvent::MasternodeStateUpdated { height, .. })) if height > baseline => break,
            Ok(Ok(_)) | Ok(Err(broadcast::error::RecvError::Lagged(_))) => {}
            Ok(Err(e)) => panic!("sync events: {e}"),
        }
    }
    wait_until("mempool relay", || async move {
        client_ref
            .progress()
            .await
            .mempool()
            .is_ok_and(|p| p.state() == SyncState::Synced)
    })
    .await;

    let address = wallet
        .core()
        .next_receive_address_for_account(0)
        .await
        .expect("receive address");
    let txid = tokio::task::block_in_place(|| {
        mn.controller
            .send_to_address(&address, Amount::from_sat(AMOUNT))
    });

    // One mocktime bump starts the masternodes' signing; polling on real time
    // keeps their signing sessions alive (dash-spv's own dashd tests do the same).
    tokio::task::block_in_place(|| mn.bump_mocktime(30));
    wait_until("the controller's lock", || {
        let locked = tokio::task::block_in_place(|| {
            mn.controller
                .try_rpc_call("getrawtransaction", &[txid.to_string().into(), 1.into()])
                .and_then(|raw| raw.get("instantlock").and_then(|v| v.as_bool()))
                .unwrap_or(false)
        });
        async move { locked }
    })
    .await;

    let (wm_ref, db_ref) = (&*wm, db.as_path());
    wait_until("the payment InstantSend-final in memory", || async move {
        matches!(
            wallet_view(wm_ref, &wallet_id, &txid).await.context,
            Some(TransactionContext::InstantSend(_))
        )
    })
    .await;
    // At this pin dash-spv emits a lock only once it has validated it. The
    // event handlers run on their own task, so the event may trail the wallet.
    let events_ref = &*lock_events;
    wait_until("the payment's InstantLockReceived event", || async move {
        !events_ref.of(&txid).is_empty()
    })
    .await;
    assert_eq!(
        lock_events.of(&txid),
        vec![true],
        "the payment's lock must reach the wallet as validated, once"
    );
    wait_until("a verified lock row", || async move {
        verified_lock_rows(db_ref, &txid) == 1
    })
    .await;
    assert_final(&wallet_view(&wm, &wallet_id, &txid).await, "in memory");
    eprintln!(
        "validated: lock rows for the payment (verified column): {:?}",
        stored_lock_rows(&db, &txid)
    );

    client.stop().await;
    drop(client);
    close(&manager, wallet).await;
    let wm_probe = Arc::downgrade(&wm);
    drop(wm);
    drop(manager);
    release(persister, wm_probe).await;

    let persister = open_persister(&db);
    assert_eq!(
        verified_lock_rows(&db, &txid),
        1,
        "after reopen: the verified row"
    );
    let manager = new_manager(Arc::clone(&persister), Network::Regtest);
    manager
        .load_from_persistor()
        .await
        .expect("load from the store");
    let wm = manager.wallet_manager_arc();
    assert_final(
        &reopened_view(&persister, &wm, &wallet_id, &txid).await,
        "after reopen",
    );
    drop(wm);
    assert!(
        manager.shutdown().await.all_clean(),
        "clean manager shutdown"
    );
}

/// The payment is InstantSend-final: its record carries the lock, and its
/// coin is locked and counted in the confirmed balance.
fn assert_final(view: &WalletView, what: &str) {
    assert!(
        matches!(view.context, Some(TransactionContext::InstantSend(_))),
        "{what}: the record must be InstantSend, got {:?}",
        view.context
    );
    assert_eq!(
        view.coins,
        vec![(true, false)],
        "{what}: one InstantSend-locked coin"
    );
    assert_eq!(
        view.confirmed, AMOUNT,
        "{what}: the payment counts as confirmed balance"
    );
}
