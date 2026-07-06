//! Functional example for the SDK_TEST_DATA seeded shielded pool.
//!
//! Drives the **full PlatformWalletManager flow** end-to-end: create wallet
//! → bind shielded → trigger a sync pass via the network coordinator → check
//! the pass completed cleanly. Goes through the same APIs production wallets
//! use, so any breakage in `bind_shielded`,
//! `NetworkShieldedCoordinator::sync`, the gRPC layer, or proof verification
//! surfaces here.
//!
//! This is an opt-in example (NOT a test) because it performs real network
//! I/O against a running devnet; cargo examples are compiled but never run by
//! `cargo test`.
//!
//! The in-process unit tests in `rs-drive-abci/.../create_genesis_state/test/
//! shielded.rs` prove crypto + drive integration are correct. This example
//! closes the remaining gap by running a real wallet against a real chain.
//!
//! # Expected chain config
//!
//! The seed config is hardcoded on the chain side (see
//! `ShieldedSeedConfig::sdk_test_data` in rs-drive-abci's
//! `create_genesis_state/test/shielded.rs`). All seeded notes are
//! **filler-only** — the genesis test-wallet note seeding was removed, so no
//! seeded note decrypts under any wallet's IVK and a fresh devnet
//! legitimately reports balance 0. What a clean run guarantees instead is
//! that the sync pass walked the seeded pool (`total_scanned > 0`) without
//! errors; balances are printed informationally. For a non-zero balance,
//! fund the wallet with real shielded transitions post-genesis.
//!
//! # Requirements
//!
//! 1. A running devnet whose genesis was created with SDK test data. The
//!    `local` dashmate config has `buildArgs.SDK_TEST_DATA = "true"` set
//!    automatically by `yarn setup`, so:
//!    ```bash
//!    yarn reset && yarn start
//!    ```
//!
//! 2. `DASH_SDK_CORE_PASSWORD` set to the dashmate Core RPC password
//!    (find it in `~/.dashmate/config.json` under
//!    `local_1.core.rpc.users.dashmate.password`).
//!
//! 3. Optional `PLATFORM_HOST` / `PLATFORM_PORT` to override the default
//!    devnet endpoint (`127.0.0.1:2443`).
//!
//! # Running
//!
//! ```bash
//! # Wallet A (default):
//! DASH_SDK_CORE_PASSWORD='<password>' cargo run -p platform-wallet \
//!     --example shielded_sync --features shielded
//!
//! # Wallet B (second, independent ZIP-32 derivation):
//! SHIELDED_SYNC_WALLET=B DASH_SDK_CORE_PASSWORD='<password>' \
//!     cargo run -p platform-wallet --example shielded_sync --features shielded
//! ```

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
use platform_wallet::manager::shielded_sync::WalletShieldedOutcome;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;

/// Wallet A seed. Historically this mirrored the chain-side test-wallet
/// seeding (removed — see the `ShieldedSeedConfig::sdk_test_data`
/// doc-comment in rs-drive-abci's `create_genesis_state/test/shielded.rs`);
/// today it's just a deterministic seed so the wallet ID is reproducible
/// across runs. Seeded notes are filler-only, so trial decryption under
/// this wallet's IVK finding nothing is expected.
const SEED_A: [u8; 32] = [0x73; 32];

/// Wallet B seed — see [`SEED_A`]. Distinct from A so the `B` run exercises
/// a second, independent ZIP-32 derivation.
const SEED_B: [u8; 32] = [0x74; 32];

#[derive(Clone, Copy, Debug)]
enum WalletIndex {
    A,
    B,
}

impl WalletIndex {
    fn seed(self) -> [u8; 32] {
        match self {
            WalletIndex::A => SEED_A,
            WalletIndex::B => SEED_B,
        }
    }
}

/// In-memory no-op persister. Real wallets persist; for this example we only
/// care that a single sync pass completes cleanly.
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

