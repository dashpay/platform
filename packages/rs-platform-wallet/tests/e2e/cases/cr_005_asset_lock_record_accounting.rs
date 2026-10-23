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
//! Checked at both observation points a host sees, on the shared harness
//! persister (a real [`SqlitePersister`] the wallet sees unmasked, so the
//! manager drives its durable Core path):
//!
//! 1. **Live** — every folded wallet-level record the manager hands the
//!    persister in `CoreChangeSet::records` (what the FFI / Swift layer
//!    stores and displays).
//! 2. **After reload** — the row the store returns from
//!    `get_core_tx_record` after reopening the database from disk, plus the
//!    wallet itself in the reopened store's start state (`WALLET_RESTORE`).
//!
//! Unlike CR-003 / ID-002b, the lock is only built and proven, never
//! consumed on Platform; teardown sweeps the unused lock and the remaining
//! Core balance back to the bank.
//!
//! [`SqlitePersister`]: platform_wallet_storage::SqlitePersister

use platform_wallet::AssetLockFundingType;

use crate::framework::prelude::*;
use crate::framework::signer::SeedBackedCoreSigner;
use crate::framework::tx_accounting::assert_tracked_lock_accounting;

/// Core duffs the bank sends to the test wallet.
const TEST_WALLET_CORE_FUNDING: u64 = 20_000_000;

/// Duffs locked into the asset-lock output.
const ASSET_LOCK_AMOUNT: u64 = 5_000_000;

/// BIP-44 account the lock is funded from and the identity index its
/// funding key is derived for.
const FUNDING_ACCOUNT_INDEX: u32 = 0;
const IDENTITY_INDEX: u32 = 0;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn cr_005_asset_lock_record_accounting() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Step 1: fund the test wallet's Core account from the bank.
    let s = crate::framework::setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)
        .await
        .expect("setup_with_core_funded_test_wallet failed");
    let network = s.ctx.config.network;
    let wallet = s.test_wallet.platform_wallet();
    let wallet_id = wallet.wallet_id();

    // Step 2: build, broadcast and prove an asset lock funded by the wallet.
    let core_signer = SeedBackedCoreSigner::new(s.test_wallet.seed_bytes(), network);
    let (_proof, _path, out_point) = wallet
        .asset_locks()
        .create_funded_asset_lock_proof(
            ASSET_LOCK_AMOUNT,
            FUNDING_ACCOUNT_INDEX,
            AssetLockFundingType::IdentityRegistration,
            IDENTITY_INDEX,
            &core_signer,
        )
        .await
        .expect("create_funded_asset_lock_proof");
    tracing::info!(
        target: "platform_wallet::e2e::cases::cr_005",
        lock_txid = %out_point.txid,
        "CR-005: asset lock proven"
    );
    let lock = wallet
        .asset_locks()
        .list_tracked_locks()
        .await
        .into_iter()
        .find(|lock| lock.out_point == out_point)
        .expect("the proven asset lock must be tracked");
    assert_eq!(lock.amount, ASSET_LOCK_AMOUNT, "tracked lock amount");

    // Step 3: every stored record and the row reread after a reload —
    // Internal, net == -(lock + fee).
    assert_tracked_lock_accounting(&s.ctx.persister, wallet_id, &lock).await;

    // Step 4: the reopened store still holds the wallet itself.
    let start_state = s
        .ctx
        .persister
        .reloaded_start_state()
        .expect("load reopened harness store");
    assert!(
        start_state.wallets.contains_key(&wallet_id),
        "reopened store must still hold the CR-005 wallet"
    );

    s.teardown().await.expect("teardown");
}
