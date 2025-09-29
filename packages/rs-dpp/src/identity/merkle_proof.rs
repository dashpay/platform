//! Merkle proof generation for Identity keys
//!
//! This module provides functionality to generate merkle proofs for identity keys,
//! which are useful for zero-knowledge proof applications.

use crate::identity::accessors::IdentityGettersV0;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel};
use crate::ProtocolError;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Merkle proof for a specific key in an identity
#[derive(Debug, Clone)]
pub struct KeyMerkleProof {
    /// The merkle root of all keys
    pub root: [u8; 32],
    /// The ID of the key being proved
    pub key_id: KeyID,
    /// The purpose of the key
    pub key_purpose: Purpose,
    /// The security level of the key
    pub key_security_level: SecurityLevel,
    /// The merkle proof path from leaf to root
    /// Each element is (sibling_hash, is_left) where is_left indicates if the sibling is on the left
    pub proof_path: Vec<([u8; 32], bool)>,
}

/// A simple binary merkle tree implementation
#[derive(Debug)]
pub struct MerkleTree {
    /// The root hash of the tree
    root: [u8; 32],
    /// All nodes in the tree, organized by level
    /// Level 0 is the leaves, last level is the root
    levels: Vec<Vec<[u8; 32]>>,
    /// Mapping from key_id to leaf index
    key_to_index: BTreeMap<KeyID, usize>,
}

impl MerkleTree {
    /// Get the root hash of the tree
    pub fn root(&self) -> [u8; 32] {
        self.root
    }

    /// Generate a merkle proof for a specific key
    pub fn generate_proof(&self, key_id: KeyID) -> Result<Vec<([u8; 32], bool)>, ProtocolError> {
        let leaf_index = self
            .key_to_index
            .get(&key_id)
            .ok_or_else(|| ProtocolError::Generic(format!("Key {} not found in tree", key_id)))?;

        let mut proof = Vec::new();
        let mut current_index = *leaf_index;

        // Walk up the tree from leaf to root
        for level_idx in 0..self.levels.len() - 1 {
            let current_level = &self.levels[level_idx];

            // Determine sibling index and position
            let sibling_index = if current_index % 2 == 0 {
                // Current node is left, sibling is right
                current_index + 1
            } else {
                // Current node is right, sibling is left
                current_index - 1
            };

            // Get sibling hash if it exists
            if sibling_index < current_level.len() {
                let sibling_hash = current_level[sibling_index];
                let is_left = sibling_index < current_index;
                proof.push((sibling_hash, is_left));
            }

            // Move to parent index in next level
            current_index /= 2;
        }

        Ok(proof)
    }
}

/// Hash function for merkle tree nodes
/// Uses SHA256 with proper domain separation
fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"MerkleNode");
    hasher.update(left);
    hasher.update(right);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Hash a leaf node containing identity key data
/// Leaf = H(H(H(key_id || public_key) || purpose) || security_level)
fn hash_leaf(key_id: KeyID, key: &IdentityPublicKey) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // First layer: H(key_id || public_key)
    hasher.update(b"KeyLeaf1");
    hasher.update(&key_id.to_le_bytes());
    hasher.update(key.data().as_slice());
    let layer1 = hasher.finalize_reset();

    // Second layer: H(layer1 || purpose)
    hasher.update(b"KeyLeaf2");
    hasher.update(&layer1);
    hasher.update(&[key.purpose() as u8]);
    let layer2 = hasher.finalize_reset();

    // Third layer: H(layer2 || security_level)
    hasher.update(b"KeyLeaf3");
    hasher.update(&layer2);
    hasher.update(&[key.security_level() as u8]);
    let result = hasher.finalize();

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

impl Identity {
    /// Build a merkle tree from all keys in this identity
    /// Each leaf is: H(H(H(key_id || public_key) || purpose) || security_level)
    pub fn build_keys_merkle_tree(&self) -> Result<MerkleTree, ProtocolError> {
        let keys = self.public_keys();

        if keys.is_empty() {
            return Err(ProtocolError::Generic("Identity has no keys".to_string()));
        }

        // Create sorted list of keys by ID
        let mut sorted_keys: Vec<(KeyID, &IdentityPublicKey)> =
            keys.iter().map(|(id, key)| (*id, key)).collect();
        sorted_keys.sort_by_key(|(id, _)| *id);

        // Build key_to_index mapping
        let key_to_index: BTreeMap<KeyID, usize> = sorted_keys
            .iter()
            .enumerate()
            .map(|(idx, (key_id, _))| (*key_id, idx))
            .collect();

        // Create leaf hashes
        let mut levels = Vec::new();
        let leaves: Vec<[u8; 32]> = sorted_keys
            .iter()
            .map(|(key_id, key)| hash_leaf(*key_id, key))
            .collect();

        levels.push(leaves.clone());

        // Build tree levels bottom-up
        let mut current_level = leaves;

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                if i + 1 < current_level.len() {
                    // Hash two nodes together
                    let hash = hash_node(&current_level[i], &current_level[i + 1]);
                    next_level.push(hash);
                } else {
                    // Odd number of nodes, promote the last one
                    next_level.push(current_level[i]);
                }
            }

