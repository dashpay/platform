//! Remote-devnet variant of `shielded_sync.rs`. Binds wallet A
//! (raw ZIP-32 seed `[0x73; 32]`) against a live devnet that's been
//! deployed from a `SDK_TEST_DATA=true` image, drives one shielded sync
//! pass, and asserts the recovered balance.
//!
//! Diverges from [`shielded_sync.rs`] in two ways:
//!
//! 1. **No Core RPC.** The remote devnet's Core RPC ports are
//!    firewalled, so we can't use `SdkBuilder::with_core(...)`. Instead
//!    we install a [`TrustedHttpContextProvider`] that pulls quorum
//!    metadata from a separate HTTP service. Same approach the iOS
//!    SwiftExampleApp uses for Devnet (see `SDK.swift::platformQuorumURL`).
//!
//! 2. **Multi-node DAPI fan-out.** Lists every active masternode's
//!    Platform HTTP port. `AddressList` picks randomly per request
//!    (`rs-dapi-client/src/address_list.rs:213`), so distributing the
//!    gRPC traffic across the full masternode set avoids funneling
//!    every encrypted-note stream through one gateway.
//!
//! # Hardcoded for paloma
//!
//! Both the quorum URL and the DAPI address list default to paloma's
//! deployment (May 2026). Override via env vars listed below.
//!
//! # Requirements
//!
//! - Paloma devnet (or another devnet built from `SDK_TEST_DATA=true`)
//!   reachable from this host. Verify with:
//!   ```bash
//!   curl http://44.238.203.84:8080/masternodes | jq '.data | length'
//!   ```
//!
//! # Running
//!
//! ```bash
//! cargo test -p platform-wallet --test shielded_sync_paloma --features shielded \
//!     -- --ignored --nocapture
//! ```
//!
//! Env overrides (all optional):
//! - `PALOMA_QUORUM_URL` — defaults to `http://44.238.203.84:8080`
//! - `PALOMA_DAPI_ADDRESSES` — comma-separated `https://<ip>:1443` list
//!
//! # What this proves
//!
//! Green: ZIP-32 derivation + bind + decryption + persistence work
//! end-to-end against paloma. If the iOS SwiftExampleApp shows balance
//! = 0 against the same devnet, the bug is iOS-side (persister
//! callback, SwiftData write, UI aggregation).
//!
//! Red on "decrypted 0 notes": paloma was built without
//! `SDK_TEST_DATA=true`, or from a commit predating the snapshot
//! machinery — its 1M notes are filler-only.
//!
//! Red on proof / network errors: quorum URL or DAPI addresses are
//! wrong or unreachable.

#![cfg(feature = "shielded")]

use std::sync::Arc;

use dash_sdk::sdk::{Address, AddressList};
use dash_sdk::SdkBuilder;
use dashcore::Network;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use platform_wallet::changeset::{
    ClientStartState, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

/// Wallet A seed — must stay byte-identical to
/// `packages/rs-drive-abci/.../shielded_test_wallets.rs::SEED_A`.
const SEED_A: [u8; 32] = [0x73; 32];

/// Per-wallet bake config (mirror of `ShieldedSeedConfig::sdk_test_data`).
const COUNT_A: u32 = 4;
const OWNED_VALUE: u64 = 100_000;
const EXPECTED_BALANCE_A: u64 = COUNT_A as u64 * OWNED_VALUE; // 400_000

/// Default quorum-list service URL for paloma. Override with
/// `PALOMA_QUORUM_URL`.
const DEFAULT_QUORUM_URL: &str = "http://44.238.203.84:8080";

/// All 13 active paloma masternodes' Platform HTTP gateways, as of
/// May 2026 (snapshot from
/// `curl http://44.238.203.84:8080/masternodes | jq -r '.data[].address'`
/// with the Platform HTTP port `1443`). Override with
/// `PALOMA_DAPI_ADDRESSES` if the deployment changes.
const DEFAULT_DAPI_ADDRESSES: &[&str] = &[
    "https://68.67.122.85:1443",
    "https://68.67.122.86:1443",
    "https://68.67.122.87:1443",
    "https://68.67.122.88:1443",
    "https://68.67.122.192:1443",
    "https://68.67.122.193:1443",
    "https://68.67.122.195:1443",
    "https://68.67.122.196:1443",
    "https://68.67.122.197:1443",
    "https://68.67.122.198:1443",
    "https://68.67.122.199:1443",
    "https://68.67.122.206:1443",
    "https://68.67.122.207:1443",
];

/// In-memory no-op persister — matches the same in-test stub from
/// `shielded_sync.rs`. We only need a single sync pass to compute the
/// balance; persisted state across runs is irrelevant.
struct NoopPersister;
impl PlatformWalletPersistence for NoopPersister {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: PlatformWalletChangeSet,
    ) -> Result<(), platform_wallet::changeset::PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, platform_wallet::changeset::PersistenceError> {
        Ok(ClientStartState::default())
    }

    fn flush(
        &self,
        _wallet_id: WalletId,
    ) -> Result<(), platform_wallet::changeset::PersistenceError> {
        Ok(())
    }
}

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

