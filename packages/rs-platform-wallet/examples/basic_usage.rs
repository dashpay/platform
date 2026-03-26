//! Example demonstrating basic usage of PlatformWalletInfo

use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::Network;
use platform_wallet::error::PlatformWalletError;
use platform_wallet::platform_wallet_info::PlatformWalletInfo;

fn main() -> Result<(), PlatformWalletError> {
    // Create a platform wallet
    let wallet_id = [1u8; 32];
    let network = Network::Testnet;
    let platform_wallet =
        PlatformWalletInfo::new(network, wallet_id, "My Platform Wallet".to_string());

    println!("Created wallet: {:?}", platform_wallet.name());

    // You can manage identities
    // In a real application, you would load identities from the platform
    println!(
        "Total identities on {:?}: {}",
        network,
        platform_wallet.identities().len()
    );

    // The platform wallet can be used with WalletManager (requires "manager" feature)
    #[cfg(feature = "manager")]
    {
        use key_wallet::manager::WalletManager;

        let _wallet_manager = WalletManager::<PlatformWalletInfo>::new(network);
        println!("Platform wallet successfully integrated with wallet managers!");
    }

    Ok(())
}
