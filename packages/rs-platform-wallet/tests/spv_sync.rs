//! Minimal SPV sync integration test.
//!
//! Creates a PlatformWalletManager, registers a wallet from a mnemonic,
//! starts SPV, waits for sync, and verifies the wallet has a non-zero balance.
//!
//! # Requirements
//! - `SPV_TEST_MNEMONIC` env var: 12/24-word BIP-39 mnemonic for a funded testnet wallet
//! - Network access to Dash testnet peers (DNS seed discovery)
//!
//! # Running
//! ```bash
//! SPV_TEST_MNEMONIC="word1 word2 ... word12" \
//!   cargo test -p platform-wallet --test spv_sync -- --ignored --nocapture
//! ```

use std::sync::{Arc, Mutex};
use std::time::Duration;

use dash_spv::client::config::MempoolStrategy;
use dash_spv::types::ValidationMode;
use dash_spv::ClientConfig;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::Network;
use platform_wallet::changeset::{
    ClientStartState, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;

/// Recording persister that captures all store calls for test verification.
struct RecordingPersister {
    /// Each entry: (wallet_id, had core changeset, synced_height from core changeset)
    records: Mutex<Vec<(WalletId, bool, Option<u32>)>>,
}

impl RecordingPersister {
    fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    /// Number of store calls that contained a core wallet changeset.
    fn core_store_count(&self) -> usize {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, has_core, _)| *has_core)
            .count()
    }

    /// The highest synced_height seen across all store calls.
    fn max_synced_height(&self) -> Option<u32> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(_, _, h)| *h)
            .max()
    }
}

impl PlatformWalletPersistence for RecordingPersister {
    fn store(
        &self,
        wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), platform_wallet::changeset::PersistenceError> {
        let has_core = changeset.core.is_some();
        let synced_height = changeset.core.as_ref().and_then(|c| c.synced_height);
        self.records
            .lock()
            .unwrap()
            .push((wallet_id, has_core, synced_height));
        Ok(())
    }

    fn flush(
        &self,
        _wallet_id: WalletId,
    ) -> Result<(), platform_wallet::changeset::PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, platform_wallet::changeset::PersistenceError> {
        Ok(ClientStartState::default())
    }
}

/// No-op event handler for tests.
struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

/// No-op context provider — we only need SPV, not platform queries.
struct NoopContextProvider;
impl dash_sdk::platform::ContextProvider for NoopContextProvider {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], dash_sdk::error::ContextProviderError> {
        Err(dash_sdk::error::ContextProviderError::Config(
            "not available in SPV-only test".into(),
        ))
    }

    fn get_data_contract(
        &self,
        _id: &dpp::prelude::Identifier,
        _platform_version: &dpp::version::PlatformVersion,
    ) -> Result<Option<Arc<dpp::data_contract::DataContract>>, dash_sdk::error::ContextProviderError>
    {
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _id: &dpp::prelude::Identifier,
    ) -> Result<
        Option<dpp::data_contract::associated_token::token_configuration::TokenConfiguration>,
        dash_sdk::error::ContextProviderError,
    > {
        Ok(None)
    }

    fn get_platform_activation_height(&self) -> Result<u32, dash_sdk::error::ContextProviderError> {
        Ok(0)
    }
}

/// Testnet DAPI addresses (from dash-evo-tool .env.example).
const TESTNET_DAPI_ADDRESSES: &[&str] = &[
    "https://68.67.122.1:1443",
    "https://68.67.122.2:1443",
    "https://68.67.122.3:1443",
];

