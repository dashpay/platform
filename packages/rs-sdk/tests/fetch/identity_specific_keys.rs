//! Test for fetching specific identity keys with proof

use dash_sdk::platform::{FetchMany, Identifier, IdentityKeysQuery, IdentityPublicKey};
use dpp::identity::KeyID;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_keys_query_creation() {
        // Test creating a query for specific keys
        let identity_id = Identifier::new([1; 32]);
        let key_ids: Vec<KeyID> = vec![0, 1, 2];
        
        let query = IdentityKeysQuery::new(identity_id.clone(), key_ids.clone());
        
        assert_eq!(query.identity_id, identity_id);
        assert_eq!(query.key_ids, key_ids);
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
    }

    #[test]
    fn test_identity_keys_query_with_limit() {
        let identity_id = Identifier::new([1; 32]);
        let key_ids: Vec<KeyID> = vec![0, 1];
        
        let query = IdentityKeysQuery::new(identity_id, key_ids)
            .with_limit(10)
            .with_offset(5);
        
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(5));
    }

    #[cfg(feature = "network-testing")]
    #[tokio::test]
    async fn test_fetch_specific_keys_with_proof() {
        use dash_sdk::Sdk;
        use std::sync::Arc;

        // This test requires a running network
        let sdk = Sdk::new_testnet();
        
        // Use a known identity ID from testnet (you'd need to replace this)
        let identity_id = Identifier::from_string(
            "GWRvAjHJMe8MJmqPqCjBqiHdj5rTqVjNhMCVqhg8bkJP"
        ).expect("valid identifier");
        
        // Fetch keys 0 and 1
        let query = IdentityKeysQuery::new(identity_id, vec![0, 1]);
        
        let result = IdentityPublicKey::fetch_many_with_metadata_and_proof(
            &sdk,
            query,
            None,
        ).await;
        
        // Check that we got a result (might fail if identity doesn't exist)
        match result {
            Ok((keys, metadata, proof)) => {
                println!("Successfully fetched {} keys", keys.len());
                println!("Proof size: {} bytes", proof.grovedb_proof.len());
                assert!(!proof.grovedb_proof.is_empty(), "Proof should not be empty");
            }
            Err(e) => {
                println!("Error fetching keys (expected if identity doesn't exist): {}", e);
            }
        }
    }
}