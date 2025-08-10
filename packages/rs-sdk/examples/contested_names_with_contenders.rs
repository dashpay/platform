//! Example showing how to get contested DPNS usernames with their contenders
//!
//! This example demonstrates using the updated get_contested_non_resolved_usernames
//! method which returns a BTreeMap of names to their Contenders.

use dash_sdk::{Sdk, SdkBuilder};
use dpp::dashcore::Network;
use dpp::platform_value::string_encoding::Encoding;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use std::num::NonZeroUsize;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create SDK with testnet configuration
    let address_list = "https://52.12.176.90:1443"
        .parse()
        .expect("Failed to parse address");

    // Create trusted context provider for testnet
    let context_provider = TrustedHttpContextProvider::new(
        Network::Testnet,
        None,                            // No devnet name
        NonZeroUsize::new(100).unwrap(), // Cache size
    )?;

    let sdk = SdkBuilder::new(address_list)
        .with_network(Network::Testnet)
        .with_context_provider(context_provider)
        .build()?;

    println!("Fetching contested non-resolved DPNS usernames with contenders...\n");
    
    // Get contested non-resolved usernames with their contenders
    let non_resolved_names = sdk.get_contested_non_resolved_usernames(Some(10)).await?;
    
    if non_resolved_names.is_empty() {
        println!("No contested non-resolved DPNS usernames found on testnet.");
        return Ok(());
    }
    
    println!("Found {} contested non-resolved usernames:\n", non_resolved_names.len());
    
    // Display each contested name with its contenders
    for (name, contenders) in non_resolved_names {
        println!("📌 Contested name: '{}'", name);
        println!("   Contenders ({} total):", contenders.contenders.len());
        
        // Show up to 5 contenders
        for (contender_id, votes) in contenders.contenders.iter().take(5) {
            let id_str = contender_id.to_string(Encoding::Base58);
            println!("     • {} - {:?} votes", id_str, votes);
        }
        
        if contenders.contenders.len() > 5 {
            println!("     ... and {} more contenders", contenders.contenders.len() - 5);
        }
        
        // Show vote tallies if present
        if let Some(abstain) = contenders.abstain_vote_tally {
            println!("   Abstain votes: {}", abstain);
        }
        
        if let Some(lock) = contenders.lock_vote_tally {
            println!("   Lock votes: {}", lock);
        }
        
        // Confirm no winner (since these are unresolved)
        match contenders.winner {
            Some(_) => println!("   ⚠️ Unexpected: This name has a winner but was marked as unresolved"),
            None => println!("   ✅ Status: Unresolved (no winner yet)"),
        }
        
        println!();
    }
    
    Ok(())
}