#[ignore]
#[tokio::test]
async fn test_spv_sync_and_balance() {
    // --- Logging ---
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .init();

    // --- Setup ---
    let mnemonic_str = std::env::var("SPV_TEST_MNEMONIC")
        .expect("SPV_TEST_MNEMONIC env var required (BIP-39 mnemonic for a funded testnet wallet)");

    let network = Network::Testnet;

    // Build SDK with testnet DAPI addresses and no-op context provider.
    let address_list: dash_sdk::dapi_client::AddressList = TESTNET_DAPI_ADDRESSES
        .iter()
        .map(|s| s.parse().expect("valid DAPI address"))
        .collect();
    let sdk = dash_sdk::SdkBuilder::new(address_list)
        .with_network(network)
        .with_context_provider(NoopContextProvider)
        .build()
        .expect("Failed to build SDK");
    let sdk = Arc::new(sdk);

    let persister = Arc::new(RecordingPersister::new());
    let persister_for_check = Arc::clone(&persister);
    let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    let manager = Arc::new(PlatformWalletManager::new(
        Arc::clone(&sdk),
        persister,
        event_handler,
    ));

    // --- Create wallet from mnemonic ---
    let mnemonic: key_wallet::Mnemonic = mnemonic_str.parse().expect("Failed to parse mnemonic");
    let seed_bytes = mnemonic.to_seed("");

    let platform_wallet = manager
        .create_wallet_from_seed_bytes(
            network,
            &seed_bytes,
            WalletAccountCreationOptions::Default,
            None,
        )
        .await
        .expect("Failed to create platform wallet");

    println!(
        "Wallet created: {}",
        hex::encode(platform_wallet.wallet_id())
    );

    // --- Build SPV config ---
    let spv_data_dir = std::env::temp_dir().join("platform-wallet-spv-test");
    std::fs::create_dir_all(&spv_data_dir).expect("Failed to create SPV data dir");

    let mut config = ClientConfig::new(network)
        .with_storage_path(spv_data_dir)
        .with_validation_mode(ValidationMode::Full)
        .with_start_height(0)
        .with_mempool_tracking(MempoolStrategy::BloomFilter);

    // Seed SPV with DAPI addresses as P2P peers (port 19999 for testnet).
    // These are known-good masternodes that support compact block filters.
    // Without this, DNS seed discovery may resolve to nodes that don't
    // support required capabilities, causing slow/stalled sync.
    for dapi_addr in TESTNET_DAPI_ADDRESSES {
        if let Some(host) = dapi_addr.strip_prefix("https://") {
            let ip_str = host.split(':').next().unwrap_or(host);
            if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
                config.add_peer(std::net::SocketAddr::new(ip, 19999));
            }
        }
    }

    // --- Start SPV in background ---
    let spv = manager.spv_arc();
    spv.start(config).await.unwrap();
    spv.spawn_run_loop();
    // --- Wait for confirmed balance ---
    // Cold start needs to sync full testnet chain headers (~1M+ blocks).
    // Second run with cached state is much faster (~20s).
    let timeout = Duration::from_secs(600);
    let start = std::time::Instant::now();
    let mut last_height = 0u32;

    println!(
        "Waiting for SPV sync and balance (timeout: {}s)...",
        timeout.as_secs()
    );

    loop {
        if start.elapsed() > timeout {
            let _ = manager.spv().stop().await;
            panic!("Timeout waiting for wallet balance after {:?}", timeout);
        }

        // Check balance via lock-free atomics (no lock needed).
        let confirmed = platform_wallet.balance().confirmed();
        let total = platform_wallet.balance().total();

        // Check synced height via state guard.
        let synced = {
            let state = platform_wallet.state().await;
            state.core_wallet.synced_height()
        };

        if synced != last_height {
            println!(
                "Synced height: {}, confirmed: {} duffs, total: {} duffs",
                synced, confirmed, total
            );
            last_height = synced;
        }

        if confirmed > 0 {
            println!("SUCCESS: Wallet has confirmed balance: {} duffs", confirmed);
            let _ = manager.spv().stop().await;

            // --- Verify persistence ---
            let core_stores = persister_for_check.core_store_count();
            println!("Persistence: {} core changeset store calls", core_stores);
            assert!(
                core_stores > 0,
                "CorePersistenceBridge should have routed core changesets to the persister"
            );

            let persisted_height = persister_for_check.max_synced_height();
            println!("Persistence: max synced_height = {:?}", persisted_height);
            assert!(
                persisted_height.is_some(),
                "At least one store call should carry a synced_height"
            );
            assert!(
                persisted_height.unwrap() > 0,
                "Persisted synced_height should be non-zero after sync"
            );

            return;
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