/// Drive the full PlatformWalletManager flow for the given wallet and assert
/// the sync pass completed cleanly (no errors, not a cooldown skip, seeded
/// pool actually scanned).
async fn run_wallet_sync_test(wallet: WalletIndex) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .try_init();

    // --- 1. Build SDK pointing at the local devnet ---
    // Dashmate's local gateway issues SHA-1-signed certs that modern rustls
    // rejects, so the conventional rs-sdk test pattern is to talk HTTP
    // (gateway accepts both on the same port). Default `PLATFORM_SSL=false`
    // matches `packages/rs-sdk/tests/.env.example`.
    let host = std::env::var("PLATFORM_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PLATFORM_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2443);
    let use_ssl = std::env::var("PLATFORM_SSL")
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    let scheme = if use_ssl { "https" } else { "http" };
    let address: Address = format!("{}://{}:{}", scheme, host, port)
        .parse()
        .expect("parse devnet address");
    let addresses = AddressList::from_iter([address]);

    // Core RPC credentials — the SDK needs these to fetch quorum public keys
    // for proof verification. With `with_core(...)` and no explicit
    // `with_context_provider`, the SDK auto-installs `GrpcContextProvider`
    // which uses Core RPC under the hood. Defaults assume dashmate's
    // local_1/seed node; override via `DASH_SDK_CORE_*` env vars (same names
    // the rs-sdk fetch tests use).
    let core_host = std::env::var("DASH_SDK_CORE_HOST").unwrap_or_else(|_| host.clone());
    let core_port: u16 = std::env::var("DASH_SDK_CORE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20002);
    let core_user = std::env::var("DASH_SDK_CORE_USER").unwrap_or_else(|_| "dashmate".to_string());
    let core_password = std::env::var("DASH_SDK_CORE_PASSWORD").unwrap_or_default();

    let network = Network::Regtest;
    let mut builder = SdkBuilder::new(addresses).with_network(network).with_core(
        &core_host,
        core_port,
        &core_user,
        &core_password,
    );

    // If the operator explicitly opted into SSL, load dashmate's CA cert
    // (overridable via `DASHMATE_CA_CERT`).
    if use_ssl {
        let ca_cert_path = std::env::var("DASHMATE_CA_CERT").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
            format!("{}/.dashmate/local_1/platform/gateway/ssl/bundle.crt", home)
        });
        if std::path::Path::new(&ca_cert_path).exists() {
            eprintln!("loading CA cert from {}", ca_cert_path);
            builder = builder
                .with_ca_certificate_file(&ca_cert_path)
                .expect("load CA cert file");
        }
    }
    eprintln!(
        "connecting to platform {}://{}:{}, core rpc {}:{}@{}",
        scheme, host, port, core_user, core_port, core_host
    );

    let sdk = builder.build().expect("build sdk");
    let sdk = Arc::new(sdk);

    // --- 2. Build the manager ---
    // `PlatformWalletManager::new` is generic over `P: PlatformWalletPersistence`,
    // so the persister must stay concrete (an `Arc<dyn ...>` would erase the
    // type param and break inference downstream).
    let persister = Arc::new(NoopPersister);
    let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    let manager = Arc::new(PlatformWalletManager::new(
        Arc::clone(&sdk),
        persister,
        vec![event_handler],
    ));

    // --- 3. Configure shielded support (creates the SQLite store) ---
    let shielded_db_dir = std::env::temp_dir().join(format!(
        "platform-wallet-shielded-test-{:?}-{}",
        wallet,
        std::process::id()
    ));
    std::fs::create_dir_all(&shielded_db_dir).expect("mkdir shielded db dir");
    let shielded_db_path = shielded_db_dir.join("shielded.db");

    manager
        .configure_shielded(&shielded_db_path)
        .await
        .expect("configure_shielded");

    // --- 4. Create a platform wallet. The transparent-layer seed is a
    //        BIP-39-style 64-byte seed and is immaterial for this example
    //        (we never spend or query transparent state); we duplicate
    //        the 32-byte shielded seed into a deterministic 64-byte
    //        pattern so the wallet ID is reproducible per `wallet`. ---
    let shielded_seed = wallet.seed();
    let mut transparent_seed = [0u8; 64];
    transparent_seed[..32].copy_from_slice(&shielded_seed);
    transparent_seed[32..].copy_from_slice(&shielded_seed);
    let platform_wallet = manager
        .create_wallet_from_seed_bytes(
            network,
            &transparent_seed,
            WalletAccountCreationOptions::Default,
            None,
        )
        .await
        .expect("create_wallet_from_seed_bytes");

    eprintln!(
        "{:?}: created platform wallet id = {}",
        wallet,
        hex::encode(platform_wallet.wallet_id())
    );

    // --- 5. Bind the shielded sub-wallet, deriving via ZIP-32
    //        (account 0) ---
    let coordinator = manager
        .shielded_coordinator()
        .await
        .expect("shielded_coordinator must exist after configure_shielded");
    platform_wallet
        .bind_shielded(&shielded_seed, &[0u32], &coordinator)
        .await
        .expect("bind_shielded");

    // --- 6. Run a single sync pass through the coordinator. `force = true`
    //        skips the cooldown gate so the example runs immediately after
    //        the chain comes up. ---
    let summary = coordinator.sync(true).await;
    eprintln!("{:?}: sync summary: {:?}", wallet, summary);

    // --- 7. Assert the pass completed cleanly for our wallet: a genuine
    //        network walk (not a cooldown skip) that streamed the seeded
    //        pool's commitments without erroring. Balance is NOT asserted —
    //        seeded notes are filler-only, so 0 is the expected balance on
    //        a fresh devnet. ---
    let outcome = summary
        .wallet_results
        .get(&platform_wallet.wallet_id())
        .expect("sync pass must report an outcome for the bound wallet");
    let wallet_summary = match outcome {
        WalletShieldedOutcome::Ok(s) => s,
        other => panic!(
            "{:?}: sync pass did not complete cleanly: {:?}",
            wallet, other
        ),
    };
    assert!(
        !wallet_summary.is_cooldown_skip,
        "{:?}: pass was a cooldown skip despite force=true",
        wallet,
    );
    assert!(
        wallet_summary.notes_result.total_scanned > 0,
        "{:?}: scanned 0 commitments — devnet not built with SDK_TEST_DATA=true?",
        wallet,
    );

    // --- 8. Read the wallet's shielded balance per ZIP-32 account,
    //        informationally. We bound account 0 only, so we expect exactly
    //        one entry. ---
    let balances = platform_wallet
        .shielded_balances(&coordinator)
        .await
        .expect("shielded_balances");
    let total_balance: u64 = balances.values().sum();
    eprintln!(
        "{:?}: per-account balances = {:?} (total {}; informational — seeded notes are filler-only)",
        wallet, balances, total_balance
    );

    // Best-effort cleanup of the temp SQLite dir.
    let _ = std::fs::remove_dir_all(&shielded_db_dir);
}

/// Sync a wallet against the seeded pool and verify the pass completed
/// cleanly (no errors, not a cooldown skip, `total_scanned > 0`).
///
/// Defaults to wallet A; set `SHIELDED_SYNC_WALLET=B` to run wallet B
/// instead, exercising a second, independent ZIP-32 derivation.
#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    let wallet = match std::env::var("SHIELDED_SYNC_WALLET")
        .unwrap_or_default()
        .trim()
        .to_ascii_uppercase()
        .as_str()
    {
        "" | "A" => WalletIndex::A,
        "B" => WalletIndex::B,
        other => panic!("invalid SHIELDED_SYNC_WALLET `{other}`; expected A or B"),
    };

    run_wallet_sync_test(wallet).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Catch an accidental aliasing of the two seeds — the `B` run is
    /// only meaningful if it derives a distinct wallet.
    #[test]
    fn wallet_seeds_are_distinct() {
        assert_ne!(SEED_A, SEED_B);
    }
}
