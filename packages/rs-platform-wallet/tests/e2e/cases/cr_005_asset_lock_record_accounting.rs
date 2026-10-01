//! CR-005 — Asset-lock transaction accounting, live and after reload.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Core (CR) → CR-005).
//!
//! Pins the wallet-level accounting of an asset lock the wallet funds
//! itself. From the wallet's side the locked duffs become its own Platform
//! credits, so only the fee leaves the wallet:
//!
//! - `direction == Internal`
//! - `net_amount == -(lock amount + fee)`
//!
//! Checked at both observation points a host sees:
//!
//! 1. **Live** — the folded wallet-level record the manager hands the
//!    persister in `CoreChangeSet::records` (what the FFI / Swift layer
//!    stores and displays).
//! 2. **After reload** — the row a real [`SqlitePersister`] returns from
//!    `get_core_tx_record` after reopening the database from disk (a
//!    consistent `backup_to` snapshot: the live handle cannot be reopened
//!    in-process, `SqlitePersister::open` refuses a double open).
//!
//! The shared harness persister (CR-003 / ID-002b / AL-001 check the same
//! accounting through it) attests only atomic changesets and shielded
//! viewing keys, so the manager drives its non-durable Core paths. This case covers the durable one: its own manager on
//! a real SQLite store (capabilities attested), with its own SPV client
//! (SPV storage under `<workdir>/cr_005/`, the SQLite store in a private
//! temporary directory), the way
//! `found_coinjoin_gap_limit_sync` drives its own runtime.
//!
//! Funding: the bank sends [`WALLET_CORE_FUNDING`] duffs to a fresh seed.
//! The lock is never consumed on Platform ([`ASSET_LOCK_AMOUNT`] stays
//! locked); the rest is swept back to the bank best-effort at the end.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dashcore::Txid;
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::transaction_record::{TransactionDirection, TransactionRecord};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use platform_wallet::changeset::changeset::DpnsNameStateEntry;
use platform_wallet::changeset::traits::ListedCoreTxid;
use platform_wallet::changeset::{
    ClientStartState, PersistenceCapabilities, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::events::PlatformEventHandler;
use platform_wallet::masternode::TrackedMasternode;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::{AssetLockFundingType, PlatformWallet, PlatformWalletManager};
use platform_wallet_storage::{FlushMode, SqlitePersister, SqlitePersisterConfig};
use rand::rngs::OsRng;
use rand::RngCore;

use crate::framework::bank::core_send_from_account;
use crate::framework::harness_persister::private_temp_dir;
use crate::framework::prelude::*;
use crate::framework::signer::SeedBackedCoreSigner;
use crate::framework::spv::{self, MnListErrorObserver};
use crate::framework::tx_accounting::{
    assert_asset_lock_accounting, assert_every_asset_lock_record,
};

/// Core duffs the bank sends to the case's own wallet.
const WALLET_CORE_FUNDING: u64 = 20_000_000;

/// Duffs locked into the asset-lock output.
const ASSET_LOCK_AMOUNT: u64 = 5_000_000;

/// Deadline for the case's own SPV client to finish mn-list sync from a
/// cold store.
const SPV_READY_TIMEOUT: Duration = Duration::from_secs(900);

/// Deadline for the bank's send to reach the case wallet's confirmed balance.
const FUNDING_TIMEOUT: Duration = Duration::from_secs(300);

/// Deadline for the asset-lock record to reach the persister.
const RECORD_TIMEOUT: Duration = Duration::from_secs(120);

/// Duffs left behind by the best-effort sweep to cover its own fee.
const SWEEP_FEE_HEADROOM: u64 = 20_000;

/// SQLite persister that also keeps every wallet-level core record the
/// manager stores, so the case can read the live projection.
struct CapturingPersister {
    inner: SqlitePersister,
    records: Mutex<Vec<TransactionRecord>>,
}

impl CapturingPersister {
    /// The latest stored wallet-level record for `txid`, if any.
    fn latest_record(&self, txid: &Txid) -> Option<TransactionRecord> {
        self.stored_records(txid).pop()
    }

    /// Every stored wallet-level record for `txid`, oldest first.
    fn stored_records(&self, txid: &Txid) -> Vec<TransactionRecord> {
        self.records
            .lock()
            .expect("captured records poisoned")
            .iter()
            .filter(|r| r.txid == *txid)
            .cloned()
            .collect()
    }
}

impl PlatformWalletPersistence for CapturingPersister {
    fn store_commits_inline(&self) -> bool {
        self.inner.store_commits_inline()
    }

    fn persistence_capabilities(&self) -> PersistenceCapabilities {
        self.inner.persistence_capabilities()
    }

    fn store_transient_is_reissuable(&self) -> bool {
        self.inner.store_transient_is_reissuable()
    }

    fn store(
        &self,
        wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        if let Some(core) = &changeset.core {
            self.records
                .lock()
                .expect("captured records poisoned")
                .extend(core.records.iter().cloned());
        }
        self.inner.store(wallet_id, changeset)
    }

    fn flush(&self, wallet_id: WalletId) -> Result<(), PersistenceError> {
        self.inner.flush(wallet_id)
    }

    fn persist_tracked_masternodes(
        &self,
        network: dashcore::Network,
        records: &[TrackedMasternode],
    ) -> Result<(), PersistenceError> {
        self.inner.persist_tracked_masternodes(network, records)
    }

    fn load_tracked_masternodes(
        &self,
        network: dashcore::Network,
    ) -> Result<Vec<TrackedMasternode>, PersistenceError> {
        self.inner.load_tracked_masternodes(network)
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        self.inner.load()
    }

    fn get_core_tx_record(
        &self,
        wallet_id: WalletId,
        txid: &Txid,
    ) -> Result<Option<TransactionRecord>, PersistenceError> {
        self.inner.get_core_tx_record(wallet_id, txid)
    }

    fn list_wallet_core_txids(
        &self,
        wallet_id: WalletId,
    ) -> Result<Option<Vec<ListedCoreTxid>>, PersistenceError> {
        self.inner.list_wallet_core_txids(wallet_id)
    }

    fn get_dpns_name_state(
        &self,
        wallet_id: WalletId,
        wallet_identity_id: &dpp::prelude::Identifier,
        normalized_label: &str,
    ) -> Result<Option<DpnsNameStateEntry>, PersistenceError> {
        self.inner
            .get_dpns_name_state(wallet_id, wallet_identity_id, normalized_label)
    }
}

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn cr_005_asset_lock_record_accounting() {
    let ctx = E2eContext::init().await.expect("e2e framework init");
    if ctx.skip_if_bank_floor_unmet("cr_005") {
        return;
    }
    let network = ctx.config.network;

    // Step 1: a manager on a real SQLite store, with its own SPV client.
    let case_dir = ctx.workdir.join("cr_005");
    // A previous run's store and SPV data would hand this run a stale
    // wallet and a warm-but-foreign chain state; start clean.
    let _ = std::fs::remove_dir_all(&case_dir);
    std::fs::create_dir_all(&case_dir).expect("create CR-005 workdir");
    // SQLite refuses a group-writable ancestor, which the workdir may be.
    let db_dir = private_temp_dir("platform-wallet-e2e-cr005-").expect("CR-005 private dir");
    let db_path = db_dir.path().join("wallet.db");
    let sqlite = SqlitePersister::open(
        SqlitePersisterConfig::new(&db_path)
            .with_flush_mode(FlushMode::Immediate)
            .with_auto_backup_dir(None),
    )
    .expect("open CR-005 SQLite store");
    let persister = Arc::new(CapturingPersister {
        inner: sqlite,
        records: Mutex::new(Vec::new()),
    });
    let mn_list_observer = Arc::new(MnListErrorObserver::new());
    let manager = Arc::new(PlatformWalletManager::new(
        Arc::clone(ctx.sdk()),
        Arc::clone(&persister),
        vec![Arc::clone(&mn_list_observer) as Arc<dyn PlatformEventHandler>],
    ));

    let mut seed = [0u8; 64];
    OsRng.fill_bytes(&mut seed);
    let wallet = manager
        .create_wallet_from_seed_bytes(network, &seed, WalletAccountCreationOptions::Default, None)
        .await
        .expect("create CR-005 wallet");

    let spv_runtime = spv::start_spv(&manager, &ctx.config, &case_dir, ctx.sdk().address_list())
        .await
        .expect("start CR-005 SPV");
    spv::wait_for_mn_list_synced(&spv_runtime, &mn_list_observer, SPV_READY_TIMEOUT)
        .await
        .expect("CR-005 SPV mn-list sync");

    // Step 2: fund the wallet from the bank.
    let core_recv = wallet
        .core()
        .next_receive_address_for_account(0)
        .await
        .expect("derive CR-005 Core receive address");
    let funding_txid = ctx
        .bank()
        .send_core_to(&core_recv, WALLET_CORE_FUNDING)
        .await
        .expect("bank.send_core_to CR-005 wallet");
    tracing::info!(
        target: "platform_wallet::e2e::cases::cr_005",
        %funding_txid,
        "CR-005: bank funding broadcast"
    );
    wait_for_confirmed_balance(&wallet, WALLET_CORE_FUNDING, FUNDING_TIMEOUT).await;

    // Step 3: build, broadcast and prove an asset lock funded by the wallet.
    let core_signer = SeedBackedCoreSigner::new(seed, network);
    let (_proof, _path, out_point) = wallet
        .asset_locks()
        .create_funded_asset_lock_proof(
            ASSET_LOCK_AMOUNT,
            0,
            AssetLockFundingType::IdentityRegistration,
            0,
            &core_signer,
        )
        .await
        .expect("create_funded_asset_lock_proof");
    let lock_txid = out_point.txid;
    tracing::info!(
        target: "platform_wallet::e2e::cases::cr_005",
        %lock_txid,
        "CR-005: asset lock proven"
    );

    // Step 4 (live): the wallet-level record the manager stored.
    let live = wait_for_captured_record(&persister, &lock_txid, RECORD_TIMEOUT).await;
    assert_asset_lock_accounting(&live, ASSET_LOCK_AMOUNT);
    // Every stored row, not only the latest: a host shows each as it lands.
    assert_every_asset_lock_record(&persister.stored_records(&lock_txid), ASSET_LOCK_AMOUNT);

    // Step 5 (reload): reopen the store from a disk snapshot and read the
    // persisted row back.
    let snapshot: PathBuf = persister
        .inner
        .backup_to(&db_dir.path().join("reload.db"))
        .expect("snapshot CR-005 store");
    let reloaded_store =
        SqlitePersister::open(SqlitePersisterConfig::new(&snapshot).with_auto_backup_dir(None))
            .expect("reopen CR-005 store snapshot");
    let start_state = reloaded_store.load().expect("load reopened CR-005 store");
    assert!(
        start_state.wallets.contains_key(&wallet.wallet_id()),
        "reopened store must still hold the CR-005 wallet"
    );
    let reloaded = reloaded_store
        .get_core_tx_record(wallet.wallet_id(), &lock_txid)
        .expect("read asset-lock row from reopened store")
        .expect("asset-lock row must survive a reload");
    assert_asset_lock_accounting(&reloaded, ASSET_LOCK_AMOUNT);
    assert_eq!(
        (reloaded.direction, reloaded.net_amount),
        (live.direction, live.net_amount),
        "a reload must not change the asset lock's accounting"
    );
    assert_eq!(live.direction, TransactionDirection::Internal);

    // Teardown: sweep what is left back to the bank (best-effort), then stop.
    sweep_to_bank(ctx, &wallet, &core_signer).await;
    let _ = manager.shutdown().await;
}

/// Poll the wallet's confirmed Core balance until it reaches `min`.
async fn wait_for_confirmed_balance(wallet: &PlatformWallet, min: u64, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        let confirmed = wallet.balance().confirmed();
        if confirmed >= min {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "CR-005 wallet confirmed balance {confirmed} did not reach {min} within {timeout:?}"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Poll the capturing persister until a record for `txid` was stored.
async fn wait_for_captured_record(
    persister: &CapturingPersister,
    txid: &Txid,
    timeout: Duration,
) -> TransactionRecord {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(record) = persister.latest_record(txid) {
            return record;
        }
        assert!(
            Instant::now() < deadline,
            "no wallet-level record for asset lock {txid} reached the persister within {timeout:?}"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Send the wallet's remaining confirmed balance back to the bank. Failures
/// are logged, not fatal: the assertions above already ran.
async fn sweep_to_bank(
    ctx: &E2eContext,
    wallet: &Arc<PlatformWallet>,
    signer: &SeedBackedCoreSigner,
) {
    let remaining = wallet.balance().confirmed();
    let Some(amount) = remaining.checked_sub(SWEEP_FEE_HEADROOM).filter(|a| *a > 0) else {
        return;
    };
    let result = async {
        let bank_addr = ctx.bank().primary_core_receive_address().await?;
        core_send_from_account(
            wallet,
            StandardAccountType::BIP44Account,
            0,
            vec![(bank_addr, amount)],
            signer,
        )
        .await
        .map_err(|e| FrameworkError::Wallet(format!("CR-005 sweep: {e}")))
    }
    .await;
    if let Err(error) = result {
        tracing::warn!(
            target: "platform_wallet::e2e::cases::cr_005",
            %error,
            remaining,
            "CR-005: best-effort sweep back to the bank failed"
        );
    }
}
