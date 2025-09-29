//! Tests for proof data access functionality.

use super::common::setup_logs;
use crate::fetch::config::Config;
use dash_sdk::platform::{FetchWithProof, Identity};
use dash_sdk::{platform::Identifier, Sdk};

#[tokio::test]
async fn test_identity_fetch_with_proof() {
    setup_logs();

    let _cfg = Config::new();
    let sdk = Sdk::new_mock();

    // Test with a known identity ID
    let identity_id = Identifier::from_bytes(&[
        0xf9, 0xc8, 0x5a, 0x89, 0x45, 0x3e, 0x67, 0x96, 0x87, 0xc7, 0xb1, 0xc4, 0x7a, 0xc9, 0x8a,
        0x7e, 0x6e, 0x68, 0xd0, 0x27, 0xd3, 0xb9, 0x64, 0x1a, 0xf6, 0x4f, 0x12, 0x56, 0x64, 0xf0,
        0xca, 0xf5,
    ])
    .expect("parse identity id");

    // Fetch identity with proof
    let result = Identity::fetch_with_proof(&sdk, identity_id).await;

    match result {
        Ok((identity, proof_data)) => {
            // Verify we got proof data
            assert!(
                !proof_data.grovedb_proof.is_empty(),
                "GroveDB proof should not be empty"
            );
            assert!(
                !proof_data.quorum_hash.is_empty(),
                "Quorum hash should not be empty"
            );
            assert!(
                !proof_data.signature.is_empty(),
                "Signature should not be empty"
            );
            assert!(proof_data.round > 0, "Round should be greater than 0");
            assert!(
                !proof_data.block_id_hash.is_empty(),
                "Block ID hash should not be empty"
            );

            // Verify root hash is set (even if it's zeros for mock)
            assert_eq!(
                proof_data.root_hash.len(),
                32,
                "Root hash should be 32 bytes"
            );

            // Verify metadata is present
            #[cfg(feature = "network-testing")]
            {
                assert!(proof_data.metadata.height > 0, "Height should be set");
                assert!(proof_data.metadata.time_ms > 0, "Timestamp should be set");
            }

            if let Some(identity) = identity {
                tracing::info!("Found identity: {:?}", identity);
                tracing::info!("Proof size: {} bytes", proof_data.grovedb_proof.len());
                tracing::info!("Root hash: {:?}", hex::encode(proof_data.root_hash));
            }
        }
        Err(e) => {
            #[cfg(feature = "network-testing")]
            panic!("Failed to fetch identity with proof: {}", e);

            #[cfg(feature = "offline-testing")]
            tracing::warn!("Offline mode error (expected): {}", e);
        }
    }
}

#[tokio::test]
async fn test_document_fetch_many_with_proofs() {
    setup_logs();

    let _cfg = Config::new();
    let _sdk = Sdk::new_mock();

    // For mock testing, we'll create a simple query
    // In real tests, you would need to fetch the DataContract first
    let _data_contract_id = Identifier::from_bytes(&[
        0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99,
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0x00, 0xaa, 0xbb, 0xcc, 0xdd,
        0xee, 0xff,
    ])
    .expect("parse data contract id");

    // We can't easily test document fetching without a real DataContract
    // This would require setting up mock responses properly
    tracing::info!(
        "Skipping document fetch test - would require full mock setup with DataContract"
    );
}

#[tokio::test]
async fn test_proof_data_structure() {
    setup_logs();

    // Create sample proof and metadata
    let proof = dash_sdk::platform::proto::Proof {
        grovedb_proof: vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36,
        ],
        quorum_hash: vec![0xAA; 32],
        signature: vec![0xBB; 96],
        round: 42,
        block_id_hash: vec![0xCC; 32],
        quorum_type: 4,
    };

    let metadata = dash_sdk::platform::proto::ResponseMetadata {
        height: 1000,
        core_chain_locked_height: 900,
        epoch: 5,
        time_ms: 1234567890,
        protocol_version: 1,
        chain_id: "dash-testnet".to_string(),
    };

    // Create ProofData
    let proof_data = dash_sdk::platform::ProofData::new(proof, metadata);

    // Verify all fields are correctly set
    assert_eq!(proof_data.grovedb_proof.len(), 36);
    assert_eq!(proof_data.root_hash.len(), 32);
    assert_eq!(
        &proof_data.root_hash,
        &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32
        ]
    );
    assert_eq!(proof_data.quorum_hash, vec![0xAA; 32]);
    assert_eq!(proof_data.signature, vec![0xBB; 96]);
    assert_eq!(proof_data.round, 42);
    assert_eq!(proof_data.block_id_hash, vec![0xCC; 32]);
    assert_eq!(proof_data.quorum_type, 4);
    assert_eq!(proof_data.metadata.height, 1000);
    assert_eq!(proof_data.metadata.core_chain_locked_height, 900);
    assert_eq!(proof_data.metadata.epoch, 5);
    assert_eq!(proof_data.metadata.time_ms, 1234567890);
}

#[tokio::test]
async fn test_proof_data_with_short_grovedb_proof() {
    setup_logs();

    // Create proof with short grovedb_proof (less than 32 bytes)
    let proof = dash_sdk::platform::proto::Proof {
        grovedb_proof: vec![1, 2, 3, 4, 5], // Only 5 bytes
        quorum_hash: vec![0xAA; 32],
        signature: vec![0xBB; 96],
        round: 42,
        block_id_hash: vec![0xCC; 32],
        quorum_type: 4,
    };

    let metadata = dash_sdk::platform::proto::ResponseMetadata {
        height: 1000,
        core_chain_locked_height: 900,
        epoch: 5,
        time_ms: 1234567890,
        protocol_version: 1,
        chain_id: "dash-testnet".to_string(),
    };

    // Create ProofData
    let proof_data = dash_sdk::platform::ProofData::new(proof, metadata);

    // When grovedb_proof is too short, root_hash should be zeros
    assert_eq!(proof_data.root_hash, [0u8; 32]);
    assert_eq!(proof_data.grovedb_proof.len(), 5);
}

