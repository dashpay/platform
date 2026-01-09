use crate::address_funds::PlatformAddress;
use crate::identity::v0::IdentityV0;
use crate::identity::{IdentityPublicKey, KeyID};
use crate::prelude::{AddressNonce, Revision};

#[cfg(feature = "identity-hashing")]
use crate::serialization::PlatformSerializable;
#[cfg(feature = "identity-hashing")]
use crate::util::hash;
use crate::version::PlatformVersion;

use crate::ProtocolError;
#[cfg(feature = "identity-serialization")]
use bincode::{Decode, Encode};
use derive_more::From;
#[cfg(feature = "identity-serialization")]
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;

use crate::fee::Credits;
use std::collections::{BTreeMap, BTreeSet};

/// The identity is not stored inside of drive, because of this, the serialization is mainly for
/// transport, the serialization of the identity will include the version, so no passthrough or
/// untagged is needed here
#[derive(Debug, Clone, PartialEq, From)]
#[cfg_attr(
   any( feature = "identity-serde-conversion" ,feature = "state-transition-serde-conversion",),
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$version"),
    // platform_version_path("dpp.identity_versions.identity_structure_version")
)]
#[cfg_attr(
    feature = "identity-serialization",
    derive(Encode, Decode, PlatformDeserialize, PlatformSerialize),
    platform_serialize(limit = 15000, unversioned)
)]
pub enum Identity {
    #[cfg_attr(
        any(
            feature = "identity-serde-conversion",
            feature = "state-transition-serde-conversion"
        ),
        serde(rename = "0")
    )]
    V0(IdentityV0),
}

/// An identity struct that represent partially set/loaded identity data.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(
    any(
        feature = "identity-serde-conversion",
        feature = "state-transition-serde-conversion",
    ),
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct PartialIdentity {
    pub id: Identifier,
    pub loaded_public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    pub balance: Option<Credits>,
    pub revision: Option<Revision>,
    /// These are keys that were requested but didn't exist
    pub not_found_public_keys: BTreeSet<KeyID>,
}

impl Identity {
    #[cfg(feature = "identity-hashing")]
    /// Computes the hash of an identity
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash::hash_double_to_vec(
            PlatformSerializable::serialize_to_bytes(self)?,
        ))
    }

    pub fn default_versioned(
        platform_version: &PlatformVersion,
    ) -> Result<Identity, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(Identity::V0(IdentityV0::default())),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Created a new identity based on asset locks and keys
    pub fn new_with_id_and_keys(
        id: Identifier,
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
                    id,
                    public_keys,
                    balance: 0,
                    revision: 0,
                };
                Ok(identity_v0.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::new_with_id_and_keys".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Create a new identity using input [PlatformAddress]es.
    ///
    /// This function derives the identity ID from the provided input addresses.
    ///
    /// ## Arguments
    ///
    /// * `inputs` - A map of `PlatformAddress` to `(AddressNonce, Credits)`.
    ///   The identity id is derived from the addresses and nonces (credits are ignored for the id derivation).
    ///   The nonces should represent state after creation of the identity (e.g. be incremented by 1).
    /// * `public_keys` - A map of KeyID to IdentityPublicKey tuples representing the public keys for the identity.
    /// * `platform_version` - The platform version to use for identity creation.
    ///
    /// ## Returns
    ///
    /// * `Result<Identity, ProtocolError>` - Returns the newly created Identity or a ProtocolError if the operation fails.
    #[cfg(feature = "state-transitions")]
    pub fn new_with_input_addresses_and_keys(
        inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
        platform_version: &PlatformVersion,
    ) -> Result<Identity, ProtocolError> {
        use crate::state_transition::identity_id_from_input_addresses;

        let identity_id = identity_id_from_input_addresses(inputs)?;
        Self::new_with_id_and_keys(identity_id, public_keys, platform_version)
    }

    /// Convenience method to get Partial Identity Info
    pub fn into_partial_identity_info(self) -> PartialIdentity {
        match self {
            Identity::V0(v0) => v0.into_partial_identity_info(),
        }
    }

    /// Convenience method to get Partial Identity Info
    pub fn into_partial_identity_info_no_balance(self) -> PartialIdentity {
        match self {
            Identity::V0(v0) => v0.into_partial_identity_info_no_balance(),
        }
    }
}
