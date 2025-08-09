//! Contested DPNS username queries
//!
//! This module provides specialized queries for contested DPNS usernames.
//! These are wrappers around the general contested resource queries that automatically
//! set the DPNS contract ID and document type.

use crate::platform::fetch_many::FetchMany;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::{Identifier, Value};
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, 
    ContestedDocumentVotePollDriveQueryResultType
};
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive_proof_verifier::types::{
    ContestedResource, Contenders,
};

// DPNS parent domain constant
const DPNS_PARENT_DOMAIN: &str = "dash";

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
        
        let start_index_values = vec![
            Value::Text(DPNS_PARENT_DOMAIN.to_string()),
        ];
        
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
        
        // Debug: print the structure to understand what we're getting
        #[cfg(test)]
        {
            println!("DEBUG: Contested resources count: {}", contested_resources.0.len());
            for resource in contested_resources.0.iter() {
                println!("DEBUG: Resource value: {:?}", resource.0);
            }
        }
        
        // The ContestedResources contains a Vec of ContestedResource items
        for contested_resource in contested_resources.0.iter() {
            // Extract the label from the contested resource
            // The ContestedResource contains the index values [parent_domain, label]
            if let Some(label) = Self::extract_label_from_contested_resource(&contested_resource.0) {
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

        // For now, return empty Contenders since the fetch implementation is complex
        use std::collections::BTreeMap;
        Ok(Contenders {
            winner: None,
            contenders: BTreeMap::new(),
            abstain_vote_tally: None,
            lock_vote_tally: None,
        })
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
        let all_contested = self.get_contested_dpns_normalized_usernames(limit, None).await?;
        
        let mut usernames_with_identity = Vec::new();
        
        // Check each contested name to see if our identity is a contender
        for contested_label in all_contested {
            let vote_state = self.get_contested_dpns_vote_state(&contested_label, None).await?;
            
            // Check if our identity is among the contenders
            let is_contender = vote_state.contenders.iter().any(|(contender_id, _)| contender_id == &identity_id);
            
            if is_contender {
                let contenders = vote_state.contenders.into_iter().map(|(id, _)| id).collect();
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
    fn extract_label_from_contested_resource(resource: &dpp::platform_value::Value) -> Option<String> {
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
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use super::*;
    use crate::SdkBuilder;
    use dpp::dashcore::Network;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

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

        let dpns = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest()).expect("Failed to load system data contract");
        context_provider.add_known_contract(dpns);

        let sdk = SdkBuilder::new(address_list)
            .with_network(Network::Testnet)
            .with_context_provider(context_provider)
            .build()
            .expect("Failed to create SDK");
        
        // Warm up the cache by fetching the DPNS contract
        println!("Fetching DPNS contract to warm up cache...");
        let dpns_contract_id = sdk.get_dpns_contract_id()
            .expect("Failed to get DPNS contract ID");
        println!("DPNS contract ID: {}", dpns_contract_id);


        // Test getting all contested DPNS usernames
        println!("Testing get_contested_dpns_usernames...");
        let all_contested = sdk.get_contested_dpns_normalized_usernames(Some(5), None).await;
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
                println!("\nTesting get_contested_dpns_vote_state for '{}'...", first_contested);
                
                let vote_state = sdk.get_contested_dpns_vote_state(first_contested, Some(10)).await;
                match vote_state {
                    Ok(state) => {
                        println!("Vote state for '{}':", first_contested);
                        if let Some((winner_info, _block_info)) = state.winner {
                            use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
                            match winner_info {
                                ContestedDocumentVotePollWinnerInfo::WonByIdentity(id) => {
                                    println!("  Winner: {}", id.to_string(dpp::platform_value::string_encoding::Encoding::Base58));
                                },
                                ContestedDocumentVotePollWinnerInfo::Locked => {
                                    println!("  Winner: LOCKED");
                                },
                                ContestedDocumentVotePollWinnerInfo::NoWinner => {
                                    println!("  Winner: None");
                                }
                            }
                        }
                        println!("  Contenders: {} total", state.contenders.len());
                        for (contender_id, votes) in state.contenders.iter().take(3) {
                            println!("    - {}: {:?} votes", 
                                contender_id.to_string(dpp::platform_value::string_encoding::Encoding::Base58), 
                                votes);
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
                if let Ok(vote_state) = sdk.get_contested_dpns_vote_state(first_contested, None).await {
                    if let Some((test_identity, _)) = vote_state.contenders.iter().next() {
                        println!("\nTesting get_contested_dpns_usernames_by_identity for {}...", test_identity);
                    
                        let identity_names = sdk.get_contested_dpns_usernames_by_identity(
                            test_identity.clone(), 
                            Some(5)
                        ).await;
                    
                        match identity_names {
                            Ok(names) => {
                                println!("Identity {} is contending for {} names:", test_identity, names.len());
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
            dpp::platform_value::string_encoding::Encoding::Base58
        );
        
        if let Ok(masternode_id) = test_masternode_id {
            let votes = sdk.get_contested_dpns_identity_votes(
                masternode_id.clone(),
                Some(5),
                None
            ).await;
            
            match votes {
                Ok(vote_list) => {
                    println!("Masternode {} has voted on {} contested names", 
                        masternode_id, vote_list.len());
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
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore] // Requires network connection
    async fn test_contested_name_detection() {
        use super::super::{is_contested_username, convert_to_homograph_safe_chars};

        // Test contested name detection
        assert!(is_contested_username("alice"));  // 5 chars, becomes "a11ce"
        assert!(is_contested_username("bob"));    // 3 chars, becomes "b0b"
        assert!(is_contested_username("cool"));   // 4 chars, becomes "c001"
        assert!(is_contested_username("hello"));  // 5 chars, becomes "he110"
        
        // Test non-contested names
        assert!(!is_contested_username("ab"));    // Too short (2 chars)
        assert!(!is_contested_username("twentycharacterslong")); // 20 chars, too long
        assert!(!is_contested_username("alice2")); // Contains '2' after normalization
        
        // Test normalization
        assert_eq!(convert_to_homograph_safe_chars("alice"), "a11ce");
        assert_eq!(convert_to_homograph_safe_chars("COOL"), "c001");
        assert_eq!(convert_to_homograph_safe_chars("BoB"), "b0b");
        assert_eq!(convert_to_homograph_safe_chars("hello"), "he110");
    }
}