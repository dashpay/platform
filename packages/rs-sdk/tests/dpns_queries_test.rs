use dash_sdk::SdkBuilder;
use dpp::dashcore::Network;

// Test values from wasm-sdk docs.html
const TEST_IDENTITY_ID: &str = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk";
const TEST_USERNAME: &str = "alice";
const TEST_PREFIX: &str = "ali";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // Requires network connection
async fn test_dpns_queries_from_docs() {
    use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
    use std::num::NonZeroUsize;
    
    // Create trusted context provider for testnet
    let context_provider = TrustedHttpContextProvider::new(
        Network::Testnet,
        None, // No devnet name
        NonZeroUsize::new(100).unwrap(), // Cache size
    )
    .expect("Failed to create context provider");
    
    // Initialize SDK for testnet with trusted context provider
    let address_list = "https://52.12.176.90:1443".parse().expect("Failed to parse address");
    let sdk = SdkBuilder::new(address_list)
        .with_network(Network::Testnet)
        .with_context_provider(context_provider)
        .build()
        .expect("Failed to create SDK");

    println!("Testing DPNS queries with values from wasm-sdk docs.html...\n");

    // Test 1: Check availability of "alice"
    println!("1. Testing dpns_check_availability('alice'):");
    match sdk.check_dpns_name_availability(TEST_USERNAME).await {
        Ok(is_available) => {
            println!("   ✅ Success: Name 'alice' is {}", 
                if is_available { "AVAILABLE" } else { "NOT AVAILABLE" });
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }
    println!();

    // Test 2: Resolve "alice" to identity ID
    println!("2. Testing dpns_resolve_name('alice'):");
    match sdk.resolve_dpns_name_to_identity(TEST_USERNAME).await {
        Ok(Some(identity_id)) => {
            println!("   ✅ Success: 'alice' resolves to identity: {}", identity_id);
        }
        Ok(None) => {
            println!("   ℹ️  Name 'alice' not found (not registered)");
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }
    println!();

    // Test 3: Get DPNS usernames for identity
    println!("3. Testing get_dpns_usernames_by_identity('{}'):", TEST_IDENTITY_ID);
    
    // Parse the identity ID from base58
    let identity_id = match dash_sdk::dpp::prelude::Identifier::from_string(
        TEST_IDENTITY_ID, 
        dpp::platform_value::string_encoding::Encoding::Base58
    ) {
        Ok(id) => id,
        Err(e) => {
            println!("   ❌ Error parsing identity ID: {}", e);
            return;
        }
    };

    match sdk.get_dpns_usernames_by_identity(identity_id, Some(10)).await {
        Ok(usernames) => {
            if usernames.is_empty() {
                println!("   ℹ️  No usernames found for this identity");
            } else {
                println!("   ✅ Success: Found {} usernames:", usernames.len());
                for (i, username) in usernames.iter().enumerate() {
                    println!("      [{}] {}", i + 1, username.full_name);
                    println!("          - Label: {}", username.label);
                    println!("          - Normalized: {}", username.normalized_label);
                    println!("          - Owner ID: {}", username.owner_id);
                    if let Some(records_id) = &username.records_identity_id {
                        println!("          - Records Identity: {}", records_id);
                    }
                }
            }
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }
    println!();

    // Test 4: Search DPNS names by prefix "ali"
    println!("4. Testing search_dpns_names('{}'):", TEST_PREFIX);
    match sdk.search_dpns_names(TEST_PREFIX, Some(10)).await {
        Ok(usernames) => {
            if usernames.is_empty() {
                println!("   ℹ️  No names found starting with '{}'", TEST_PREFIX);
            } else {
                println!("   ✅ Success: Found {} names starting with '{}':", usernames.len(), TEST_PREFIX);
                for (i, username) in usernames.iter().enumerate() {
                    println!("      [{}] {}", i + 1, username.full_name);
                    println!("          - Label: {}", username.label);
                    println!("          - Normalized: {}", username.normalized_label);
                    println!("          - Owner ID: {}", username.owner_id);
                }
            }
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }
    println!();

    // Test with a name that's more likely to exist on testnet
    println!("5. Testing with 'therealslimshaddy5' (known existing name):");
    match sdk.resolve_dpns_name_to_identity("therealslimshaddy5").await {
        Ok(Some(identity_id)) => {
            println!("   ✅ Success: 'therealslimshaddy5' resolves to identity: {}", identity_id);
            
            // Get usernames for this identity
            match sdk.get_dpns_usernames_by_identity(identity_id, Some(5)).await {
                Ok(usernames) => {
                    println!("   ✅ This identity owns {} usernames", usernames.len());
                }
                Err(e) => {
                    println!("   ❌ Error getting usernames: {}", e);
                }
            }
        }
        Ok(None) => {
            println!("   ℹ️  Name 'therealslimshaddy5' not found");
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // Requires network connection
async fn test_dpns_search_variations() {
    use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
    use std::num::NonZeroUsize;
    
    // Create trusted context provider for testnet
    let context_provider = TrustedHttpContextProvider::new(
        Network::Testnet,
        None, // No devnet name
        NonZeroUsize::new(100).unwrap(), // Cache size
    )
    .expect("Failed to create context provider");
    
    let address_list = "https://52.12.176.90:1443".parse().expect("Failed to parse address");
    let sdk = SdkBuilder::new(address_list)
        .with_network(Network::Testnet)
        .with_context_provider(context_provider)
        .build()
        .expect("Failed to create SDK");

    println!("Testing DPNS search with various prefixes...\n");

    let test_prefixes = vec!["a", "test", "d", "dash", "demo", "user"];

    for prefix in test_prefixes {
        println!("Searching for names starting with '{}':", prefix);
        match sdk.search_dpns_names(prefix, Some(5)).await {
            Ok(usernames) => {
                if usernames.is_empty() {
                    println!("  - No names found");
                } else {
                    println!("  - Found {} names:", usernames.len());
                    for username in usernames.iter().take(3) {
                        println!("    • {}", username.full_name);
                    }
                    if usernames.len() > 3 {
                        println!("    ... and {} more", usernames.len() - 3);
                    }
                }
            }
            Err(e) => {
                println!("  - Error: {}", e);
            }
        }
        println!();
    }
}