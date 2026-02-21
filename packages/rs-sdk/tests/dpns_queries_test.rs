use dash_sdk::SdkBuilder;
use dpp::dashcore::Network;

// Test values from wasm-sdk docs.html (testnet DPNS integration test fixtures)
/// Base58-encoded testnet identity ID used for DPNS query testing (source: wasm-sdk docs.html)
const TEST_IDENTITY_ID: &str = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk";
/// Known testnet DPNS username for integration testing (source: wasm-sdk docs.html)
const TEST_USERNAME: &str = "alice";
/// Search prefix for DPNS name search testing (source: wasm-sdk docs.html)
const TEST_PREFIX: &str = "ali";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // Requires network connection
async fn test_dpns_queries_from_docs() {
    use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
    use std::num::NonZeroUsize;

    // Create trusted context provider for testnet
    let context_provider = TrustedHttpContextProvider::new(
        Network::Testnet,
        None,                            // No devnet name
        NonZeroUsize::new(100).unwrap(), // Cache size
    )
    .expect("Failed to create context provider");

    // Initialize SDK for testnet with trusted context provider
    // Dash Platform testnet node address (DAPI endpoint)
    let address_list = "https://52.12.176.90:1443"
        .parse()
        .expect("Failed to parse address");
    let sdk = SdkBuilder::new(address_list)
        .with_network(Network::Testnet)
        .with_context_provider(context_provider)
        .build()
        .expect("Failed to create SDK");

    // Test 1: Check availability of "alice"
    let _is_available = sdk
        .check_dpns_name_availability(TEST_USERNAME)
        .await
        .expect("check availability should succeed");

    // Test 2: Resolve "alice" to identity ID
    let _maybe_identity = sdk
        .resolve_dpns_name_to_identity(TEST_USERNAME)
        .await
        .expect("resolve should succeed");

    // Test 3: Get DPNS usernames for identity
    // Parse the identity ID from base58
    let identity_id = dash_sdk::dpp::prelude::Identifier::from_string(
        TEST_IDENTITY_ID,
        dpp::platform_value::string_encoding::Encoding::Base58,
    )
    .expect("identity id should parse");

    let _usernames = sdk
        .get_dpns_usernames_by_identity(identity_id, Some(10))
        .await
        .expect("get usernames by identity should succeed");

    // Test 4: Search DPNS names by prefix "ali"
    let _search_results = sdk
        .search_dpns_names(TEST_PREFIX, Some(10))
        .await
        .expect("search should succeed");

    // Test with a name that's more likely to exist on testnet
    let maybe_identity = sdk
        .resolve_dpns_name_to_identity("therealslimshaddy5")
        .await
        .expect("resolve should succeed");

    if let Some(identity_id) = maybe_identity {
        let _usernames = sdk
            .get_dpns_usernames_by_identity(identity_id, Some(5))
            .await
            .expect("get usernames by identity should succeed");
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
        None,                            // No devnet name
        NonZeroUsize::new(100).unwrap(), // Cache size
    )
    .expect("Failed to create context provider");

    // Dash Platform testnet node address (DAPI endpoint)
    let address_list = "https://52.12.176.90:1443"
        .parse()
        .expect("Failed to parse address");
    let sdk = SdkBuilder::new(address_list)
        .with_network(Network::Testnet)
        .with_context_provider(context_provider)
        .build()
        .expect("Failed to create SDK");

    let test_prefixes = vec!["a", "test", "d", "dash", "demo", "user"];

    for prefix in test_prefixes {
        let _results = sdk
            .search_dpns_names(prefix, Some(5))
            .await
            .expect("search should succeed");
    }
}
