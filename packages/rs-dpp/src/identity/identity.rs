use crate::identity::v0::IdentityV0;
use crate::identity::{IdentityPublicKey, KeyID};
use crate::prelude::{AssetLockProof, Revision};
use crate::serialization::ValueConvertible;
use crate::serialization::{PlatformDeserializable, PlatformSerializable};
use crate::util::hash;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_versioning::{PlatformSerdeVersionedDeserialize, PlatformVersioned};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// The identity is not stored inside of drive, because of this, the serialization is mainly for
/// transport, the serialization of the identity will include the version, so no passthrough or
/// untagged is needed here
#[derive(Debug, Clone, PartialEq, From)]
#[cfg_attr(
    feature = "identity-serde-conversion",
    derive(Serialize, PlatformSerdeVersionedDeserialize),
    serde(untagged),
    platform_version_path("dpp.identity_versions.identity_structure_version")
)]
#[cfg_attr(
    feature = "identity-serialization",
    derive(Encode, Decode, PlatformDeserialize, PlatformSerialize),
    platform_serialize(limit = 15000, unversioned)
)]
pub enum Identity {
    #[cfg_attr(feature = "identity-serde-conversion", versioned(0))]
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
    #[cfg(feature = "identity-hashing")]
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

        /// Convenience method to get Partial Identity Info
        pub fn into_partial_identity_info(self) -> PartialIdentity {
            match self {
                Identity::V0(v0) => v0.into_partial_identity_info()
            }
        }

        /// Convenience method to get Partial Identity Info
        pub fn into_partial_identity_info_no_balance(self) -> PartialIdentity {
            match self {
                Identity::V0(v0) => v0.into_partial_identity_info_no_balance()
            }
        }
    }
