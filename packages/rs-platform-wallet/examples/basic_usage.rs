//! Example demonstrating basic usage of PlatformWalletInfo

use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use platform_wallet::{PlatformWalletError, PlatformWalletInfo};

fn main() -> Result<(), PlatformWalletError> {
    // Create a platform wallet
    let wallet_id = [1u8; 32];
    let mut platform_wallet = PlatformWalletInfo::new(wallet_id, "My Platform Wallet".to_string());

    println!("Created wallet: {:?}", platform_wallet.name());

    // You can manage identities
    // In a real application, you would load identities from the platform
    println!("Total identities: {}", platform_wallet.identities().len());
    println!(
        "Total credit balance: {}",
        platform_wallet.identity_manager.total_credit_balance()
    );

    // The platform wallet can be used with WalletManager (requires "manager" feature)
    #[cfg(feature = "manager")]
    {
        use key_wallet_manager::wallet_manager::WalletManager;

        let _wallet_manager = WalletManager::<PlatformWalletInfo>::new();
        println!("Platform wallet successfully integrated with wallet managers!");
    }

    Ok(())
}
