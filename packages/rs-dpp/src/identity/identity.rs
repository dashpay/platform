use serde::{Deserialize, Serialize};

use super::{IdentityPublicKey, KeyID};
use crate::identifier::Identifier;

// TODO implement!
type InstantAssetLockProof = String;
// TODO implement!
type ChainAssetLockProof = String;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AssetLockProof {
    Instant(InstantAssetLockProof),
    Chain(ChainAssetLockProof),
}

/// Implement the Identity. Identity is a low-level construct that provides the foundation
/// for user-facing functionality on the platform
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub protocol_version: i32,
    pub id: Identifier,
    pub public_keys: Vec<IdentityPublicKey>,
    pub balance: i64,
    pub revision: i64,
    #[serde(skip)]
    asset_lock_proof: Option<AssetLockProof>,
}

impl Identity {
    /// Get Identity protocol version
    pub fn get_protocol_version(&self) -> i32 {
        self.protocol_version
    }

    /// Returns Identity id
    pub fn get_id(&self) -> &Identifier {
        &self.id
    }

    /// Set Identity public key
    pub fn set_public_keys(mut self, pub_key: Vec<IdentityPublicKey>) -> Self {
        self.public_keys = pub_key;
        self
    }

    /// Get Identity public keys revision
    pub fn get_public_keys(&self) -> &[IdentityPublicKey] {
        &self.public_keys
    }

    // Returns a public key for a given id
    pub fn get_public_key_by_id(&self, key_id: KeyID) -> Option<&IdentityPublicKey> {
        self.public_keys.iter().find(|i| i.id == key_id)
    }

    /// Returns balance
    pub fn get_balance(&self) -> i64 {
        self.balance
    }

    /// Set Identity balance
    pub fn set_balance(mut self, balance: i64) -> Self {
        self.balance = balance;
        self
    }

    /// Increase Identity balance
    pub fn increase_balance(mut self, amount: u64) -> Self {
        self.balance += amount as i64;
        self
    }

    /// Reduce the Identity balance
    pub fn reduce_balance(mut self, amount: u64) -> Self {
        self.balance -= amount as i64;
        self
    }

    /// Set Identity asset lock
    pub fn set_asset_lock_proof(mut self, lock: AssetLockProof) -> Self {
        self.asset_lock_proof = Some(lock);
        self
    }

    /// Get Identity asset lock
    pub fn get_asset_lock_proof(&self) -> Option<&AssetLockProof> {
        self.asset_lock_proof.as_ref()
    }

    /// Set Identity revision
    pub fn set_revision(mut self, revision: i64) -> Self {
        self.revision = revision;
        self
    }

    /// Get Identity revision
    pub fn get_revision(&self) -> i64 {
        self.revision
    }

    // how to convert data to buffer
    pub fn to_buffer() -> Vec<u8> {
        /// first we need the implementation to_buffer
        unimplemented!()
    }

    pub fn hash() -> [u8; 32] {
        unimplemented!()
    }
}

#[test]
fn test_to_buffer() {}
