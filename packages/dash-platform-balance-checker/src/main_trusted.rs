use anyhow::Result;
use dash_sdk::dapi_client::{Address, AddressList};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::platform::{Fetch, Identifier, Identity};
use dash_sdk::{Sdk, SdkBuilder};
use dpp::dashcore::Network;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use std::num::NonZeroUsize;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Dash Balance Checker - Testnet (Trusted Context Provider)");
    println!("=========================================================\n");

    let identity_id_string = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk";
    println!("Identity ID: {}", identity_id_string);

    // Parse the identity ID
    let identity_id = Identifier::from_string(
        identity_id_string,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    println!("Parsed Identity ID: {}", identity_id);

    // Create SDK instance for testnet
    println!("\nConnecting to testnet...");

    // Testnet addresses
    let addresses = vec![
        Address::from_str("https://52.13.132.146:1443")?,
        Address::from_str("https://52.89.154.48:1443")?,
        Address::from_str("https://44.227.137.77:1443")?,
        Address::from_str("https://52.40.219.41:1443")?,
        Address::from_str("https://54.149.33.167:1443")?,
    ];

    // Build SDK with trusted context provider
    println!("Building SDK with trusted context provider...");

    // Create the trusted context provider
    let context_provider = TrustedHttpContextProvider::new(
        Network::Testnet,
        None, // devnet_name - only needed for devnet
        NonZeroUsize::new(100).expect("cache size"),
    )?;

    // Build SDK with mock core (context provider will handle everything)
    let sdk = SdkBuilder::new(AddressList::from_iter(addresses))
        .with_core("127.0.0.1", 1, "mock", "mock") // Mock values, won't be used
        .build()?;

    // Set our trusted context provider
    sdk.set_context_provider(context_provider);

    // Fetch the identity
    println!("Fetching identity with proof verification...");
    match Identity::fetch(&sdk, identity_id).await {
        Ok(Some(identity)) => {
            println!("\n✓ Identity found!");
            println!("  Balance: {} credits", identity.balance());
            println!(
                "  Balance in DASH: {} DASH",
                identity.balance() as f64 / 100_000_000.0
            );
            println!("  Revision: {}", identity.revision());
            println!("  Public keys: {}", identity.public_keys().len());

            // Show public key details
            for (key_id, public_key) in identity.public_keys() {
                println!(
                    "    Key {}: type={}, purpose={}",
                    key_id,
                    public_key.key_type(),
                    public_key.purpose()
                );
            }
        }
        Ok(None) => {
            println!("\n✗ Identity not found on testnet");
        }
        Err(e) => {
            println!("\n✗ Error fetching identity: {}", e);
            println!("  Details: {:?}", e);
        }
    }

    Ok(())
}
