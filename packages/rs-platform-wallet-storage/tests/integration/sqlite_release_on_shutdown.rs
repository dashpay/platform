//! A manager that is shut down and dropped must release its SQLite persister,
//! so the host can open the same file again in this process.
//!
//! The registry that refuses a second in-process open (`AlreadyOpen`) frees a
//! path only when the last handle on its persister drops. Anything that keeps
//! a registered wallet alive past the manager therefore keeps the file locked
//! too: each wallet holds the persister. These tests release nothing by hand
//! (no `remove_wallet`), so they fail on any reference cycle back to a wallet
//! or the persister.

use std::path::Path;
use std::sync::Arc;

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::{PlatformEventHandler, PlatformWalletManager};
use platform_wallet_storage::WalletStorageError;

use crate::common::{secure_tempdir, FlushMode, SqlitePersister, SqlitePersisterConfig};

const MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon about";

struct Noop;
impl platform_wallet::events::EventHandler for Noop {}
impl PlatformEventHandler for Noop {}

fn open(path: &Path) -> Result<SqlitePersister, WalletStorageError> {
    SqlitePersister::open(SqlitePersisterConfig::new(path).with_flush_mode(FlushMode::Immediate))
}

fn manager(persister: &Arc<SqlitePersister>) -> PlatformWalletManager<SqlitePersister> {
    let sdk = Arc::new(
        dash_sdk::SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .build()
            .expect("mock sdk"),
    );
    PlatformWalletManager::new(sdk, Arc::clone(persister), Arc::new(Noop))
}

/// Shut `manager` down cleanly, then drop it as a host drops its handle.
async fn shut_down_and_drop(manager: PlatformWalletManager<SqlitePersister>) {
    assert!(
        manager.shutdown().await.all_clean(),
        "clean manager shutdown"
    );
    drop(manager);
}

