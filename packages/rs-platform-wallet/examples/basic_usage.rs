//! Example demonstrating basic usage of PlatformWalletManager
//!
//! Creates a wallet via PlatformWalletManager and shows how to access
//! balances, addresses, identities, and asset locks.

use std::sync::Arc;

use dash_sdk::Sdk;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::Network;
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet::events::PlatformEventHandler;
use platform_wallet::PlatformWalletManager;

/// Minimal no-op persister for the example.
struct NoopPersister;
impl PlatformWalletPersistence for NoopPersister {
    fn store(
        &self,
        _wallet_id: platform_wallet::wallet::platform_wallet::WalletId,
        _changeset: platform_wallet::changeset::PlatformWalletChangeSet,
    ) -> Result<(), platform_wallet::changeset::PersistenceError> {
        Ok(())
    }
    fn flush(
        &self,
        _wallet_id: platform_wallet::wallet::platform_wallet::WalletId,
    ) -> Result<(), platform_wallet::changeset::PersistenceError> {
        Ok(())
    }
    fn load(
        &self,
    ) -> Result<
        platform_wallet::changeset::ClientStartState,
        platform_wallet::changeset::PersistenceError,
    > {
        Ok(platform_wallet::changeset::ClientStartState::default())
    }
}

/// Minimal no-op event handler for the example.
struct NoopEventHandler;
impl platform_wallet::events::EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = Arc::new(Sdk::new_mock());
    let persister = Arc::new(NoopPersister);
    let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);

    // Create a manager
    let manager = PlatformWalletManager::new(sdk.clone(), persister, vec![event_handler]);

    // Create a wallet from seed bytes
    let seed_bytes = [0u8; 64]; // dummy seed for example
    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &seed_bytes,
            WalletAccountCreationOptions::Default,
            None,
        )
        .await?;

    println!("Wallet ID: {}", hex::encode(wallet.wallet_id()));

    // --- Core wallet: balances and addresses ---
    let core = wallet.core();

    // Lock-free balance (AtomicU64, no lock needed)
    let balance_arc = wallet.balance();
    println!(
        "Balance: spendable={}, unconfirmed={}, total={}",
        balance_arc.confirmed(),
        balance_arc.unconfirmed(),
        balance_arc.total()
    );

    // Derive a receive address (blocking, acquires lock internally)
    let address = core.next_receive_address_for_account(0).await?;
    println!("Receive address: {}", address);

    // Read wallet info via state guard (derefs to PlatformWalletInfo)
    {
        let state = wallet.state().await;
        let synced_height = state.core_wallet.synced_height();
        let utxos = state
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .map(|a| a.spendable_utxos(synced_height))
            .unwrap_or_default();
        let tx_count = state.core_wallet.transaction_history().len();
        let birth = state.core_wallet.birth_height();
        let id_count = state.identity_manager.identity_count();
        println!(
            "UTXOs: {}, transactions: {}, birth_height: {}",
            utxos.len(),
            tx_count,
            birth
        );
        println!("Managed identities: {}", id_count);
    }

    // --- Asset locks ---
    let asset_locks = wallet.asset_locks();
    let tracked = asset_locks.list_tracked_locks_blocking();
    println!("Tracked asset locks: {}", tracked.len());

    Ok(())
}
