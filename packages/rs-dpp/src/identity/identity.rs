use crate::identity::v0::identity::IdentityV0;
use crate::identity::{IdentityPublicKey, KeyID};
use crate::prelude::Revision;
use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};
use crate::util::hash;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use derive_more::From;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_versioning::PlatformSerdeVersionedDeserialize;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(
    Debug,
    Serialize,
    PlatformSerdeVersionedDeserialize,
    Encode,
    Decode,
    Clone,
    PartialEq,
    PlatformDeserialize,
    PlatformSerialize,
    From,
)]
#[platform_error_type(ProtocolError)]
#[platform_deserialize_limit(15000)]
#[platform_serialize_limit(15000)]
#[serde(untagged)]
pub enum Identity {
    #[versioned(0)]
    V0(IdentityV0),
}

/// An identity struct that represent partially set/loaded identity data.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PartialIdentity {
    pub id: Identifier,
    pub loaded_public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    pub balance: Option<u64>,
    pub revision: Option<Revision>,
    /// These are keys that were requested but didn't exist
    pub not_found_public_keys: BTreeSet<KeyID>,
}

impl Identity {
    /// Computes the hash of an identity
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash::hash_to_vec(PlatformSerializable::serialize(self)?))
    }
}