/// Parse `PALOMA_DAPI_ADDRESSES` (comma-separated) or fall back to the
/// hardcoded list. Trims whitespace per entry, skips empties.
fn dapi_addresses() -> AddressList {
    let raw = std::env::var("PALOMA_DAPI_ADDRESSES").unwrap_or_default();
    let addrs: Vec<Address> = if raw.trim().is_empty() {
        DEFAULT_DAPI_ADDRESSES
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect()
    } else {
        raw.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    assert!(
        !addrs.is_empty(),
        "no DAPI addresses configured (PALOMA_DAPI_ADDRESSES empty and default list parse failed?)"
    );
    AddressList::from_iter(addrs)
}

fn quorum_url() -> String {
    std::env::var("PALOMA_QUORUM_URL").unwrap_or_else(|_| DEFAULT_QUORUM_URL.to_string())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "requires a reachable SDK_TEST_DATA devnet — see file header"]
async fn wallet_a_recovers_balance_on_paloma() {
    let _ = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .try_init();

    let addresses = dapi_addresses();
    let quorum = quorum_url();
    let live_count = addresses.get_live_addresses().len();
    eprintln!(
        "paloma config: quorum_url={} dapi_node_count={} expected_balance={}",
        quorum, live_count, EXPECTED_BALANCE_A,
    );

    // --- 1. Build SDK pointing at all paloma masternodes, with the
    //        trusted context provider handling proof verification via
    //        the HTTP quorum-list service. ---
    let provider = TrustedHttpContextProvider::new_with_url(
        Network::Devnet,
        quorum.clone(),
        std::num::NonZeroUsize::new(100).unwrap(),
    )
    .expect("build TrustedHttpContextProvider");

    let sdk = SdkBuilder::new(addresses)
        .with_network(Network::Devnet)
        .with_context_provider(provider)
        .build()
        .expect("build sdk");
    let sdk = Arc::new(sdk);

    // --- 2. Manager + shielded coordinator with an ephemeral SQLite DB. ---
    let persister = Arc::new(NoopPersister);
    let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    let manager = Arc::new(PlatformWalletManager::new(
        Arc::clone(&sdk),
        persister,
        event_handler,
    ));

    let shielded_db_dir = std::env::temp_dir().join(format!(
        "platform-wallet-shielded-paloma-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&shielded_db_dir).expect("mkdir shielded db dir");
    let shielded_db_path = shielded_db_dir.join("shielded.db");

    manager
        .configure_shielded(&shielded_db_path)
        .await
        .expect("configure_shielded");

    // --- 3. Create the platform wallet. Transparent seed is immaterial
    //        for this test; reuse the 32-byte shielded seed twice. ---
    let mut transparent_seed = [0u8; 64];
    transparent_seed[..32].copy_from_slice(&SEED_A);
    transparent_seed[32..].copy_from_slice(&SEED_A);
    let platform_wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Devnet,
            transparent_seed,
            WalletAccountCreationOptions::Default,
            // birth_height_override: skip SPV-tip lookup (no SPV running here)
            Some(0),
        )
        .await
        .expect("create_wallet_from_seed_bytes");

    eprintln!(
        "wallet_a created: wallet_id = {}",
        hex::encode(platform_wallet.wallet_id())
    );

    // --- 4. Bind shielded account 0 with the raw seed (mirrors the
    //        iOS `bindShieldedRawSeed` path; same FFI shape). ---
    let coordinator = manager
        .shielded_coordinator()
        .await
        .expect("shielded_coordinator must exist after configure_shielded");
    platform_wallet
        .bind_shielded(&SEED_A, &[0u32], &coordinator)
        .await
        .expect("bind_shielded");

    // --- 5. Run one sync pass with cooldown bypassed. ---
    let summary = coordinator.sync(true).await;
    eprintln!("sync summary: {:?}", summary);

    // --- 6. Read the per-account balance + assert. ---
    let balances = platform_wallet
        .shielded_balances(&coordinator)
        .await
        .expect("shielded_balances");
    let total_balance: u64 = balances.values().sum();
    eprintln!(
        "paloma wallet A balances = {:?} (total {})",
        balances, total_balance,
    );

    assert_eq!(
        total_balance, EXPECTED_BALANCE_A,
        "wallet A balance mismatch on paloma (expected {} = {} × {}, got {})",
        EXPECTED_BALANCE_A, COUNT_A, OWNED_VALUE, total_balance,
    );

    let _ = std::fs::remove_dir_all(&shielded_db_dir);
}
