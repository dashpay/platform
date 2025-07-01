use anyhow::Result;
use dash_sdk::dapi_client::{Address, AddressList};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::platform::{Fetch, Identifier, Identity};
use dash_sdk::SdkBuilder;
use std::env;
use std::str::FromStr;

fn print_usage() {
    println!("Usage: dash-platform-balance-checker <identity-id> [OPTIONS]");
    println!();
    println!("Arguments:");
    println!("  <identity-id>              Identity ID in Base58 format");
    println!();
    println!("Options:");
    println!("  --testnet                  Use testnet instead of mainnet (default: mainnet)");
    println!("  --core-host <host>         Core RPC host (default: localhost)");
    println!("  --core-port <port>         Core RPC port (default: 9998 mainnet, 19998 testnet)");
    println!("  --core-user <username>     Core RPC username (default: dashrpc)");
    println!("  --core-password <password> Core RPC password (default: password)");
    println!("  --no-core                  Skip Core connection (may limit functionality)");
    println!();
    println!("Examples:");
    println!("  # Check balance on mainnet using local Core");
    println!("  dash-platform-balance-checker 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk");
    println!();
    println!("  # Check balance on testnet with custom Core");
    println!("  dash-platform-balance-checker 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk \\");
    println!("    --testnet --core-host 192.168.1.100 --core-port 19998 \\");
    println!("    --core-user myuser --core-password mypass");
    println!();
    println!("  # Check balance without Core connection");
    println!("  dash-platform-balance-checker 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk --no-core");
}

fn parse_args(args: &[String]) -> Result<(String, bool, String, u16, String, String, bool)> {
    if args.len() < 2 {
        return Err(anyhow::anyhow!("Missing identity ID"));
    }
    
    let identity_id = args[1].clone();
    let mut use_testnet = false;
    let mut core_host = "localhost".to_string();
    let mut core_port = 0u16; // Will be set based on network
    let mut core_user = "dashrpc".to_string();
    let mut core_password = "password".to_string();
    let mut no_core = false;
    
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--testnet" => use_testnet = true,
            "--no-core" => no_core = true,
            "--core-host" => {
                if i + 1 < args.len() {
                    core_host = args[i + 1].clone();
                    i += 1;
                } else {
                    return Err(anyhow::anyhow!("--core-host requires a value"));
                }
            }
            "--core-port" => {
                if i + 1 < args.len() {
                    core_port = args[i + 1].parse()
                        .map_err(|_| anyhow::anyhow!("Invalid port number"))?;
                    i += 1;
                } else {
                    return Err(anyhow::anyhow!("--core-port requires a value"));
                }
            }
            "--core-user" => {
                if i + 1 < args.len() {
                    core_user = args[i + 1].clone();
                    i += 1;
                } else {
                    return Err(anyhow::anyhow!("--core-user requires a value"));
                }
            }
            "--core-password" => {
                if i + 1 < args.len() {
                    core_password = args[i + 1].clone();
                    i += 1;
                } else {
                    return Err(anyhow::anyhow!("--core-password requires a value"));
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown option: {}", args[i])),
        }
        i += 1;
    }
    
    // Set default port based on network if not specified
    if core_port == 0 {
        core_port = if use_testnet { 19998 } else { 9998 };
    }
    
    Ok((identity_id, use_testnet, core_host, core_port, core_user, core_password, no_core))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    // Check for help flag
    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_usage();
        return Ok(());
    }
    
    // Parse arguments
    let (identity_id_string, use_testnet, core_host, core_port, core_user, core_password, no_core) = 
        match parse_args(&args) {
            Ok(parsed) => parsed,
            Err(e) => {
                eprintln!("Error: {}", e);
                eprintln!();
                print_usage();
                return Ok(());
            }
        };
    
    let network = if use_testnet { "Testnet" } else { "Mainnet" };
    
    println!("Dash Platform Balance Checker - {}", network);
    println!("=====================================\n");
    println!("Identity ID: {}", identity_id_string);
    
    // Parse the identity ID
    let identity_id = match Identifier::from_string(
        &identity_id_string,
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
    if !no_core {
        println!("\nCore connection: {}:{}", core_host, core_port);
        println!("Core user: {}", core_user);
    } else {
        println!("\nRunning without Core connection");
    }
    
    println!("\nConnecting to {}...", network.to_lowercase());
    
    // Create SDK instance
    let sdk = if use_testnet {
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
        if !no_core {
            builder = builder.with_core(&core_host, core_port, &core_user, &core_password);
        }
        builder.build()
            .expect("Failed to build SDK")
    } else {
        // Mainnet configuration
        let addresses = vec![
            Address::from_str("https://dapi.dash.org:443")?,
            Address::from_str("https://dapi-1.dash.org:443")?,
            Address::from_str("https://dapi-2.dash.org:443")?,
        ];
        
        let mut builder = SdkBuilder::new(AddressList::from_iter(addresses));
        if !no_core {
            builder = builder.with_core(&core_host, core_port, &core_user, &core_password);
        }
        builder.build()
            .expect("Failed to build SDK")
    };
    
    // Fetch the identity
    println!("Fetching identity...");
    match Identity::fetch(&sdk, identity_id).await {
        Ok(Some(identity)) => {
            println!("\n✓ Identity found!");
            println!("  Balance: {} credits", identity.balance());
            println!(
                "  Balance in DASH: {} DASH",
                identity.balance() as f64 / 100_000_000_000.0  // 1 DASH = 100,000,000,000 credits
            );
            println!("  Revision: {}", identity.revision());
            println!("  Public keys: {}", identity.public_keys().len());
        }
        Ok(None) => {
            println!("\n✗ Identity not found on {}", network.to_lowercase());
            println!(
                "  The identity {} does not exist on {}",
                identity_id_string, network.to_lowercase()
            );
        }
        Err(e) => {
            println!("\n✗ Error fetching identity: {}", e);
            if no_core {
                println!("  Note: Some operations may fail without Core connection");
            }
        }
    }
    
    Ok(())
}