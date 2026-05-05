//! Example showing how to get contested DPNS usernames for a specific identity
//!
//! This example demonstrates using the get_non_resolved_dpns_contests_for_identity
//! method to find all unresolved contests where a specific identity is competing.

use dash_sdk::SdkBuilder;
use dpp::dashcore::Network;
use dpp::identifier::Identifier;
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

    // Example identity ID - replace with an actual identity ID to test
    // This is just an example ID, you should use a real one from your identity
    let identity_id_str = "HccabTZZpMEDAqU4oQFk3PE47kS6jDDmCjoxR88gFttA";

    let identity_id = Identifier::from_string(identity_id_str, Encoding::Base58)?;

    println!(
        "Fetching contested DPNS usernames for identity: {}\n",
        identity_id_str
    );

    // Get non-resolved contests for this identity
    let identity_contests = sdk
        .get_non_resolved_dpns_contests_for_identity(
            identity_id,
            Some(20), // limit to 20 results
        )
        .await?;

    if identity_contests.is_empty() {
        println!("This identity is not currently contending for any unresolved DPNS usernames.");
        println!("\nTip: To find identities with contests, first run:");
        println!("  1. get_contested_non_resolved_usernames() to find unresolved contests");
        println!("  2. Pick a contender from one of those contests");
        println!("  3. Use that identity ID with this method");
        return Ok(());
    }

    println!(
        "Identity {} is contending in {} unresolved contests:\n",
        identity_id_str,
        identity_contests.len()
    );

    // Display each contest where this identity is competing
    for (name, contest_info) in &identity_contests {
        println!("🏆 Contest for username: '{}'", name);
        println!(
            "   Total contenders: {}",
            contest_info.contenders.contenders.len()
        );
        println!("   Voting ends: {} ms", contest_info.end_time);

        // Show all contenders and highlight our identity
        println!("   Contenders:");
        for (contender_id, votes) in &contest_info.contenders.contenders {
            let id_str = contender_id.to_string(Encoding::Base58);
            if contender_id == &identity_id {
                println!("     • {} (YOU) - {:?} votes ⭐", id_str, votes);
            } else {
                println!("     • {} - {:?} votes", id_str, votes);
            }
        }

        // Show vote tallies
        if let Some(abstain) = contest_info.contenders.abstain_vote_tally {
            println!("   Abstain votes: {}", abstain);
        }

        if let Some(lock) = contest_info.contenders.lock_vote_tally {
            println!("   Lock votes: {}", lock);
        }

        // Confirm status
        if contest_info.contenders.winner.is_some() {
            println!("   ⚠️ Status: Has a winner (unexpected for unresolved contest)");
        } else {
            println!("   ✅ Status: Still unresolved (voting ongoing)");
        }

        println!();
    }

    // Summary
    println!("═══════════════════════════════════════════════════════");
    println!("Summary:");
    println!(
        "  • Identity is competing for {} username(s)",
        identity_contests.len()
    );
    println!("  • All contests are still unresolved (no winners yet)");
    println!("  • Voting is ongoing for these names");

    Ok(())
}
