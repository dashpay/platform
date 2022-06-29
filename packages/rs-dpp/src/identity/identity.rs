use std::collections::BTreeMap;
use std::convert::TryInto;

use ciborium::value::Value as CborValue;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{errors::ProtocolError, identifier::Identifier, metadata::Metadata, util::hash};
use crate::common::bytes_for_system_value_from_tree_map;
use crate::identity::identity_public_key;
use crate::util::cbor_value::{CborBTreeMapHelper, CborCanonicalMap};
use crate::util::deserializer;
use crate::util::json_value::{JsonValueExt, ReplaceWith};

use super::{IdentityPublicKey, KeyID};

// TODO implement!
type InstantAssetLockProof = String;
// TODO implement!
type ChainAssetLockProof = String;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AssetLockProof {
    Instant(InstantAssetLockProof),
    Chain(ChainAssetLockProof),
}

pub const IDENTIFIER_FIELDS: [&str; 1] = ["$id"];

/// Implement the Identity. Identity is a low-level construct that provides the foundation
/// for user-facing functionality on the platform
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub protocol_version: u32,
    pub id: Identifier,
    pub public_keys: Vec<IdentityPublicKey>,
    pub balance: i64,
    pub revision: i64,
    #[serde(skip)]
    pub asset_lock_proof: Option<AssetLockProof>,
    #[serde(skip)]
    pub metadata: Option<Metadata>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IdentityForBuffer {
    // TODO the struct probably should be made from references
    id: Identifier,
    public_keys: Vec<IdentityPublicKey>,
    balance: i64,
    revision: i64,
}

impl From<&Identity> for IdentityForBuffer {
    fn from(i: &Identity) -> Self {
        IdentityForBuffer {
            id: i.id.clone(),
            public_keys: i.public_keys.clone(),
            balance: i.balance,
            revision: i.revision,
        }
    }
}

impl Identity {
    /// Get Identity protocol version
    pub fn get_protocol_version(&self) -> u32 {
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

    /// Get metadata
    pub fn get_metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    /// Set metadata
    pub fn set_metadata(mut self, m: Metadata) -> Self {
        self.metadata = Some(m);
        self
    }

    pub fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        self.to_cbor()
    }

    pub fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        // Prepend protocol version to the result
        let mut buf = self.get_protocol_version().to_le_bytes().to_vec();

        let mut identity_map = CborCanonicalMap::new();

        identity_map.insert("id", self.get_id().to_buffer().to_vec());
        identity_map.insert("balance", self.get_balance());
        identity_map.insert("revision", self.get_revision());
        identity_map.insert(
            "publicKeys",
            self.get_public_keys()
                .iter()
                .map(|pk| pk.into())
                .collect::<Vec<CborValue>>(),
        );

        let mut identity_cbor = identity_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
        buf.append(&mut identity_cbor);

        Ok(buf)
    }

    pub fn from_buffer(b: impl AsRef<[u8]>) -> Result<Self, ProtocolError> {
        Self::from_cbor(b.as_ref())
    }

    pub fn from_cbor(identity_cbor: &[u8]) -> Result<Self, ProtocolError> {
        let (protocol_version_bytes, identity_cbor_bytes) = identity_cbor.split_at(4);

        let protocol_version = deserializer::get_protocol_version(protocol_version_bytes)?;

        // Deserialize the contract
        let identity_map: BTreeMap<String, CborValue> =
            ciborium::de::from_reader(identity_cbor_bytes).map_err(|e| {
                ProtocolError::DecodingError(format!("Unable to decode identity CBOR: {}", e))
            })?;

        let identity_id= identity_map.get_identifier("id")?;
        let revision = identity_map.get_i64("revision")?;
        let balance: i64 = identity_map.get_i64("balance")?;

        let keys_cbor_value = identity_map.get("publicKeys").ok_or_else(|| {
            ProtocolError::DecodingError(String::from(
                "Unable to decode identity CBOR: missing property publicKeys",
            ))
        })?;
        let keys_cbor = keys_cbor_value.as_array().ok_or_else(|| {
            ProtocolError::DecodingError(String::from(
                "Unable to decode identity CBOR: invalid public keys",
            ))
        })?;

        let public_keys = keys_cbor
            .iter()
            .map(IdentityPublicKey::from_cbor_value)
            .collect::<Result<Vec<IdentityPublicKey>, ProtocolError>>()?;

        Ok(Self {
            protocol_version,
            id: identity_id.into(),
            public_keys,
            balance,
            revision,
            asset_lock_proof: None,
            metadata: None,
        })
    }

    pub fn from_raw_object(mut raw_object: JsonValue) -> Result<Identity, ProtocolError> {
        // TODO identifier_default_deserializer: default deserializer should be changed to bytes
        // Identifiers fields should be replaced with the string format to deserialize Identity
        raw_object.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Base58)?;

        if let Some(public_keys_value) = raw_object.get_mut("publicKeys") {
            if let Some(public_keys_array) = public_keys_value.as_array_mut() {
                for public_key in public_keys_array.iter_mut() {
                    public_key.replace_base64_paths(identity_public_key::BINARY_DATA_FIELDS)?;
                }
            }
        }

        let identity: Identity = serde_json::from_value(raw_object)?;

        Ok(identity)
    }

    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash::hash(&self.to_buffer()?))
    }
}

#[test]
fn test_to_buffer() {}
