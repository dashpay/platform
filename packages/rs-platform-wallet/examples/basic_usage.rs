//! Example demonstrating basic usage of PlatformWallet

use dash_sdk::Sdk;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::error::PlatformWalletError;
use platform_wallet::PlatformWallet;

fn main() -> Result<(), PlatformWalletError> {
    // Create a mock SDK (no network needed for this example)
    let sdk = Sdk::new_mock();

    // Create a platform wallet from a mnemonic
    let network = Network::Testnet;
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let options = WalletAccountCreationOptions::default();

    let wallet =
        PlatformWallet::from_mnemonic(sdk.clone(), network, mnemonic, "", options.clone())?;

    println!("Created wallet: {:?}", wallet);

    // Access sub-wallets
    println!("Wallet ID: {}", hex::encode(wallet.wallet_id()));

    // Core wallet manages UTXOs, balances, and addresses
    let _core = wallet.core();

    // Identity wallet manages Platform identities
    let _identity = wallet.identity();

    // DashPay wallet manages contact requests and social payments
    let _dashpay = wallet.dashpay();

    // Token wallet manages Platform token balances
    let _tokens = wallet.tokens();

    // You can also create a wallet with a random mnemonic
    let (random_wallet, generated_mnemonic) = PlatformWallet::random(sdk, network, options)?;

    println!("Random wallet: {:?}", random_wallet);
    println!("Save this mnemonic: {}", generated_mnemonic);

    Ok(())
}