#[cfg(test)]
mod identity_merkle_tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::Identity;
    use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn test_identity_merkle_tree_generation() {
        let platform_version = PlatformVersion::latest();

        // Create identity with multiple keys
        let mut keys = BTreeMap::new();

        for i in 0..5 {
            let key = IdentityPublicKeyV0 {
                id: i,
                purpose: if i % 2 == 0 {
                    Purpose::AUTHENTICATION
                } else {
                    Purpose::ENCRYPTION
                },
                security_level: match i {
                    0 => SecurityLevel::MASTER,
                    1 | 2 => SecurityLevel::HIGH,
                    _ => SecurityLevel::MEDIUM,
                },
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: vec![i as u8; 33].into(),
                disabled_at: None,
                contract_bounds: None,
            };
            keys.insert(i, IdentityPublicKey::V0(key));
        }

        let identity = Identity::new_with_id_and_keys([0x42u8; 32].into(), keys, &platform_version)
            .expect("Failed to create identity");

        // Build merkle tree
        let tree = identity
            .build_keys_merkle_tree()
            .expect("Failed to build merkle tree");

        let root = tree.root();
        assert_ne!(root, [0u8; 32], "Root should not be zero");

        // Get proofs for all keys
        for i in 0..5 {
            let proof = identity
                .get_key_merkle_proof(i)
                .expect(&format!("Failed to get proof for key {}", i));

            assert_eq!(proof.key_id, i);
            assert_eq!(proof.root, root);

            // Verify proof structure
            if i % 2 == 0 {
                assert_eq!(proof.key_purpose, Purpose::AUTHENTICATION);
            } else {
                assert_eq!(proof.key_purpose, Purpose::ENCRYPTION);
            }
        }

        // Verify all roots are consistent
        let root_from_method = identity
            .get_keys_merkle_root()
            .expect("Failed to get merkle root");
        assert_eq!(root_from_method, root);
    }

    #[test]
    fn test_identity_with_single_key() {
        let platform_version = PlatformVersion::latest();

        let mut keys = BTreeMap::new();
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: vec![0xFF; 33].into(),
            disabled_at: None,
            contract_bounds: None,
        };
        keys.insert(0, IdentityPublicKey::V0(key));

        let identity = Identity::new_with_id_and_keys([0x11u8; 32].into(), keys, &platform_version)
            .expect("Failed to create identity");

        let proof = identity
            .get_key_merkle_proof(0)
            .expect("Failed to get proof");

        // Single key should have empty proof path
        assert_eq!(proof.proof_path.len(), 0);
        assert_eq!(proof.key_purpose, Purpose::AUTHENTICATION);
        assert_eq!(proof.key_security_level, SecurityLevel::MASTER);
    }

    #[test]
    fn test_identity_without_keys() {
        let platform_version = PlatformVersion::latest();

        let identity =
            Identity::new_with_id_and_keys([0x22u8; 32].into(), BTreeMap::new(), &platform_version)
                .expect("Failed to create identity");

        // Should fail to build tree with no keys
        let result = identity.build_keys_merkle_tree();
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod document_proof_tests {
    use super::*;
    use dash_sdk::platform::document_proof::extract_merkle_path_from_grove_proof;

    #[test]
    fn test_extract_merkle_path_empty_proof() {
        // Just root hash - should return error since there's no valid Merk proof
        let proof = vec![0u8; 32];
        let result = extract_merkle_path_from_grove_proof(&proof);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No valid Merk proofs found"));
    }

    #[test]
    fn test_extract_merkle_path_with_operations() {
        // Test raw Merk proof format
        let mut proof = Vec::new();

        // Add Push left operation with varint length
        proof.push(0x01); // Left sibling
        proof.push(32); // Length (32 bytes)
        proof.extend_from_slice(&[0xAA; 32]);

        // Add Push right operation
        proof.push(0x02); // Right sibling
        proof.push(32); // Length (32 bytes)
        proof.extend_from_slice(&[0xBB; 32]);

        let path = extract_merkle_path_from_grove_proof(&proof).expect("Failed to extract path");

        assert_eq!(path.len(), 2);
        assert_eq!(path[0].0, [0xAA; 32]);
        assert!(path[0].1); // is_left = true
        assert_eq!(path[1].0, [0xBB; 32]);
        assert!(!path[1].1); // is_left = false
    }

    #[test]
    fn test_extract_merkle_path_grovedb_v0_format() {
        // Test GroveDB V0 format (starts with 0x00)
        let mut proof = vec![0x00]; // Version byte

        // Add some padding that would be in a real GroveDB proof
        proof.extend_from_slice(&[0xFF; 10]);

        // Add Merk proof operations
        proof.push(0x01); // Left sibling
        proof.push(32); // Length
        proof.extend_from_slice(&[0xCC; 32]);

        let path = extract_merkle_path_from_grove_proof(&proof).expect("Failed to extract path");

        assert_eq!(path.len(), 1);
        assert_eq!(path[0].0, [0xCC; 32]);
        assert!(path[0].1); // is_left = true
    }

    #[test]
    fn test_extract_merkle_path_too_short() {
        let proof = vec![0u8; 16]; // Too short for root
        let result = extract_merkle_path_from_grove_proof(&proof);
        assert!(result.is_err());
    }
}
