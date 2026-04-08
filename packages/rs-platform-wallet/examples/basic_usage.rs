//! Example demonstrating basic usage of PlatformWallet
//!
//! Creates a wallet from a mnemonic and shows how to access
//! balances, addresses, identities, and asset locks.

use std::sync::Arc;

use dash_sdk::Sdk;
use key_wallet::Network;
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet::error::PlatformWalletError;
use platform_wallet::PlatformWallet;

/// Minimal no-op persister for the example.
struct NoopPersister;
impl PlatformWalletPersistence for NoopPersister {
    fn store(
        &self,
        _wallet_id: platform_wallet::wallet::platform_wallet::WalletId,
        _changeset: platform_wallet::changeset::PlatformWalletChangeSet,
    ) {
    }
    fn flush(
        &self,
        _wallet_id: platform_wallet::wallet::platform_wallet::WalletId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    fn load(
        &self,
        _wallet_id: platform_wallet::wallet::platform_wallet::WalletId,
    ) -> Result<platform_wallet::changeset::PlatformWalletChangeSet, Box<dyn std::error::Error + Send + Sync>>
    {
        Ok(Default::default())
    }
}

fn main() -> Result<(), PlatformWalletError> {
    let sdk = Arc::new(Sdk::new_mock());
    let persister: Arc<dyn PlatformWalletPersistence> = Arc::new(NoopPersister);
    let network = Network::Testnet;
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    // Create a wallet from a BIP-39 mnemonic
    let wallet = PlatformWallet::from_mnemonic(
        sdk.clone(),
        network,
        mnemonic,
        "",
        Default::default(),
        persister.clone(),
    )?;

    println!("Wallet ID: {}", hex::encode(wallet.wallet_id()));

    // --- Core wallet: balances and addresses ---
    let core = wallet.core();

    // Lock-free balance (AtomicU64, no lock needed)
    let balance = core.balance();
    println!(
        "Balance: spendable={}, unconfirmed={}, total={}",
        balance.spendable(),
        balance.unconfirmed(),
        balance.total()
    );

    // Derive a receive address (blocking, acquires lock internally)
    let address = core.next_receive_address_blocking()?;
    println!("Receive address: {}", address);

    // Read wallet info (UTXOs, transaction history, accounts, identities)
    // All mutable state is behind a single lock — one acquisition gives
    // access to everything.
    {
        let info = wallet.state_blocking();
        let utxos = info.managed_state.wallet_info().get_spendable_utxos();
        let tx_count = info.managed_state.wallet_info().transaction_history().len();
        let birth = info.managed_state.wallet_info().birth_height();
        let id_count = info.identity_manager.identities().len();
        println!("UTXOs: {}, transactions: {}, birth_height: {}", utxos.len(), tx_count, birth);
        println!("Managed identities: {}", id_count);
    }

    // --- Asset locks ---
    let asset_locks = wallet.asset_locks();
    let tracked = asset_locks.list_tracked_locks_blocking();
    println!("Tracked asset locks: {}", tracked.len());

    // --- Generate a random wallet ---
    let (random_wallet, generated_mnemonic) =
        PlatformWallet::random(sdk, network, Default::default(), persister)?;
    println!("Random wallet: {}", hex::encode(random_wallet.wallet_id()));
    println!("Save this mnemonic: {}", generated_mnemonic);

    Ok(())
}
