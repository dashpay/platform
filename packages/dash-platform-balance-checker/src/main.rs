use anyhow::Result;
use clap::Parser;
use dash_sdk::dapi_client::{Address, AddressList};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::platform::{Fetch, Identifier, Identity};
use dash_sdk::SdkBuilder;
use std::env;
use std::str::FromStr;

/// Number of credits per DASH
/// 1 DASH = 100,000,000,000 credits (100 billion)
const CREDITS_PER_DASH: f64 = 100_000_000_000.0;

/// Dash Platform Balance Checker - Check identity balances on Dash Platform
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(after_help = "EXAMPLES:
    # Check balance on mainnet using local Core
    dash-platform-balance-checker 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk

    # Check balance on testnet with custom Core (password from env var)
    DASH_CORE_RPC_PASSWORD=mypass dash-platform-balance-checker 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk \\
        --testnet --core-host 192.168.1.100 --core-port 19998 --core-user myuser

    # Check balance without Core connection
    dash-platform-balance-checker 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk --no-core

SECURITY:
    Core RPC password can be provided in three ways (in order of preference):
    1. Set DASH_CORE_RPC_PASSWORD environment variable
    2. Enter interactively when prompted (most secure)
    3. Pass via --core-password flag (least secure, visible in process list)")]
struct Args {
    /// Identity ID in Base58 format
    identity_id: String,

    /// Use testnet instead of mainnet
    #[arg(long)]
    testnet: bool,

    /// Core RPC host
    #[arg(long, default_value = "localhost")]
    core_host: String,

    /// Core RPC port (defaults to 9998 for mainnet, 19998 for testnet)
    #[arg(long)]
    core_port: Option<u16>,

    /// Core RPC username
    #[arg(long, default_value = "dashrpc")]
    core_user: String,

    /// Core RPC password (reads from DASH_CORE_RPC_PASSWORD env var if not provided)
    #[arg(long, hide = true)]
    core_password: Option<String>,

    /// Skip Core connection (may limit functionality)
    #[arg(long)]
    no_core: bool,
}

impl Args {
    /// Get the effective core port, using defaults based on network if not specified
    fn get_core_port(&self) -> u16 {
        self.core_port
            .unwrap_or(if self.testnet { 19998 } else { 9998 })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    let network = if args.testnet { "Testnet" } else { "Mainnet" };
    let core_port = args.get_core_port();

    // Handle Core password securely
    let core_password = if args.no_core {
        String::new() // Won't be used anyway
    } else {
        match &args.core_password {
            Some(password) => {
                eprintln!("Warning: Passing passwords via command line is insecure.");
                eprintln!("Consider using DASH_CORE_RPC_PASSWORD environment variable instead.");
                password.clone()
            }
            None => {
                // Try environment variable first
                match env::var("DASH_CORE_RPC_PASSWORD") {
                    Ok(password) => password,
                    Err(_) => {
                        // Prompt for password
                        eprint!("Enter Core RPC password: ");
                        match rpassword::read_password() {
                            Ok(password) => password,
                            Err(e) => {
                                eprintln!("Error reading password: {}", e);
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    };

    println!("Dash Platform Balance Checker - {}", network);
    println!("=====================================\n");
    println!("Identity ID: {}", args.identity_id);

    // Parse the identity ID
    let identity_id = match Identifier::from_string(
        &args.identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    ) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error: Invalid identity ID format: {}", e);
            eprintln!("Identity ID must be in Base58 format");
            return Ok(());
        }
    };

    println!("Parsed Identity ID: {}", identity_id);

    // Core connection info
    if !args.no_core {
        println!("\nCore connection: {}:{}", args.core_host, core_port);
        println!("Core user: {}", args.core_user);
    } else {
        println!("\nRunning without Core connection");
    }

    println!("\nConnecting to {}...", network.to_lowercase());

    // Create SDK instance
    let sdk = if args.testnet {
        // Testnet configuration
        let addresses = vec![
            Address::from_str("https://52.13.132.146:1443")?,
            Address::from_str("https://52.89.154.48:1443")?,
            Address::from_str("https://44.227.137.77:1443")?,
            Address::from_str("https://52.40.219.41:1443")?,
            Address::from_str("https://54.149.33.167:1443")?,
            Address::from_str("https://54.187.14.232:1443")?,
            Address::from_str("https://52.12.176.90:1443")?,
            Address::from_str("https://52.34.144.50:1443")?,
            Address::from_str("https://44.239.39.153:1443")?,
        ];

        let mut builder = SdkBuilder::new(AddressList::from_iter(addresses));
        if !args.no_core {
            builder =
                builder.with_core(&args.core_host, core_port, &args.core_user, &core_password);
        }
        builder.build().expect("Failed to build SDK")
    } else {
        // Mainnet configuration
        let addresses = vec![
            Address::from_str("https://dapi.dash.org:443")?,
            Address::from_str("https://dapi-1.dash.org:443")?,
            Address::from_str("https://dapi-2.dash.org:443")?,
        ];

        let mut builder = SdkBuilder::new(AddressList::from_iter(addresses));
        if !args.no_core {
            builder =
                builder.with_core(&args.core_host, core_port, &args.core_user, &core_password);
        }
        builder.build().expect("Failed to build SDK")
    };

    // Fetch the identity
    println!("Fetching identity...");
    match Identity::fetch(&sdk, identity_id).await {
        Ok(Some(identity)) => {
            println!("\n✓ Identity found!");
            println!("  Balance: {} credits", identity.balance());
            println!(
                "  Balance in DASH: {} DASH",
                identity.balance() as f64 / CREDITS_PER_DASH
            );
            println!("  Revision: {}", identity.revision());
            println!("  Public keys: {}", identity.public_keys().len());
        }
        Ok(None) => {
            println!("\n✗ Identity not found on {}", network.to_lowercase());
            println!(
                "  The identity {} does not exist on {}",
                args.identity_id,
                network.to_lowercase()
            );
        }
        Err(e) => {
            println!("\n✗ Error fetching identity: {}", e);
            if args.no_core {
                println!("  Note: Some operations may fail without Core connection");
            }
        }
    }

    Ok(())
}
