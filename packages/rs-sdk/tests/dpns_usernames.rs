//! Network-integration tests for DPNS username helpers on `dash_sdk::Sdk`.
//!
//! These tests are `#[ignore]` by default — they require a live Platform endpoint
//! (testnet, devnet, or SSH tunnel) plus a reachable Dash Core RPC, all configured
//! through `packages/rs-sdk/tests/.env` (`DASH_SDK_PLATFORM_HOST/PORT/SSL`,
//! `DASH_SDK_CORE_HOST/PORT/USER/PASSWORD`). They use the shared [`Config`] harness
//! exactly like the rest of `tests/fetch/*`, so quorum info comes from the local
//! Core RPC instead of a hardcoded HTTP context endpoint.
//!
//! Run with:
//! ```sh
//! cargo test -p dash-sdk --features generate-test-vectors -- --include-ignored
//! ```

#[allow(dead_code, unused_imports)]
mod fetch {
    #[path = "../fetch/config.rs"]
    pub mod config;
    #[path = "../fetch/generated_data.rs"]
    pub mod generated_data;
}

use dash_sdk::Sdk;
use dpp::prelude::Identifier;
use fetch::config::Config;

/// Build an SDK wired through the standard `tests/.env` harness, with a fresh
/// per-test namespace so dump dirs don't collide when regenerating vectors.
async fn build_testnet_sdk(namespace: &str) -> Sdk {
    Config::new().setup_api(namespace).await
}

// ---------------------------------------------------------------------------
// Relocated from src/platform/dpns_usernames/queries.rs
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // Requires network connection
async fn test_dpns_queries() {
    let sdk = build_testnet_sdk("dpns_queries").await;

    let results = sdk.search_dpns_names("test", Some(5)).await.unwrap();
    println!("Search results for 'test': {:?}", results);

    let is_available = sdk
        .check_dpns_name_availability("somerandomunusedname123456")
        .await
        .unwrap();
    assert!(is_available, "Random name should be available");

    if let Ok(Some(identity_id)) = sdk
        .resolve_dpns_name_to_identity("therealslimshaddy5")
        .await
    {
        println!("'therealslimshaddy5' resolves to identity: {}", identity_id);

        let usernames = sdk
            .get_dpns_usernames_by_identity(identity_id, Some(5))
            .await
            .unwrap();
        println!("Usernames for identity {}: {:?}", identity_id, usernames);
    }
}

