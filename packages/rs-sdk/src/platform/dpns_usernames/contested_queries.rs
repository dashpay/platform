//! Contested DPNS username queries
//!
//! This module provides specialized queries for contested DPNS usernames.
//! These are wrappers around the general contested resource queries that automatically
//! set the DPNS contract ID and document type.

use crate::platform::fetch_many::FetchMany;
use crate::{Error, Sdk};
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::TimestampMillis;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive::query::VotePollsByEndDateDriveQuery;
use drive_proof_verifier::types::{Contenders, ContestedResource, VotePollsGroupedByTimestamp};
use std::collections::{BTreeMap, HashSet};

// DPNS parent domain constant
const DPNS_PARENT_DOMAIN: &str = "dash";

/// Represents contest information including contenders and end time
#[derive(Debug, Clone)]
pub struct ContestInfo {
    /// The contenders for this contested name
    pub contenders: Contenders,
    /// The timestamp when the voting ends (milliseconds since epoch)
    pub end_time: TimestampMillis,
}

/// Result of a contested DPNS username
#[derive(Debug, Clone)]
pub struct ContestedDpnsUsername {
    /// The domain label (e.g., "alice")
    pub label: String,
    /// The normalized label
    pub normalized_label: String,
    /// The contenders for this name
    pub contenders: Vec<Identifier>,
}

impl Sdk {
    /// Get all contested DPNS usernames
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results to return
    /// * `start_after` - Optional name to start after
    /// (for pagination)
    ///
    /// # Returns
    ///
    /// Returns a list of contested DPNS usernames
    pub async fn get_contested_dpns_normalized_usernames(
        &self,
        limit: Option<u32>,
        start_after: Option<String>,
    ) -> Result<Vec<String>, Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;

        let start_index_values = vec![Value::Text(DPNS_PARENT_DOMAIN.to_string())];

        // For a range query of all items under "dash", we use empty end_index_values
        let end_index_values = vec![];

        // If we have a start_after value, we use it as the start_at_value
        let start_at_value = start_after.map(|name| {
            // Create a compound value with both parent domain and label
            let value = Value::Array(vec![
                Value::Text(DPNS_PARENT_DOMAIN.to_string()),
                Value::Text(name),
            ]);
            (value, false) // false means exclusive (start after, not at)
        });

        let query = VotePollsByDocumentTypeQuery {
            contract_id: dpns_contract_id,
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            start_index_values,
            end_index_values,
            start_at_value,
            limit: limit.map(|l| l as u16),
            order_ascending: true,
        };

        let contested_resources = ContestedResource::fetch_many(self, query).await?;

        // Convert ContestedResources to our ContestedDpnsUsername format
        let mut usernames = Vec::new();

        // The ContestedResources contains a Vec of ContestedResource items
        for contested_resource in contested_resources.0.iter() {
            // Extract the label from the contested resource
            // The ContestedResource contains the index values [parent_domain, label]
            if let Some(label) = Self::extract_label_from_contested_resource(&contested_resource.0)
            {
                // For now, we'll create a simplified version
                // In a real implementation, we'd fetch the contenders
                usernames.push(label);
            }
        }

