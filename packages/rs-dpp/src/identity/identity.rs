use crate::identity::conversion::platform_value::IdentityPlatformValueConversionMethodsV0;
use crate::identity::v0::IdentityV0;
use crate::identity::{IdentityPublicKey, KeyID};
use crate::prelude::{AssetLockProof, Revision};
use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};
use crate::util::hash;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use derive_more::From;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_versioning::PlatformSerdeVersionedDeserialize;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use crate::serialization_traits::ValueConvertible;

/// The identity is not stored inside of drive, because of this, the serialization is mainly for
/// transport, the serialization of the identity will include the version, so no passthrough or
/// untagged is needed here
#[derive(
    Debug,
    Serialize,
    PlatformSerdeVersionedDeserialize,
    Clone,
    PartialEq,
    PlatformDeserialize,
    PlatformSerialize,
    From,
)]
#[platform_error_type(ProtocolError)]
#[platform_serialize(limit = 15000, allow_nested)]
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

    /// Created a new identity based on asset locks and keys
    pub fn new_with_asset_lock_and_keys(
        asset_lock_proof: AssetLockProof,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
        platform_version: &PlatformVersion,
    ) -> Result<Identity, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => {
                let identity_v0 = IdentityV0 {
                    id: asset_lock_proof.create_identifier()?,
                    public_keys,
                    balance: 0,
                    revision: 0,
                };
                Ok(identity_v0.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::new_with_asset_lock_and_keys".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
