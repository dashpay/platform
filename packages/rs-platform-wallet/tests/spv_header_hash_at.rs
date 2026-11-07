//! `SpvRuntime::header_hash_at` answers what the SPV header store holds at a
//! height, so a caller outside the crate can check that a block it recorded
//! is still on the active chain.

use std::sync::Arc;

use key_wallet::Network;
use key_wallet_manager::WalletManager;
use platform_wallet::spv::{HeaderLookup, SpvRuntime};
use platform_wallet::{PlatformEventManager, PlatformWalletInfo};
use tokio::sync::RwLock;

#[tokio::test]
async fn a_runtime_without_a_client_cannot_read_a_header() {
    let wallets = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
        Network::Testnet,
    )));
    let runtime = SpvRuntime::new(wallets, Arc::new(PlatformEventManager::new(vec![])));
    let lookup = runtime.header_hash_at(0).await;
    assert!(
        matches!(lookup, HeaderLookup::Unreadable(_)),
        "got {lookup:?}"
    );
}