            levels.push(next_level.clone());
            current_level = next_level;
        }

        let root = current_level[0];

        Ok(MerkleTree {
            root,
            levels,
            key_to_index,
        })
    }

    /// Get merkle proof for a specific key
    pub fn get_key_merkle_proof(&self, key_id: KeyID) -> Result<KeyMerkleProof, ProtocolError> {
        let tree = self.build_keys_merkle_tree()?;

        let key = self
            .get_public_key_by_id(key_id)
            .ok_or_else(|| ProtocolError::Generic(format!("Key {} not found", key_id)))?;

        let proof_path = tree.generate_proof(key_id)?;

        Ok(KeyMerkleProof {
            root: tree.root(),
            key_id,
            key_purpose: key.purpose(),
            key_security_level: key.security_level(),
            proof_path,
        })
    }

    /// Get just the merkle root of all keys
    pub fn get_keys_merkle_root(&self) -> Result<[u8; 32], ProtocolError> {
        Ok(self.build_keys_merkle_tree()?.root())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::IdentityPublicKey;
    use crate::identity::KeyType;
    use crate::version::PlatformVersion;

    #[test]
    fn test_merkle_tree_single_key() {
        let platform_version = PlatformVersion::latest();

        // Create identity with single key
        let mut keys = BTreeMap::new();
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: vec![1, 2, 3, 4, 5].into(),
            disabled_at: None,
            contract_bounds: None,
        };
        keys.insert(0, IdentityPublicKey::V0(key));

        let identity =
            Identity::new_with_id_and_keys([1u8; 32].into(), keys, &platform_version).unwrap();

        let tree = identity.build_keys_merkle_tree().unwrap();
        assert_eq!(tree.levels.len(), 1); // Only root level for single node

        let proof = identity.get_key_merkle_proof(0).unwrap();
        assert_eq!(proof.key_id, 0);
        assert_eq!(proof.proof_path.len(), 0); // No siblings for single key
    }

    #[test]
    fn test_merkle_tree_multiple_keys() {
        let platform_version = PlatformVersion::latest();

        // Create identity with multiple keys
        let mut keys = BTreeMap::new();
        for i in 0..4 {
            let key = IdentityPublicKeyV0 {
                id: i,
                purpose: if i < 2 {
                    Purpose::AUTHENTICATION
                } else {
                    Purpose::ENCRYPTION
                },
                security_level: if i == 0 {
                    SecurityLevel::MASTER
                } else {
                    SecurityLevel::HIGH
                },
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: vec![i as u8; 33].into(),
                disabled_at: None,
                contract_bounds: None,
            };
            keys.insert(i, IdentityPublicKey::V0(key));
        }

        let identity =
            Identity::new_with_id_and_keys([1u8; 32].into(), keys, &platform_version).unwrap();

        let tree = identity.build_keys_merkle_tree().unwrap();
        assert!(tree.levels.len() > 1);

        // Test getting proof for each key
        for i in 0..4 {
            let proof = identity.get_key_merkle_proof(i).unwrap();
            assert_eq!(proof.key_id, i);
            assert!(proof.proof_path.len() > 0);

            // Verify proof structure
            if i < 2 {
                assert_eq!(proof.key_purpose, Purpose::AUTHENTICATION);
            } else {
                assert_eq!(proof.key_purpose, Purpose::ENCRYPTION);
            }

            if i == 0 {
                assert_eq!(proof.key_security_level, SecurityLevel::MASTER);
            } else {
                assert_eq!(proof.key_security_level, SecurityLevel::HIGH);
            }
        }

        // Check that root is consistent
        let root1 = identity.get_keys_merkle_root().unwrap();
        let root2 = identity.get_key_merkle_proof(0).unwrap().root;
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_merkle_proof_verification() {
        let platform_version = PlatformVersion::latest();

        // Create identity with 3 keys
        let mut keys = BTreeMap::new();
        for i in 0..3 {
            let key = IdentityPublicKeyV0 {
                id: i,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: vec![i as u8; 33].into(),
                disabled_at: None,
                contract_bounds: None,
            };
            keys.insert(i, IdentityPublicKey::V0(key));
        }

        let identity =
            Identity::new_with_id_and_keys([1u8; 32].into(), keys, &platform_version).unwrap();

        // Get proof for middle key
        let proof = identity.get_key_merkle_proof(1).unwrap();

        // Manually verify the proof
        let key = identity.get_public_key_by_id(1).unwrap();
        let mut current_hash = hash_leaf(1, key);

        for (sibling_hash, is_left) in &proof.proof_path {
            current_hash = if *is_left {
                hash_node(sibling_hash, &current_hash)
            } else {
                hash_node(&current_hash, sibling_hash)
            };
        }

        assert_eq!(current_hash, proof.root);
    }
}