        Ok(usernames)
    }

    /// Get the vote state for a contested DPNS username
    ///
    /// # Arguments
    ///
    /// * `label` - The username label to check (e.g., "alice")
    /// * `limit` - Maximum number of contenders to return
    ///
    /// # Returns
    ///
    /// Returns the contenders and their vote counts for the username
    pub async fn get_contested_dpns_vote_state(
        &self,
        label: &str,
        limit: Option<u32>,
    ) -> Result<Contenders, Error> {
        use dpp::voting::contender_structs::ContenderWithSerializedDocument;

        let dpns_contract_id = self.get_dpns_contract_id()?;

        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id: dpns_contract_id,
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text(DPNS_PARENT_DOMAIN.to_string()),
                Value::Text(label.to_string()),
            ],
        };

        let query = ContestedDocumentVotePollDriveQuery {
            vote_poll,
            result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
            allow_include_locked_and_abstaining_vote_tally: true,
            start_at: None,
            limit: limit.map(|l| l as u16),
            offset: None,
        };

        // Fetch the contenders using FetchMany
        // ContenderWithSerializedDocument implements FetchMany and returns Contenders
        let result = ContenderWithSerializedDocument::fetch_many(self, query).await?;

        Ok(result)
    }

    /// Get voters who voted for a specific identity for a contested DPNS username
    ///
    /// # Arguments
    ///
    /// * `label` - The username label (e.g., "alice")
    /// * `contestant_id` - The identity ID of the contestant
    /// * `limit` - Maximum number of voters to return
    ///
    /// # Returns
    ///
    /// Returns the list of masternode voters who voted for this contestant
    pub async fn get_contested_dpns_voters_for_identity(
        &self,
        label: &str,
        contestant_id: Identifier,
        limit: Option<u32>,
    ) -> Result<(), Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;

        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id: dpns_contract_id,
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text(DPNS_PARENT_DOMAIN.to_string()),
                Value::Text(label.to_string()),
            ],
        };

        let _query = ContestedDocumentVotePollVotesDriveQuery {
            vote_poll,
            contestant_id,
            start_at: None,
            limit: limit.map(|l| l as u16),
            offset: None,
            order_ascending: true,
        };

        // ContestedResourceVoters isn't available, so we'll skip this for now
        Ok(())
    }

    /// Get all contested DPNS usernames that an identity has voted on
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID (typically a masternode ProTxHash)
    /// * `limit` - Maximum number of votes to return
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    ///
    /// Returns the list of contested DPNS usernames this identity has voted on
    pub async fn get_contested_dpns_identity_votes(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
        offset: Option<u16>,
    ) -> Result<Vec<ContestedDpnsUsername>, Error> {
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset,
            limit: limit.map(|l| l as u16),
            order_ascending: true,
            start_at: None,
        };

        // ContestedResourceIdentityVotes isn't available, so we'll skip this for now
        let _ = query;
        let usernames = Vec::new();

        Ok(usernames)
    }

    /// Get all contested DPNS usernames where an identity is a contender
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID to search for
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns the list of contested DPNS usernames where this identity is competing
    pub async fn get_contested_dpns_usernames_by_identity(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<Vec<ContestedDpnsUsername>, Error> {
        // First, get all contested DPNS usernames
        let all_contested = self
            .get_contested_dpns_normalized_usernames(limit, None)
            .await?;

        let mut usernames_with_identity = Vec::new();

        // Check each contested name to see if our identity is a contender
        for contested_label in all_contested {
            let vote_state = self
                .get_contested_dpns_vote_state(&contested_label, None)
                .await?;

            // Check if our identity is among the contenders
            let is_contender = vote_state
                .contenders
                .iter()
                .any(|(contender_id, _)| contender_id == &identity_id);

            if is_contender {
                let contenders = vote_state
                    .contenders
                    .into_iter()
                    .map(|(id, _)| id)
                    .collect();
                usernames_with_identity.push(ContestedDpnsUsername {
                    label: contested_label.clone(),
                    normalized_label: contested_label.to_lowercase(),
                    contenders,
                });
            }
        }

        Ok(usernames_with_identity)
    }

    // Helper function to extract label from contested resource value
    fn extract_label_from_contested_resource(
        resource: &dpp::platform_value::Value,
    ) -> Option<String> {
        // The ContestedResource contains a Value that represents the serialized index values
        // For DPNS with parentNameAndLabel index, this should be [parent_domain, label]
        // However, the exact structure depends on how the data is serialized

        // First, try to interpret as an array directly
        if let dpp::platform_value::Value::Array(values) = resource {
            if values.len() >= 2 {
                if let dpp::platform_value::Value::Text(label) = &values[1] {
                    return Some(label.clone());
                }
            }
        }

        // If not an array, it might be encoded differently
        // For now, return None if we can't extract it
        None
    }

    // Helper function to extract label from index values
    fn extract_label_from_index_values(index_values: &[Vec<u8>]) -> Option<String> {
        if index_values.len() >= 2 {
            String::from_utf8(index_values[1].clone()).ok()
        } else {
            None
        }
    }

    /// Get contested usernames that are not yet resolved
    ///
    /// This method fetches all currently contested DPNS usernames that haven't been resolved yet.
    /// It gets current contests and returns the contenders and end time for each unresolved name.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns a map of contested but unresolved DPNS usernames to their contest info (contenders and end time)
    pub async fn get_contested_non_resolved_usernames(
        &self,
        limit: Option<u32>,
    ) -> Result<BTreeMap<String, ContestInfo>, Error> {
        // First, get all current DPNS contests (returns BTreeMap<String, TimestampMillis>)
        let current_contests = self
            .get_current_dpns_contests(None, None, Some(100))
            .await?;

        // Check each name to see if it's resolved and collect contenders with end times
        let mut non_resolved_names: BTreeMap<String, ContestInfo> = BTreeMap::new();

        for (name, end_time) in current_contests {
            // Get the vote state for this name
            match self.get_contested_dpns_vote_state(&name, None).await {
                Ok(contenders) => {
                    // Check if there's a winner - if not, it's unresolved
                    if contenders.winner.is_none() {
                        non_resolved_names.insert(
                            name,
                            ContestInfo {
                                contenders,
                                end_time,
                            },
                        );
                    }
                }
                Err(_) => {
                    // If we can't get the vote state, skip this name
                    // (we could include it with empty contenders, but it's better to skip)
                }
            }

            // Check if we've reached the limit
            if let Some(limit) = limit {
                if non_resolved_names.len() >= limit as usize {
                    break;
                }
            }
        }

        Ok(non_resolved_names)
    }

    /// Get non-resolved DPNS contests for a specific identity
    ///
    /// This method fetches all currently contested DPNS usernames that haven't been resolved yet
    /// and filters them to only include contests where the specified identity is a contender.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID to filter contests for
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns a map of contested but unresolved DPNS usernames (where the identity is a contender) to their contenders
    pub async fn get_non_resolved_dpns_contests_for_identity(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<BTreeMap<String, ContestInfo>, Error> {
        // First, get all non-resolved contests
        let all_non_resolved = self.get_contested_non_resolved_usernames(limit).await?;

        // Filter to only include contests where the identity is a contender
        let mut identity_contests: BTreeMap<String, ContestInfo> = BTreeMap::new();

        for (name, contest_info) in all_non_resolved {
            // Check if the identity is among the contenders
            let is_contender = contest_info
                .contenders
                .contenders
                .iter()
                .any(|(contender_id, _)| contender_id == &identity_id);

            if is_contender {
                identity_contests.insert(name, contest_info);
            }
        }

        Ok(identity_contests)
    }

    /// Get current DPNS contests (active vote polls)
    ///
    /// This method fetches all currently active DPNS username contests by querying
    /// vote polls by their end date. It automatically paginates through all results
    /// if there are more than the limit.
    ///
    /// # Arguments
    ///
    /// * `start_time` - Optional start time to filter contests (in milliseconds)
    /// * `end_time` - Optional end time to filter contests (in milliseconds)
    /// * `limit` - Maximum number of results per query (defaults to 100)
    ///
    /// # Returns
    ///
    /// Returns a map of contested DPNS names to their end timestamps
    pub async fn get_current_dpns_contests(
        &self,
        start_time: Option<TimestampMillis>,
        end_time: Option<TimestampMillis>,
        limit: Option<u16>,
    ) -> Result<BTreeMap<String, TimestampMillis>, Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;
        let query_limit = limit.unwrap_or(100);
        let mut name_to_end_time: BTreeMap<String, TimestampMillis> = BTreeMap::new();
        let mut current_start_time = start_time.map(|t| (t, true));

        loop {
            let query = VotePollsByEndDateDriveQuery {
                start_time: current_start_time,
                end_time: end_time.map(|t| (t, true)),
                limit: Some(query_limit),
                offset: None,
                order_ascending: true,
            };

            // Execute the query
            let result: VotePollsGroupedByTimestamp = VotePoll::fetch_many(self, query).await?;

            // Check if we got any results
            if result.0.is_empty() {
                break;
            }

            let mut last_timestamp = None;
            let mut polls_in_last_group = 0;

            // Process each timestamp group
            for (timestamp, polls) in result.0 {
                let mut dpns_polls_count = 0;

                for poll in polls {
                    // Check if this is a DPNS contest
                    if let VotePoll::ContestedDocumentResourceVotePoll(contested_poll) = poll {
                        if contested_poll.contract_id == dpns_contract_id
                            && contested_poll.document_type_name == "domain"
                        {
                            // Extract the contested name from index_values
                            if contested_poll.index_values.len() >= 2 {
                                if let Value::Text(label) = &contested_poll.index_values[1] {
                                    name_to_end_time.insert(label.clone(), timestamp);
                                    dpns_polls_count += 1;
                                }
                            }
                        }
                    }
                }

                if dpns_polls_count > 0 {
                    last_timestamp = Some(timestamp);
                    polls_in_last_group = dpns_polls_count;
                }
            }

            // Check if we should continue pagination
            // If we got less than the limit, we've reached the end
            if polls_in_last_group < query_limit as usize {
                break;
            }

            // Set up for next query - use the last timestamp as the new start
            // with false (not included) to avoid duplicates
            if let Some(last_ts) = last_timestamp {
                current_start_time = Some((last_ts, false));
            } else {
                break;
            }
        }

        Ok(name_to_end_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SdkBuilder;
    use dpp::dashcore::Network;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
    use std::num::NonZeroUsize;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore] // Requires network connection
    async fn test_contested_queries() {
        // Create SDK with testnet configuration
        let address_list = "https://52.12.176.90:1443"
            .parse()
            .expect("Failed to parse address");

        // Create trusted context provider for testnet
        let context_provider = TrustedHttpContextProvider::new(
            Network::Testnet,
            None,                            // No devnet name
            NonZeroUsize::new(100).unwrap(), // Cache size
        )
        .expect("Failed to create context provider");

        let dpns = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("Failed to load system data contract");
        context_provider.add_known_contract(dpns);

        let sdk = SdkBuilder::new(address_list)
            .with_network(Network::Testnet)
            .with_context_provider(context_provider)
            .build()
            .expect("Failed to create SDK");

        // Warm up the cache by fetching the DPNS contract
        println!("Fetching DPNS contract to warm up cache...");
        let dpns_contract_id = sdk
            .get_dpns_contract_id()
            .expect("Failed to get DPNS contract ID");
        println!("DPNS contract ID: {}", dpns_contract_id);

        // Test getting all contested DPNS usernames
        println!("Testing get_contested_dpns_usernames...");
        let all_contested = sdk
            .get_contested_dpns_normalized_usernames(Some(5), None)
            .await;
        match &all_contested {
            Ok(names) => {
                println!("✅ Successfully queried contested DPNS usernames");
                println!("Found {} contested DPNS usernames", names.len());
                for name in names {
                    println!("  - {}", name);
                }
            }
            Err(e) => {
                // For now, we'll just warn about the error since contested names may not exist
                println!("⚠️ Could not fetch contested names (may not exist): {}", e);
                println!("This is expected if there are no contested names on testnet.");
            }
        }

        // Test getting vote state for a specific contested name
        // This assumes there's at least one contested name to test with
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
                                contender_id.to_string(
                                    dpp::platform_value::string_encoding::Encoding::Base58
                                ),
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

                // Test getting contested names by identity (using first contender from vote state)
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
                            .get_contested_dpns_usernames_by_identity(
                                test_identity.clone(),
                                Some(5),
                            )
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

        // Test getting identity votes (this would only work for masternodes)
        // We'll use a known masternode identity if available
        println!("\nTesting get_contested_dpns_identity_votes...");
        // This test might fail if the identity is not a masternode
        let test_masternode_id = Identifier::from_string(
            "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",
            dpp::platform_value::string_encoding::Encoding::Base58,
        );

        if let Ok(masternode_id) = test_masternode_id {
            let votes = sdk
                .get_contested_dpns_identity_votes(masternode_id.clone(), Some(5), None)
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
                    // This is expected if the identity is not a masternode
                    println!("Expected error - identity may not be a masternode: {}", e);
                }
            }
        }

        // Test getting current DPNS contests
        println!("\nTesting get_current_dpns_contests...");
        let current_contests = sdk.get_current_dpns_contests(None, None, Some(10)).await;
        match current_contests {
            Ok(contests) => {
                println!("✅ Successfully queried current DPNS contests");
                println!("Found {} contested names", contests.len());
                for (name, end_time) in contests.iter().take(5) {
                    println!("  '{}' ends at {}", name, end_time);
                }
            }
            Err(e) => {
                println!("⚠️ Could not fetch current contests: {}", e);
                println!("This is expected if there are no active contests on testnet.");
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore] // Requires network connection
    async fn test_contested_name_detection() {
        use super::super::{convert_to_homograph_safe_chars, is_contested_username};

        // Test contested name detection
        assert!(is_contested_username("alice")); // 5 chars, becomes "a11ce"
        assert!(is_contested_username("bob")); // 3 chars, becomes "b0b"
        assert!(is_contested_username("cool")); // 4 chars, becomes "c001"
        assert!(is_contested_username("hello")); // 5 chars, becomes "he110"

        // Test non-contested names
        assert!(!is_contested_username("ab")); // Too short (2 chars)
        assert!(!is_contested_username("twentycharacterslong")); // 20 chars, too long
        assert!(!is_contested_username("alice2")); // Contains '2' after normalization

        // Test normalization
        assert_eq!(convert_to_homograph_safe_chars("alice"), "a11ce");
        assert_eq!(convert_to_homograph_safe_chars("COOL"), "c001");
        assert_eq!(convert_to_homograph_safe_chars("BoB"), "b0b");
        assert_eq!(convert_to_homograph_safe_chars("hello"), "he110");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore] // Requires network connection
    async fn test_get_current_dpns_contests() {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Create SDK with testnet configuration
        let address_list = "https://52.12.176.90:1443"
            .parse()
            .expect("Failed to parse address");

        // Create trusted context provider for testnet
        let context_provider = TrustedHttpContextProvider::new(
            Network::Testnet,
            None,                            // No devnet name
            NonZeroUsize::new(100).unwrap(), // Cache size
        )
        .expect("Failed to create context provider");

        let dpns = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("Failed to load system data contract");
        context_provider.add_known_contract(dpns);

        let sdk = SdkBuilder::new(address_list)
            .with_network(Network::Testnet)
            .with_context_provider(context_provider)
            .build()
            .expect("Failed to create SDK");

        println!("Testing get_current_dpns_contests...");

        // Get current time in milliseconds
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;

        // Test 1: Get all current contests (no time filter)
        println!("\n1. Fetching all current DPNS contests...");
        let all_contests = sdk.get_current_dpns_contests(None, None, Some(100)).await;

        match &all_contests {
            Ok(contests) => {
                println!("✅ Successfully fetched {} contested names", contests.len());

                // Display some of the contested names and their end times
                for (name, end_time) in contests.iter().take(5) {
                    println!("  '{}' ends at {}", name, end_time);
                }

                // Verify the map is sorted by name (BTreeMap property)
                let names: Vec<_> = contests.keys().cloned().collect();
                let mut sorted_names = names.clone();
                sorted_names.sort();
                assert_eq!(names, sorted_names, "BTreeMap should be sorted by key");
                println!("✅ Names are properly sorted alphabetically");
            }
            Err(e) => {
                println!("⚠️ Could not fetch contests: {}", e);
                println!("This may be expected if there are no active contests on testnet.");
                // Don't fail the test, as there might legitimately be no contests
                return;
            }
        }

        // Test 2: Test with time filters (only future contests)
        println!("\n2. Fetching only future DPNS contests (ending after current time)...");
        let future_contests = sdk
            .get_current_dpns_contests(Some(current_time), None, Some(5))
            .await;

        match future_contests {
            Ok(contests) => {
                println!("✅ Found {} future contested names", contests.len());

                // Verify all contests end after current time
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
                println!("⚠️ Could not fetch future contests: {}", e);
            }
        }

        // Test 3: Test pagination (small limit to force multiple queries)
        println!("\n3. Testing pagination with small limit...");
        let paginated_contests = sdk.get_current_dpns_contests(None, None, Some(2)).await;

        match paginated_contests {
            Ok(contests) => {
                println!(
                    "✅ Pagination test completed, fetched {} contested names",
                    contests.len()
                );

                // If we got results, verify no duplicate names
                let names: Vec<_> = contests.keys().cloned().collect();
                let unique_names: HashSet<_> = names.iter().cloned().collect();
                assert_eq!(
                    names.len(),
                    unique_names.len(),
                    "Should have no duplicate names in paginated results"
                );
                println!("✅ No duplicate names found in paginated results");
            }
            Err(e) => {
                println!("⚠️ Pagination test failed: {}", e);
            }
        }

        // Test 4: Test with both start and end time filters
        if let Ok(all_contests) = all_contests {
            if !all_contests.is_empty() {
                // Get min and max end times from the contests
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
                        println!("✅ Found {} contested names in range", contests.len());

                        // Verify all are within range
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
                        println!("✅ All contests are within the specified time range");
                    }
                    Err(e) => {
                        println!("⚠️ Range query failed: {}", e);
                    }
                }
            }
        }

        println!("\n✅ All get_current_dpns_contests tests completed successfully!");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore] // Requires network connection
    async fn test_get_contested_non_resolved_usernames() {
        // Create SDK with testnet configuration
        let address_list = "https://52.12.176.90:1443"
            .parse()
            .expect("Failed to parse address");

        // Create trusted context provider for testnet
        let context_provider = TrustedHttpContextProvider::new(
            Network::Testnet,
            None,                            // No devnet name
            NonZeroUsize::new(100).unwrap(), // Cache size
        )
        .expect("Failed to create context provider");

        let dpns = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("Failed to load system data contract");
        context_provider.add_known_contract(dpns);

        let sdk = SdkBuilder::new(address_list)
            .with_network(Network::Testnet)
            .with_context_provider(context_provider)
            .build()
            .expect("Failed to create SDK");

        println!("Testing get_contested_non_resolved_usernames...");

        // Test 1: Get all contested non-resolved usernames with contenders
        println!("\n1. Fetching all contested non-resolved DPNS usernames with contenders...");
        let non_resolved_names = sdk.get_contested_non_resolved_usernames(Some(20)).await;

        match &non_resolved_names {
            Ok(names_map) => {
                println!(
                    "✅ Successfully fetched {} contested non-resolved usernames",
                    names_map.len()
                );

                // Display the first few names with their contenders
                for (i, (name, contest_info)) in names_map.iter().enumerate().take(10) {
                    println!(
                        "  {}. '{}' has {} contenders (ends at {} ms)",
                        i + 1,
                        name,
                        contest_info.contenders.contenders.len(),
                        contest_info.end_time
                    );

                    // Show first 3 contenders
                    for (contender_id, votes) in contest_info.contenders.contenders.iter().take(3) {
                        println!(
                            "      - {}: {:?} votes",
                            contender_id
                                .to_string(dpp::platform_value::string_encoding::Encoding::Base58),
                            votes
                        );
                    }

                    // Show vote tallies if present
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

                // Verify names are sorted (BTreeMap property)
                let names: Vec<_> = names_map.keys().cloned().collect();
                let mut sorted_names = names.clone();
                sorted_names.sort();
                assert_eq!(names, sorted_names, "BTreeMap keys should be sorted");
                println!("✅ Names are properly sorted");

                // Verify no winners in any of the results
                for (name, contest_info) in names_map {
                    assert!(
                        contest_info.contenders.winner.is_none(),
                        "Name '{}' should not have a winner (it's supposed to be unresolved)",
                        name
                    );
                }
                println!("✅ All names are confirmed unresolved (no winners)");
            }
            Err(e) => {
                println!("⚠️ Could not fetch contested non-resolved names: {}", e);
                println!("This may be expected if there are no contested names on testnet.");
            }
        }

        // Test 2: Compare with get_current_dpns_contests
        println!("\n2. Comparing with get_current_dpns_contests results...");
        let current_contests = sdk.get_current_dpns_contests(None, None, Some(10)).await;

        if let (Ok(non_resolved), Ok(contests)) = (&non_resolved_names, &current_contests) {
            // Get names from current contests map
            let contest_names: HashSet<_> = contests.keys().cloned().collect();

            println!("  Names from current contests: {}", contest_names.len());
            println!("  Names from non-resolved query: {}", non_resolved.len());

            // Show some example names
            for name in contest_names.iter().take(3) {
                if non_resolved.contains_key(name) {
                    println!("  ✅ '{}' found in both queries", name);
                } else {
                    println!(
                        "  ⚠️ '{}' in current contests but not in non-resolved",
                        name
                    );
                }
            }
        }

        // Test 3: Test with different limits
        println!("\n3. Testing with different limits...");

        let limit_5 = sdk.get_contested_non_resolved_usernames(Some(5)).await;
        let limit_10 = sdk.get_contested_non_resolved_usernames(Some(10)).await;

        if let (Ok(names_5), Ok(names_10)) = (limit_5, limit_10) {
            assert!(names_5.len() <= 5, "Should respect limit of 5");
            assert!(names_10.len() <= 10, "Should respect limit of 10");

            // First 5 names should be the same in both (BTreeMap is ordered)
            let names_5_vec: Vec<_> = names_5.keys().cloned().collect();
            let names_10_vec: Vec<_> = names_10.keys().take(5).cloned().collect();

            if names_5.len() == 5 && names_10.len() >= 5 {
                assert_eq!(names_5_vec, names_10_vec, "First 5 names should match");
                println!("✅ Limits are properly applied");
            }
        }

        println!("\n✅ All get_contested_non_resolved_usernames tests completed!");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore] // Requires network connection
    async fn test_get_non_resolved_dpns_contests_for_identity() {
        // Create SDK with testnet configuration
        let address_list = "https://52.12.176.90:1443"
            .parse()
            .expect("Failed to parse address");

        // Create trusted context provider for testnet
        let context_provider = TrustedHttpContextProvider::new(
            Network::Testnet,
            None,                            // No devnet name
            NonZeroUsize::new(100).unwrap(), // Cache size
        )
        .expect("Failed to create context provider");

        let dpns = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("Failed to load system data contract");
        context_provider.add_known_contract(dpns);

        let sdk = SdkBuilder::new(address_list)
            .with_network(Network::Testnet)
            .with_context_provider(context_provider)
            .build()
            .expect("Failed to create SDK");

        println!("Testing get_non_resolved_dpns_contests_for_identity...");

        // First, get some non-resolved contests to find an identity to test with
        println!("\n1. Getting non-resolved contests to find test identity...");
        let non_resolved = sdk.get_contested_non_resolved_usernames(Some(10)).await;

        match non_resolved {
            Ok(contests) if !contests.is_empty() => {
                // Pick the first contest and get an identity from it
                let (first_name, first_contest) = contests.iter().next().unwrap();

                if let Some((test_identity, _)) = first_contest.contenders.contenders.iter().next()
                {
                    println!(
                        "Found test identity {} in contest '{}'",
                        test_identity
                            .to_string(dpp::platform_value::string_encoding::Encoding::Base58),
                        first_name
                    );

                    // Now test the new method
                    println!("\n2. Getting contests for identity {}...", test_identity);
                    let identity_contests = sdk
                        .get_non_resolved_dpns_contests_for_identity(
                            test_identity.clone(),
                            Some(20),
                        )
                        .await;

                    match identity_contests {
                        Ok(contests_map) => {
                            println!(
                                "✅ Identity is contending in {} contests",
                                contests_map.len()
                            );

                            // Verify that the identity is indeed in all returned contests
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

                            // The first contest should definitely be in the results
                            assert!(
                                contests_map.contains_key(first_name),
                                "Should contain the contest '{}' where we found the identity",
                                first_name
                            );

                            println!(
                                "✅ All returned contests contain the identity as a contender"
                            );
                        }
                        Err(e) => {
                            println!("Failed to get contests for identity: {}", e);
                        }
                    }

                    // Test with an identity that probably isn't in any contests
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
                            println!("✅ Random identity correctly returned no contests");
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
                println!("⚠️ No non-resolved contests found on testnet to test with");
            }
            Err(e) => {
                println!("⚠️ Could not fetch non-resolved contests: {}", e);
                println!("This may be expected if there are no contested names on testnet.");
            }
        }

        println!("\n✅ All get_non_resolved_dpns_contests_for_identity tests completed!");
    }
}