/// Register a wallet through the public lifecycle, then shut the manager down
/// and drop it. Returns the wallet id; the caller keeps its persister handle.
async fn register_then_shut_down(persister: &Arc<SqlitePersister>) -> WalletId {
    let manager = manager(persister);
    let without_wallet = Arc::strong_count(persister);
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
    drop(wallet);
    assert!(
        manager.get_wallet(&wallet_id).await.is_some(),
        "the wallet is registered when the manager shuts down"
    );
    assert!(
        Arc::strong_count(persister) > without_wallet,
        "the registered wallet holds the persister before shutdown"
    );
    shut_down_and_drop(manager).await;
    wallet_id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_same_file_reopens_after_the_manager_is_shut_down_and_dropped() {
    let tmp = secure_tempdir().expect("owner-only tempdir");
    let path = tmp.path().join("wallet.db");

    let persister = Arc::new(open(&path).expect("first open"));
    register_then_shut_down(&persister).await;
    drop(persister);

    let reopened = open(&path);
    assert!(
        reopened.is_ok(),
        "reopening the file after shutdown and drop must succeed, got {:?}",
        reopened.err()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_the_host_holds_the_persister_after_a_created_wallet_shuts_down() {
    let tmp = secure_tempdir().expect("owner-only tempdir");
    let path = tmp.path().join("wallet.db");

    let persister = Arc::new(open(&path).expect("first open"));
    register_then_shut_down(&persister).await;
    assert_eq!(
        Arc::strong_count(&persister),
        1,
        "after a created wallet's manager is shut down and dropped, \
         only the test may hold the persister"
    );
}

/// The load path builds its wallets apart from the create path, so it gets
/// its own check. The file it loads is a backup of one a manager created a
/// wallet in, so the seed step's release is not what this test depends on.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_the_host_holds_the_persister_after_a_loaded_wallet_shuts_down() {
    let tmp = secure_tempdir().expect("owner-only tempdir");
    let seed_path = tmp.path().join("seed.db");
    let path = tmp.path().join("wallet.db");

    let seed = Arc::new(open(&seed_path).expect("open the seed file"));
    let wallet_id = register_then_shut_down(&seed).await;
    seed.backup_to(&path).expect("copy the seeded file");
    drop(seed);

    let persister = Arc::new(open(&path).expect("open the copy"));
    let manager = manager(&persister);
    let without_wallet = Arc::strong_count(&persister);
    manager
        .load_from_persistor()
        .await
        .expect("load from SQLite");
    assert!(
        manager.get_wallet(&wallet_id).await.is_some(),
        "the load registers the stored wallet"
    );
    assert!(
        Arc::strong_count(&persister) > without_wallet,
        "the loaded wallet holds the persister before shutdown"
    );
    shut_down_and_drop(manager).await;
    assert_eq!(
        Arc::strong_count(&persister),
        1,
        "after a loaded wallet's manager is shut down and dropped, \
         only the test may hold the persister"
    );
}

/// A shielded spend that may have executed keeps a redrive row and the note
/// reservations it carries. Releasing the manager at shutdown must purge none
/// of it: the release only drops references, and never goes through wallet
/// removal, which deletes a wallet's redrive rows.
#[cfg(feature = "shielded")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stored_redrive_and_its_holds_survive_release_and_reopen() {
    use key_wallet::mnemonic::Mnemonic;
    use platform_wallet::wallet::shielded::store::PendingRedrive;
    use platform_wallet::wallet::shielded::{FileBackedShieldedStore, ShieldedStore, SubwalletId};

    const NULLIFIER: [u8; 32] = [0x3C; 32];
    const ANCHOR: [u8; 32] = [0x2B; 32];
    const ACTIVITY: [u8; 32] = [0x1A; 32];
    const ANCHOR_HEIGHT: u64 = 4_200;

    fn redrive_rows(path: &Path, id: SubwalletId) -> Vec<(Vec<u8>, Vec<u8>, Option<i64>)> {
        let conn = crate::common::ro_conn(path);
        let mut stmt = conn
            .prepare(
                "SELECT activity_id, nullifiers, anchor_height FROM shielded_pending_spends \
                 WHERE wallet_id = ?1 AND account_index = ?2",
            )
            .expect("prepare");
        stmt.query_map(
            rusqlite::params![id.wallet_id.as_slice(), id.account_index],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
    }

    let tmp = secure_tempdir().expect("owner-only tempdir");
    let path = tmp.path().join("wallet.db");
    let shielded_path = tmp.path().join("shielded.sqlite");

    let persister = Arc::new(open(&path).expect("first open"));
    let manager = manager(&persister);
    let wallet = manager
        .create_wallet_from_mnemonic(
            MNEMONIC,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
            None,
        )
        .await
        .expect("create wallet");
    let id = SubwalletId::new(wallet.wallet_id(), 0);
    manager
        .configure_shielded(&shielded_path)
        .await
        .expect("configure shielded");
    let coordinator = manager
        .shielded_coordinator()
        .await
        .expect("coordinator configured");
    let seed = Mnemonic::from_phrase(MNEMONIC)
        .expect("mnemonic")
        .to_seed("");
    wallet
        .bind_shielded(&seed[..], &[0], &coordinator)
        .await
        .expect("bind shielded account 0");
    drop(wallet);

    let redrive = PendingRedrive {
        activity_id: ACTIVITY,
        anchor: ANCHOR,
        nullifiers: vec![NULLIFIER],
        st_bytes: vec![0xAB; 96],
        attempts: 0,
        identity_nonce_finalized: false,
        identity_user_abandoned: false,
        anchor_height: Some(ANCHOR_HEIGHT),
    };
    let held = vec![(NULLIFIER, ANCHOR, Some(ACTIVITY), Some(ANCHOR_HEIGHT))];
    {
        let mut store = coordinator.store().write().await;
        assert!(store
            .mark_pending(id, &NULLIFIER)
            .expect("reserve the note"));
        store
            .set_pending_spend(id, &NULLIFIER, ANCHOR, ACTIVITY, Some(ANCHOR_HEIGHT))
            .expect("attach the spend");
        store
            .arm_redrive(id, redrive.clone())
            .expect("arm the redrive");
        assert_eq!(store.stale_pending_spends(id).expect("holds"), held);
    }
    let stored = redrive_rows(&shielded_path, id);
    assert_eq!(
        stored,
        vec![(
            ACTIVITY.to_vec(),
            NULLIFIER.to_vec(),
            Some(ANCHOR_HEIGHT as i64)
        )],
        "the redrive row is on disk before shutdown"
    );

    let coordinator_weak = Arc::downgrade(&coordinator);
    drop(coordinator);
    shut_down_and_drop(manager).await;

    assert!(
        coordinator_weak.upgrade().is_none(),
        "the shielded coordinator, and the store it owns, are released"
    );
    let holders = Arc::strong_count(&persister);
    let persister_weak = Arc::downgrade(&persister);
    drop(persister);
    let reopened = open(&path);

    assert_eq!(
        redrive_rows(&shielded_path, id),
        stored,
        "the redrive row survives shutdown and release"
    );
    let store = FileBackedShieldedStore::open_path(&shielded_path, 100).expect("reopen store");
    assert_eq!(
        store.pending_redrives(id).expect("redrives"),
        vec![redrive],
        "the reopened store reports the redrive"
    );
    assert_eq!(
        store.stale_pending_spends(id).expect("holds"),
        held,
        "the reopened store restores the note reservation from the redrive"
    );

    assert!(
        reopened.is_ok(),
        "reopening the wallet file after shutdown and drop must succeed, got {:?} \
         with {holders} strong references on the persister",
        reopened.err()
    );
    assert_eq!(
        holders, 1,
        "after shutdown and drop only the test may hold the persister"
    );
    assert!(
        persister_weak.upgrade().is_none(),
        "the persister is released"
    );
}