// ---------------------------------------------------------------------------
// Relocated from src/platform/dpns_usernames/contested_queries.rs
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // Requires network connection
async fn test_contested_queries() {
    let sdk = build_testnet_sdk("contested_queries").await;

    println!("Testing get_contested_dpns_usernames...");
    let all_contested = sdk
        .get_contested_dpns_normalized_usernames(Some(5), None)
        .await;
    match &all_contested {
        Ok(names) => {
            println!("Successfully queried contested DPNS usernames");
            println!("Found {} contested DPNS usernames", names.len());
            for name in names {
                println!("  - {}", name);
            }
        }
        Err(e) => {
            println!("Could not fetch contested names (may not exist): {}", e);
            println!("This is expected if there are no contested names on testnet.");
        }
    }

    if let Ok(contested_names) = all_contested {
        if let Some(first_contested) = contested_names.first() {
            println!(
                "\nTesting get_contested_dpns_vote_state for '{}'...",
                first_contested
            );

            let vote_state = sdk
                .get_contested_dpns_vote_state(first_contested, Some(10))
                .await;
            match vote_state {
                Ok(state) => {
                    println!("Vote state for '{}':", first_contested);
                    if let Some((winner_info, _block_info)) = state.winner {
                        use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
                        match winner_info {
                            ContestedDocumentVotePollWinnerInfo::WonByIdentity(id) => {
                                println!(
                                    "  Winner: {}",
                                    id.to_string(
                                        dpp::platform_value::string_encoding::Encoding::Base58
                                    )
                                );
                            }
                            ContestedDocumentVotePollWinnerInfo::Locked => {
                                println!("  Winner: LOCKED");
                            }
                            ContestedDocumentVotePollWinnerInfo::NoWinner => {
                                println!("  Winner: None");
                            }
                        }
                    }
                    println!("  Contenders: {} total", state.contenders.len());
                    for (contender_id, votes) in state.contenders.iter().take(3) {
                        println!(
                            "    - {}: {:?} votes",
                            contender_id
                                .to_string(dpp::platform_value::string_encoding::Encoding::Base58),
                            votes
                        );
                    }
                    if let Some(abstain) = state.abstain_vote_tally {
                        println!("  Abstain votes: {}", abstain);
                    }
                    if let Some(lock) = state.lock_vote_tally {
                        println!("  Lock votes: {}", lock);
                    }
                }
                Err(e) => {
                    println!("Failed to get vote state: {}", e);
                }
            }

            if let Ok(vote_state) = sdk
                .get_contested_dpns_vote_state(first_contested, None)
                .await
            {
                if let Some((test_identity, _)) = vote_state.contenders.iter().next() {
                    println!(
                        "\nTesting get_contested_dpns_usernames_by_identity for {}...",
                        test_identity
                    );

                    let identity_names = sdk
                        .get_contested_dpns_usernames_by_identity(*test_identity, Some(5))
                        .await;

                    match identity_names {
                        Ok(names) => {
                            println!(
                                "Identity {} is contending for {} names:",
                                test_identity,
                                names.len()
                            );
                            for name in names {
                                println!("  - {}", name.label);
                            }
                        }
                        Err(e) => {
                            println!("Failed to get names for identity: {}", e);
                        }
                    }
                }
            }
        }
    }

    println!("\nTesting get_contested_dpns_identity_votes...");
    let test_masternode_id = Identifier::from_string(
        "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",
        dpp::platform_value::string_encoding::Encoding::Base58,
    );

    if let Ok(masternode_id) = test_masternode_id {
        let votes = sdk
            .get_contested_dpns_identity_votes(masternode_id, Some(5), None)
            .await;

        match votes {
            Ok(vote_list) => {
                println!(
                    "Masternode {} has voted on {} contested names",
                    masternode_id,
                    vote_list.len()
                );
                for name in vote_list {
                    println!("  - {}", name.label);
                }
            }
            Err(e) => {
                println!("Expected error - identity may not be a masternode: {}", e);
            }
        }
    }

    println!("\nTesting get_current_dpns_contests...");
    let current_contests = sdk.get_current_dpns_contests(None, None, Some(10)).await;
    match current_contests {
        Ok(contests) => {
            println!("Successfully queried current DPNS contests");
            println!("Found {} contested names", contests.len());
            for (name, end_time) in contests.iter().take(5) {
                println!("  '{}' ends at {}", name, end_time);
            }
        }
        Err(e) => {
            println!("Could not fetch current contests: {}", e);
            println!("This is expected if there are no active contests on testnet.");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // Requires network connection
async fn test_get_current_dpns_contests() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let sdk = build_testnet_sdk("get_current_dpns_contests").await;

    println!("Testing get_current_dpns_contests...");

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64;

    println!("\n1. Fetching all current DPNS contests...");
    let all_contests = sdk.get_current_dpns_contests(None, None, Some(100)).await;

    match &all_contests {
        Ok(contests) => {
            println!("Successfully fetched {} contested names", contests.len());

            for (name, end_time) in contests.iter().take(5) {
                println!("  '{}' ends at {}", name, end_time);
            }

            let names: Vec<_> = contests.keys().cloned().collect();
            let mut sorted_names = names.clone();
            sorted_names.sort();
            assert_eq!(names, sorted_names, "BTreeMap should be sorted by key");
            println!("Names are properly sorted alphabetically");
        }
        Err(e) => {
            println!("Could not fetch contests: {}", e);
            println!("This may be expected if there are no active contests on testnet.");
            return;
        }
    }

    println!("\n2. Fetching only future DPNS contests (ending after current time)...");
    let future_contests = sdk
        .get_current_dpns_contests(Some(current_time), None, Some(5))
        .await;

    match future_contests {
        Ok(contests) => {
            println!("Found {} future contested names", contests.len());

            for (name, end_time) in &contests {
                assert!(
                    *end_time >= current_time,
                    "Contest '{}' end time {} should be after current time {}",
                    name,
                    end_time,
                    current_time
                );
                println!("  - '{}' ends at {}", name, end_time);
            }
        }
        Err(e) => {
            println!("Could not fetch future contests: {}", e);
        }
    }

    println!("\n3. Testing pagination with small limit...");
    let paginated_contests = sdk.get_current_dpns_contests(None, None, Some(2)).await;

    match paginated_contests {
        Ok(contests) => {
            println!(
                "Pagination test completed, fetched {} contested names",
                contests.len()
            );

            let names: Vec<_> = contests.keys().cloned().collect();
            let unique_names: std::collections::HashSet<_> = names.iter().cloned().collect();
            assert_eq!(
                names.len(),
                unique_names.len(),
                "Should have no duplicate names in paginated results"
            );
            println!("No duplicate names found in paginated results");
        }
        Err(e) => {
            println!("Pagination test failed: {}", e);
        }
    }

    if let Ok(all_contests) = all_contests {
        if !all_contests.is_empty() {
            let end_times: Vec<_> = all_contests.values().cloned().collect();
            let start_time = *end_times.iter().min().unwrap_or(&current_time);
            let end_time = *end_times.iter().max().unwrap_or(&(current_time + 1000000));

            println!(
                "\n4. Testing with time range [{}, {}]...",
                start_time, end_time
            );
            let range_contests = sdk
                .get_current_dpns_contests(Some(start_time), Some(end_time), Some(10))
                .await;

            match range_contests {
                Ok(contests) => {
                    println!("Found {} contested names in range", contests.len());

                    for (name, contest_time) in &contests {
                        assert!(
                            *contest_time >= start_time && *contest_time <= end_time,
                            "Contest '{}' end time {} should be within range [{}, {}]",
                            name,
                            contest_time,
                            start_time,
                            end_time
                        );
                    }
                    println!("All contests are within the specified time range");
                }
                Err(e) => {
                    println!("Range query failed: {}", e);
                }
            }
        }
    }

    println!("\nAll get_current_dpns_contests tests completed successfully!");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // Requires network connection
async fn test_get_contested_non_resolved_usernames() {
    let sdk = build_testnet_sdk("contested_non_resolved_usernames").await;

    println!("Testing get_contested_non_resolved_usernames...");

    println!("\n1. Fetching all contested non-resolved DPNS usernames with contenders...");
    let non_resolved_names = sdk.get_contested_non_resolved_usernames(Some(20)).await;

    match &non_resolved_names {
        Ok(names_map) => {
            println!(
                "Successfully fetched {} contested non-resolved usernames",
                names_map.len()
            );

            for (i, (name, contest_info)) in names_map.iter().enumerate().take(10) {
                println!(
                    "  {}. '{}' has {} contenders (ends at {} ms)",
                    i + 1,
                    name,
                    contest_info.contenders.contenders.len(),
                    contest_info.end_time
                );

                for (contender_id, votes) in contest_info.contenders.contenders.iter().take(3) {
                    println!(
                        "      - {}: {:?} votes",
                        contender_id
                            .to_string(dpp::platform_value::string_encoding::Encoding::Base58),
                        votes
                    );
                }

                if let Some(abstain) = contest_info.contenders.abstain_vote_tally {
                    println!("      Abstain votes: {}", abstain);
                }
                if let Some(lock) = contest_info.contenders.lock_vote_tally {
                    println!("      Lock votes: {}", lock);
                }
            }

            if names_map.len() > 10 {
                println!("  ... and {} more", names_map.len() - 10);
            }

            let names: Vec<_> = names_map.keys().cloned().collect();
            let mut sorted_names = names.clone();
            sorted_names.sort();
            assert_eq!(names, sorted_names, "BTreeMap keys should be sorted");
            println!("Names are properly sorted");

            for (name, contest_info) in names_map {
                assert!(
                    contest_info.contenders.winner.is_none(),
                    "Name '{}' should not have a winner (it's supposed to be unresolved)",
                    name
                );
            }
            println!("All names are confirmed unresolved (no winners)");
        }
        Err(e) => {
            println!("Could not fetch contested non-resolved names: {}", e);
            println!("This may be expected if there are no contested names on testnet.");
        }
    }

    println!("\n2. Comparing with get_current_dpns_contests results...");
    let current_contests = sdk.get_current_dpns_contests(None, None, Some(10)).await;

    if let (Ok(non_resolved), Ok(contests)) = (&non_resolved_names, &current_contests) {
        let contest_names: std::collections::HashSet<_> = contests.keys().cloned().collect();

        println!("  Names from current contests: {}", contest_names.len());
        println!("  Names from non-resolved query: {}", non_resolved.len());

        for name in contest_names.iter().take(3) {
            if non_resolved.contains_key(name) {
                println!("  '{}' found in both queries", name);
            } else {
                println!("  '{}' in current contests but not in non-resolved", name);
            }
        }
    }

    println!("\n3. Testing with different limits...");

    let limit_5 = sdk.get_contested_non_resolved_usernames(Some(5)).await;
    let limit_10 = sdk.get_contested_non_resolved_usernames(Some(10)).await;

    if let (Ok(names_5), Ok(names_10)) = (limit_5, limit_10) {
        assert!(names_5.len() <= 5, "Should respect limit of 5");
        assert!(names_10.len() <= 10, "Should respect limit of 10");

        let names_5_vec: Vec<_> = names_5.keys().cloned().collect();
        let names_10_vec: Vec<_> = names_10.keys().take(5).cloned().collect();

        if names_5.len() == 5 && names_10.len() >= 5 {
            assert_eq!(names_5_vec, names_10_vec, "First 5 names should match");
            println!("Limits are properly applied");
        }
    }

    println!("\nAll get_contested_non_resolved_usernames tests completed!");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // Requires network connection
async fn test_get_non_resolved_dpns_contests_for_identity() {
    let sdk = build_testnet_sdk("non_resolved_dpns_contests_for_identity").await;

    println!("Testing get_non_resolved_dpns_contests_for_identity...");

    println!("\n1. Getting non-resolved contests to find test identity...");
    let non_resolved = sdk.get_contested_non_resolved_usernames(Some(10)).await;

    match non_resolved {
        Ok(contests) if !contests.is_empty() => {
            let (first_name, first_contest) = contests.iter().next().unwrap();

            if let Some((test_identity, _)) = first_contest.contenders.contenders.iter().next() {
                println!(
                    "Found test identity {} in contest '{}'",
                    test_identity.to_string(dpp::platform_value::string_encoding::Encoding::Base58),
                    first_name
                );

                println!("\n2. Getting contests for identity {}...", test_identity);
                let identity_contests = sdk
                    .get_non_resolved_dpns_contests_for_identity(*test_identity, Some(20))
                    .await;

                match identity_contests {
                    Ok(contests_map) => {
                        println!("Identity is contending in {} contests", contests_map.len());

                        for (name, contest_info) in &contests_map {
                            let is_contender = contest_info
                                .contenders
                                .contenders
                                .iter()
                                .any(|(id, _)| id == test_identity);

                            assert!(
                                is_contender,
                                "Identity should be a contender in contest '{}'",
                                name
                            );

                            println!(
                                "  - '{}' ({} contenders total, ends at {})",
                                name,
                                contest_info.contenders.contenders.len(),
                                contest_info.end_time
                            );
                        }

                        assert!(
                            contests_map.contains_key(first_name),
                            "Should contain the contest '{}' where we found the identity",
                            first_name
                        );

                        println!("All returned contests contain the identity as a contender");
                    }
                    Err(e) => {
                        println!("Failed to get contests for identity: {}", e);
                    }
                }

                println!("\n3. Testing with a random identity (should return empty)...");
                let random_id = Identifier::random();
                let empty_contests = sdk
                    .get_non_resolved_dpns_contests_for_identity(random_id, Some(10))
                    .await;

                match empty_contests {
                    Ok(contests_map) => {
                        assert!(
                            contests_map.is_empty(),
                            "Random identity should not be in any contests"
                        );
                        println!("Random identity correctly returned no contests");
                    }
                    Err(e) => {
                        println!("Failed to query for random identity: {}", e);
                    }
                }
            } else {
                println!("No contenders found in first contest");
            }
        }
        Ok(_) => {
            println!("No non-resolved contests found on testnet to test with");
        }
        Err(e) => {
            println!("Could not fetch non-resolved contests: {}", e);
            println!("This may be expected if there are no contested names on testnet.");
        }
    }

    println!("\nAll get_non_resolved_dpns_contests_for_identity tests completed!");
}

// ---------------------------------------------------------------------------
// Relocated from tests/dpns_queries_test.rs
// ---------------------------------------------------------------------------

// Sample inputs lifted from wasm-sdk docs.html. They're only used to drive
// the SDK code paths — no assertions are made about whether they exist on
// the chain under test, so the smoke test stays chain-agnostic.
const TEST_IDENTITY_ID: &str = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk";
const TEST_USERNAME: &str = "alice";
const TEST_PREFIX: &str = "ali";

/// Chain-agnostic smoke test: exercises every DPNS query path used by the
/// wasm-sdk docs example and asserts only that the SDK completed each call
/// without a transport/proof error. Results are logged, not asserted on, so
/// the test runs against any network — testnet, devnet, or a fresh local
/// chain where "alice" is unregistered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // Requires network connection
async fn test_dpns_queries_smoke() {
    let sdk = build_testnet_sdk("dpns_queries_smoke").await;

    let is_available = sdk
        .check_dpns_name_availability(TEST_USERNAME)
        .await
        .expect("check_dpns_name_availability should succeed");
    println!("'{TEST_USERNAME}' available: {is_available}");

    let maybe_identity = sdk
        .resolve_dpns_name_to_identity(TEST_USERNAME)
        .await
        .expect("resolve_dpns_name_to_identity should succeed");
    match &maybe_identity {
        Some(id) => println!("'{TEST_USERNAME}' resolves to identity: {id}"),
        None => println!("'{TEST_USERNAME}' does not resolve on this chain"),
    }

    let identity_id = dash_sdk::dpp::prelude::Identifier::from_string(
        TEST_IDENTITY_ID,
        dpp::platform_value::string_encoding::Encoding::Base58,
    )
    .expect("identity id should parse");

    let usernames = sdk
        .get_dpns_usernames_by_identity(identity_id, Some(10))
        .await
        .expect("get_dpns_usernames_by_identity should succeed");
    println!(
        "Identity {TEST_IDENTITY_ID} owns {} username(s)",
        usernames.len()
    );

    let search_results = sdk
        .search_dpns_names(TEST_PREFIX, Some(10))
        .await
        .expect("search_dpns_names should succeed");
    println!(
        "search_dpns_names('{TEST_PREFIX}') returned {} result(s)",
        search_results.len()
    );

    let maybe_identity = sdk
        .resolve_dpns_name_to_identity("therealslimshaddy5")
        .await
        .expect("resolve_dpns_name_to_identity should succeed");

    if let Some(identity_id) = maybe_identity {
        let usernames = sdk
            .get_dpns_usernames_by_identity(identity_id, Some(5))
            .await
            .expect("get_dpns_usernames_by_identity should succeed");
        println!(
            "Resolved identity {identity_id} owns {} username(s)",
            usernames.len()
        );
    } else {
        println!("'therealslimshaddy5' does not resolve on this chain");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // Requires network connection
async fn test_dpns_search_variations() {
    let sdk = build_testnet_sdk("dpns_search_variations").await;

    let test_prefixes = vec!["a", "test", "d", "dash", "demo", "user"];

    for prefix in test_prefixes {
        let results = sdk
            .search_dpns_names(prefix, Some(5))
            .await
            .expect("search should succeed");
        assert!(
            results.len() <= 5,
            "search for '{prefix}' should respect the limit of 5"
        );
    }
}
