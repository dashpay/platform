//! Minimal SPV sync integration test.
//!
//! Creates a PlatformWalletManager, registers a wallet from a mnemonic,
//! starts SPV, waits for sync, and verifies the wallet has a non-zero balance.
//!
//! # Requirements
//! - `SPV_TEST_MNEMONIC` env var: 12-word BIP-39 mnemonic for a funded testnet wallet
//! - Network access to Dash testnet peers (DNS seed discovery)
//!
//! # Running
//! ```bash
//! SPV_TEST_MNEMONIC="word1 word2 ... word12" \
//!   cargo test -p platform-wallet --test spv_sync -- --ignored --nocapture
//! ```

use std::sync::Arc;
use std::time::Duration;

use dash_spv::client::config::MempoolStrategy;
use dash_spv::types::ValidationMode;
use dash_spv::ClientConfig;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::Network;
use platform_wallet::changeset::{PlatformWalletChangeSet, PlatformWalletPersistence};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;
use tokio_util::sync::CancellationToken;

/// No-op persister for tests.
struct NoopPersister;
impl PlatformWalletPersistence for NoopPersister {
    fn store(&self, _wallet_id: WalletId, _changeset: PlatformWalletChangeSet) {}
    fn flush(
        &self,
        _wallet_id: WalletId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    fn load(
        &self,
        _wallet_id: WalletId,
    ) -> Result<PlatformWalletChangeSet, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Default::default())
    }
}

#[ignore]
#[tokio::test]
async fn test_spv_sync_and_balance() {
    // --- Setup ---
    let mnemonic_str = std::env::var("SPV_TEST_MNEMONIC").expect(
        "SPV_TEST_MNEMONIC env var required (12-word BIP-39 mnemonic for a funded testnet wallet)",
    );

    let network = Network::Testnet;
    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    let persister: Arc<dyn PlatformWalletPersistence> = Arc::new(NoopPersister);

    // Wrap in Arc so we can clone into the SPV background task.
    let manager = Arc::new(PlatformWalletManager::new(Arc::clone(&sdk), persister));

    // --- Create wallet from mnemonic ---
    let mnemonic: key_wallet::Mnemonic = mnemonic_str.parse().expect("Failed to parse mnemonic");
    let seed_bytes = mnemonic.to_seed("");

    let platform_wallet = manager
        .create_wallet_from_seed_bytes(
            network,
            seed_bytes,
            WalletAccountCreationOptions::Default,
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

    let config = ClientConfig::new(network)
        .with_storage_path(spv_data_dir)
        .with_validation_mode(ValidationMode::Full)
        .with_start_height(0)
        .with_mempool_tracking(MempoolStrategy::BloomFilter);

    // --- Start SPV in background ---
    let cancel = CancellationToken::new();
    let manager_for_spv = Arc::clone(&manager);
    let cancel_for_spv = cancel.clone();
    let spv_handle = tokio::spawn(async move {
        if let Err(e) = manager_for_spv.spv().run(config, cancel_for_spv).await {
            eprintln!("SPV runtime error: {}", e);
        }
    });

    // --- Wait for spendable balance ---
    let timeout = Duration::from_secs(120);
    let start = std::time::Instant::now();
    let mut last_height = 0u32;

    println!(
        "Waiting for SPV sync and balance (timeout: {}s)...",
        timeout.as_secs()
    );

    loop {
        if start.elapsed() > timeout {
            cancel.cancel();
            let _ = spv_handle.await;
            panic!("Timeout waiting for wallet balance after {:?}", timeout);
        }

        // Check balance via lock-free atomics (no lock needed)
        let spendable = platform_wallet.balance().spendable();
        let total = platform_wallet.balance().total();

        // Check synced height via state guard
        let synced = {
            let state = platform_wallet.state().await;
            state.core_wallet.synced_height()
        };

        if synced != last_height {
            println!(
                "Synced height: {}, spendable: {} duffs, total: {} duffs",
                synced, spendable, total
            );
            last_height = synced;
        }

        if spendable > 0 {
            println!("SUCCESS: Wallet has spendable balance: {} duffs", spendable);
            cancel.cancel();
            let _ = spv_handle.await;
            return;
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
