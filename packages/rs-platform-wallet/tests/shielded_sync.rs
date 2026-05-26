//! Functional test for the SDK_TEST_DATA seeded shielded pool.
//!
//! Drives the **full PlatformWalletManager flow** end-to-end: create wallet
//! → bind shielded → trigger a sync pass via the network coordinator → check
//! the recovered balance. Goes through the same APIs production wallets use,
//! so any breakage in `bind_shielded`, `NetworkShieldedCoordinator::sync`, the
//! gRPC layer, or proof verification surfaces here.
//!
//! The in-process unit tests in `rs-drive-abci/.../create_genesis_state/test/
//! shielded.rs` prove crypto + drive integration are correct. This test
//! closes the remaining gap by running a real wallet against a real chain.
//!
//! # Expected chain config
//!
//! The seed config is hardcoded on the chain side (see
//! `ShieldedSeedConfig::sdk_test_data` in rs-drive-abci):
//! `total_notes = 5_000, owned_count = 8 (split 4/4), owned_value = 100_000,
//! rng_seed = 0xDEAD_BEEF`. Each wallet's expected balance after sync is
//! `4 × 100_000 = 400_000`.
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
//! DASH_SDK_CORE_PASSWORD='<password>' cargo test -p platform-wallet \
//!     --test shielded_sync --features shielded -- --ignored --nocapture
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
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;

/// Wallet A seed — **must stay byte-identical** to
/// `packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/test/shielded_test_wallets.rs::SEED_A`.
///
/// Both sides derive the IVK via the same ZIP-32 path
/// (`SpendingKey::from_zip32_seed(seed, coin_type=1, account=0)`), so the
/// recipient address the chain encrypts to is byte-identical to the address
/// the wallet's IVK trial-decrypts under. If the chain-side switches
/// derivation, this test fails with "decrypted 0 notes".
const SEED_A: [u8; 32] = [0x73; 32];

/// Wallet B seed — see [`SEED_A`].
const SEED_B: [u8; 32] = [0x74; 32];

/// Hardcoded mirror of `ShieldedSeedConfig::sdk_test_data` on the chain side
/// (rs-drive-abci). If the chain-side constant changes, update both — there's
/// no shared crate to import from (rs-drive-abci is downstream of this crate).
const COUNT_A: u32 = 4;
const COUNT_B: u32 = 4;
const OWNED_VALUE: u64 = 100_000;

const EXPECTED_BALANCE_A: u64 = COUNT_A as u64 * OWNED_VALUE; // 400_000
const EXPECTED_BALANCE_B: u64 = COUNT_B as u64 * OWNED_VALUE; // 400_000

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

/// In-memory no-op persister. Real wallets persist; for this test we only
/// care that a single sync pass recovers the right balance.
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
/// the recovered balance equals what the chain's seed config produces.
async fn run_wallet_balance_test(wallet: WalletIndex) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .try_init();

    let (expected_count, expected_balance) = match wallet {
        WalletIndex::A => (COUNT_A, EXPECTED_BALANCE_A),
        WalletIndex::B => (COUNT_B, EXPECTED_BALANCE_B),
    };
    eprintln!(
        "{:?}: expecting {} notes summing to {}",
        wallet, expected_count, expected_balance,
    );

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
        event_handler,
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
    //        BIP-39-style 64-byte seed and is immaterial for this test
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
            transparent_seed,
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

    // --- 5. Bind the shielded sub-wallet with the SAME seed bytes the
    //        chain-side seeder uses, deriving via ZIP-32 (account 0) ---
    let coordinator = manager
        .shielded_coordinator()
        .await
        .expect("shielded_coordinator must exist after configure_shielded");
    platform_wallet
        .bind_shielded(&shielded_seed, &[0u32], &coordinator)
        .await
        .expect("bind_shielded");

    // --- 6. Run a single sync pass through the coordinator. `force = true`
    //        skips the cooldown gate so the test runs immediately after the
    //        chain comes up. ---
    let summary = coordinator.sync(true).await;
    eprintln!("{:?}: sync summary: {:?}", wallet, summary);

    // --- 7. Read the wallet's shielded balance per ZIP-32 account. We bound
    //        account 0 only, so we expect exactly one entry. ---
    let balances = platform_wallet
        .shielded_balances(&coordinator)
        .await
        .expect("shielded_balances");
    let total_balance: u64 = balances.values().sum();
    eprintln!(
        "{:?}: per-account balances = {:?} (total {})",
        wallet, balances, total_balance
    );

    assert_eq!(
        total_balance, expected_balance,
        "{:?}: balance mismatch (expected {} = {} × {}, got {})",
        wallet, expected_balance, expected_count, OWNED_VALUE, total_balance,
    );

    // Best-effort cleanup of the temp SQLite dir.
    let _ = std::fs::remove_dir_all(&shielded_db_dir);
}

/// Sync wallet A against the seeded pool and verify balance =
/// `count_a × owned_value`.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "requires a SDK_TEST_DATA devnet — see file header"]
async fn wallet_a_recovers_deterministic_balance_via_manager_sync() {
    run_wallet_balance_test(WalletIndex::A).await;
}

/// Sync wallet B against the seeded pool and verify balance =
/// `count_b × owned_value`. Together with the wallet-A test, this also pins
/// cross-wallet privacy at the network layer — if A's IVK leaked over the
/// wire and B picked up A's notes, B's balance would exceed `count_b × value`.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "requires a SDK_TEST_DATA devnet — see file header"]
async fn wallet_b_recovers_deterministic_balance_via_manager_sync() {
    run_wallet_balance_test(WalletIndex::B).await;
}

#[cfg(test)]
mod constants {
    //! Sanity-checks for the hardcoded expectations. Always-on tests so the
    //! test binary compiles even when no devnet is up, and the math stays
    //! coherent if someone bumps the constants.

    use super::*;

    #[test]
    fn expected_balance_matches_count_times_value() {
        assert_eq!(EXPECTED_BALANCE_A, COUNT_A as u64 * OWNED_VALUE);
        assert_eq!(EXPECTED_BALANCE_B, COUNT_B as u64 * OWNED_VALUE);
    }

    #[test]
    fn wallet_seeds_are_distinct() {
        assert_ne!(SEED_A, SEED_B);
    }
}
