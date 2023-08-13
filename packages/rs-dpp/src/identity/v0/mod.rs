mod conversion;
#[cfg(feature = "random-identities")]
pub mod random;

use std::collections::{BTreeMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::hash::{Hash, Hasher};

use platform_value::{ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::{
    identity_public_key, IdentityPublicKey, KeyID, KeyType, PartialIdentity, Purpose, SecurityLevel,
};
use crate::prelude::Revision;
#[cfg(feature = "cbor")]
use crate::util::cbor_value::{CborBTreeMapHelper, CborCanonicalMap};
use crate::{errors::ProtocolError, identifier::Identifier, metadata::Metadata, util::hash};
use bincode::{Decode, Encode};
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;

/// Implement the Identity. Identity is a low-level construct that provides the foundation
/// for user-facing functionality on the platform
#[derive(Default, Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "identity-serialization", derive(Encode, Decode))]
#[cfg_attr(
    feature = "identity-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct IdentityV0 {
    pub id: Identifier,
    #[cfg_attr(
        feature = "identity-serde-conversion",
        serde(with = "public_key_serialization")
    )]
    pub public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    pub balance: u64,
    pub revision: Revision,
}

impl Hash for IdentityV0 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

mod public_key_serialization {
    use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use crate::identity::{IdentityPublicKey, KeyID};
    use serde::ser::SerializeSeq;
    use serde::{Deserialize, Serializer};
    use std::collections::BTreeMap;

    /// deserialize_public_keys deserializes public keys from a vector
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let public_key_vec: Vec<IdentityPublicKey> = Deserialize::deserialize(deserializer)?;
        Ok(public_key_vec.into_iter().map(|k| (k.id(), k)).collect())
    }

    pub fn serialize<S>(
        public_keys: &BTreeMap<KeyID, IdentityPublicKey>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(public_keys.len()))?;
        for element in public_keys.values() {
            seq.serialize_element(element)?;
        }
        seq.end()
    }
}

impl IdentityV0 {
    /// Get Identity protocol version
    pub fn get_feature_version(&self) -> u16 {
        0
    }

    /// Convenience method to get Partial Identity Info
    pub fn into_partial_identity_info(self) -> PartialIdentity {
        let Self {
            id,
            public_keys,
            balance,
            revision,
            ..
        } = self;
        PartialIdentity {
            id,
            loaded_public_keys: public_keys,
            balance: Some(balance),
            revision: Some(revision),
            not_found_public_keys: Default::default(),
        }
    }

    /// Convenience method to get Partial Identity Info
    pub fn into_partial_identity_info_no_balance(self) -> PartialIdentity {
        let Self {
            id,
            public_keys,
            revision,
            ..
        } = self;
        PartialIdentity {
            id,
            loaded_public_keys: public_keys,
            balance: None,
            revision: Some(revision),
            not_found_public_keys: Default::default(),
        }
    }
}

#[cfg(feature = "identity-value-conversion")]
impl TryFrom<Value> for IdentityV0 {
    type Error = ProtocolError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value).map_err(ProtocolError::ValueError)
    }
}

#[cfg(feature = "identity-value-conversion")]
impl TryFrom<&Value> for IdentityV0 {
    type Error = ProtocolError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value.clone()).map_err(ProtocolError::ValueError)
    }
}
