//! Offline check of the harness persister's accounting observation points.
//!
//! `framework::harness_persister::HarnessPersister` backs the live and
//! after-reload asset-lock accounting checks in CR-003 / ID-002b / AL-001.
//! Those only run against a live network, so this case pins the plumbing
//! offline: a wallet registered through a manager on the persister, then
//! one wallet-level Core record stored for it, must be readable both as the
//! live record and from a reopened SQLite snapshot, unchanged.

use std::sync::Arc;

use dashcore::Transaction;
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::transaction_record::{
    InputDetail, OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::transaction_router::TransactionType;
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::{AccountType, Network};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::PlatformWalletManager;

use crate::framework::harness_persister::HarnessPersister;

const TEST_SEED: [u8; 64] = [11u8; 64];
const SPENT: u64 = 1_000_000;
const CHANGE: u64 = 400_000;
const SENT: u64 = 599_000;

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

// Multi-thread: `PlatformWalletManager::shutdown` joins worker threads via
// `Handle::block_on`, which panics on a current-thread runtime.
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 4)]
async fn harness_persister_round_trips_a_wallet_record() {
    let persister = Arc::new(HarnessPersister::open());
    let manager = PlatformWalletManager::new(
        Arc::new(dash_sdk::Sdk::new_mock()),
        Arc::clone(&persister),
        vec![Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>],
    );
    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &TEST_SEED,
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("register wallet through the harness persister");
    let wallet_id = wallet.wallet_id();
    let ours = wallet
        .core()
        .next_receive_address_for_account(0)
        .await
        .expect("derive receive address");

    let transaction = Transaction::dummy(&ours, 0..1, &[CHANGE, SENT]);
    let mut record = TransactionRecord::new(
        transaction,
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        TransactionContext::Mempool,
        TransactionType::Standard,
        TransactionDirection::Outgoing,
        vec![InputDetail {
            index: 0,
            value: SPENT,
            address: ours.clone(),
        }],
        vec![OutputDetail {
            index: 0,
            role: OutputRole::Change,
            address: Some(ours),
            value: CHANGE,
        }],
        CHANGE as i64 - SPENT as i64,
    );
    record.fee = Some(SPENT - CHANGE - SENT);
    let txid = record.txid;

    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: vec![record.clone()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("the harness persister never fails a store");

    let live = persister
        .live_record(wallet_id, &txid)
        .expect("stored record is captured live");
    assert_eq!(
        (live.direction, live.net_amount),
        (record.direction, record.net_amount)
    );

    let reloaded = persister
        .reloaded_record(wallet_id, &txid)
        .expect("reload from the tee store")
        .expect("stored record has a row after a reload");
    assert_eq!(reloaded.txid, txid);
    assert_eq!(
        (reloaded.direction, reloaded.net_amount),
        (record.direction, record.net_amount),
        "a reload must return the stored accounting unchanged"
    );

    let shutdown = manager.shutdown().await;
    assert!(shutdown.all_clean(), "manager teardown: {shutdown:?}");
}

/// The harness persister attests `SHIELDED_FVK_RESTART` (atomic changesets
/// plus shielded viewing keys) and honours it: a real `bind_shielded`
/// succeeds through it, and the viewing key it persisted comes back from
/// `load` — the path a seedless rebind reads.
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 4)]
async fn harness_persister_honours_shielded_fvk_restart() {
    use platform_wallet::changeset::PersistenceCapabilities;
    use platform_wallet::wallet::shielded::{
        FileBackedShieldedStore, NetworkShieldedCoordinator, SubwalletId,
    };

    const SHIELDED_SEED: [u8; 64] = [13u8; 64];

    let persister = Arc::new(HarnessPersister::open());
    assert!(
        persister
            .persistence_capabilities()
            .contains(PersistenceCapabilities::SHIELDED_FVK_RESTART),
        "the harness persister must attest what bind_shielded requires"
    );

    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    let manager = PlatformWalletManager::new(
        Arc::clone(&sdk),
        Arc::clone(&persister),
        vec![Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>],
    );
    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &SHIELDED_SEED,
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("register wallet through the harness persister");

    let tree_dir = tempfile::tempdir().expect("tempdir");
    let tree_path = tree_dir.path().join("tree.sqlite");
    let store = FileBackedShieldedStore::open_path(&tree_path, 100).expect("open tree store");
    let coordinator = Arc::new(NetworkShieldedCoordinator::new(
        Arc::clone(&sdk),
        Network::Testnet,
        tree_path,
        store,
    ));

    wallet
        .bind_shielded(&SHIELDED_SEED, &[0], &coordinator)
        .await
        .expect("bind_shielded through the harness persister");

    let loaded = persister.load().expect("load");
    assert!(
        loaded
            .shielded
            .viewing_keys
            .contains_key(&SubwalletId::new(wallet.wallet_id(), 0)),
        "the viewing key bind_shielded persisted must come back from load"
    );
    assert!(
        loaded.wallets.is_empty(),
        "wallets stay session-scoped: WALLET_RESTORE is not attested"
    );

    let shutdown = manager.shutdown().await;
    assert!(shutdown.all_clean(), "manager teardown: {shutdown:?}");
}
