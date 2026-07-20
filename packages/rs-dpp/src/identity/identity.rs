use crate::address_funds::PlatformAddress;
use crate::identity::v0::IdentityV0;
use crate::identity::{IdentityPublicKey, KeyID};
use crate::prelude::{AddressNonce, Revision};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;

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
   feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$formatVersion"),
    // platform_version_path("dpp.identity_versions.identity_structure_version")
)]
#[cfg_attr(
    feature = "identity-serialization",
    derive(Encode, Decode, PlatformDeserialize, PlatformSerialize),
    platform_serialize(limit = 15000, unversioned)
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
pub enum Identity {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for Identity {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for PartialIdentity {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl ValueConvertible for PartialIdentity {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use platform_value::{platform_value, BinaryData, Value};
    use serde_json::json;

    fn fixture_pubkey(id: u32, byte: u8) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![byte; 33]),
            disabled_at: None,
        })
    }

    fn fixture() -> Identity {
        let mut public_keys = BTreeMap::new();
        public_keys.insert(0, fixture_pubkey(0, 0xa0));
        public_keys.insert(1, fixture_pubkey(1, 0xb1));
        Identity::V0(IdentityV0 {
            id: Identifier::new([0x42; 32]),
            public_keys,
            balance: 1_000_000,
            revision: 7,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Internally-tagged enum (`tag = "$formatVersion"`); inner V0 has
        // `rename_all = "camelCase"`. `public_keys` uses a custom serde wrapper
        // that emits a `Vec` of `IdentityPublicKey` values (keys dropped, then
        // reconstructed on deserialize from each key's `id`). Each
        // `IdentityPublicKey` is itself an internally-tagged enum, so the inner
        // wire shape mirrors the per-key test in
        // `identity_public_key::mod::json_convertible_tests`.
        // Sized-int fields with JSON loss:
        // - `balance`: u64 (Credits)
        // - `revision`: u64 (Revision)
        // - inner `id`: u32, `purpose`/`securityLevel`/`type`: u8 reprs.
        // `Identifier` serializes as base58 in JSON; `BinaryData` as base64.
        // Purpose::AUTHENTICATION = 0, KeyType::ECDSA_SECP256K1 = 0,
        // SecurityLevel::MASTER = 0.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "id": "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf",
                // After Phase D step 4, `disabled_at` carries
                // `#[serde(skip_serializing_if = "Option::is_none")]`, so
                // non-disabled keys no longer emit `disabledAt: null`.
                "publicKeys": [
                    {
                        "$formatVersion": "0",
                        "id": 0,
                        "purpose": 0,
                        "securityLevel": 0,
                        "contractBounds": serde_json::Value::Null,
                        "type": 0,
                        "readOnly": false,
                        "data": "oKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCg",
                    },
                    {
                        "$formatVersion": "0",
                        "id": 1,
                        "purpose": 0,
                        "securityLevel": 0,
                        "contractBounds": serde_json::Value::Null,
                        "type": 0,
                        "readOnly": false,
                        "data": "sbGxsbGxsbGxsbGxsbGxsbGxsbGxsbGxsbGxsbGxsbGx",
                    },
                ],
                "balance": 1_000_000u64,
                "revision": 7,
            })
        );
        let recovered = Identity::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Explicit `u32` / `u8` / `u64` suffixes lock typed-int variants.
        // `Identifier` interpolates as `Value::Identifier`; `BinaryData` of
        // length 33 lacks a fixed-sized variant, so it stays as
        // `Value::Bytes(Vec<u8>)`.
        let id = Identifier::new([0x42; 32]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "id": id,
                // `disabledAt: None` is now stripped per the
                // `skip_serializing_if` attribute (Phase D step 4).
                "publicKeys": [
                    {
                        "$formatVersion": "0",
                        "id": 0u32,
                        "purpose": 0u8,
                        "securityLevel": 0u8,
                        "contractBounds": Value::Null,
                        "type": 0u8,
                        "readOnly": false,
                        "data": Value::Bytes(vec![0xa0; 33]),
                    },
                    {
                        "$formatVersion": "0",
                        "id": 1u32,
                        "purpose": 0u8,
                        "securityLevel": 0u8,
                        "contractBounds": Value::Null,
                        "type": 0u8,
                        "readOnly": false,
                        "data": Value::Bytes(vec![0xb1; 33]),
                    },
                ],
                "balance": 1_000_000u64,
                "revision": 7u64,
            })
        );
        let recovered = Identity::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}

/// An identity struct that represent partially set/loaded identity data.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::accessors::IdentityGettersV0;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use platform_value::{BinaryData, Identifier};
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use std::collections::BTreeMap;

    fn sample_key(id: u32) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0x42; 33]),
            disabled_at: None,
        })
    }

    #[test]
    fn default_versioned_returns_default_v0() {
        let identity =
            Identity::default_versioned(LATEST_PLATFORM_VERSION).expect("default should succeed");
        assert_eq!(identity.id(), Identifier::default());
        assert_eq!(identity.balance(), 0);
        assert_eq!(identity.revision(), 0);
        assert!(identity.public_keys().is_empty());
    }

    #[test]
    fn new_with_id_and_keys_preserves_inputs() {
        let id = Identifier::from([4u8; 32]);
        let mut keys: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
        keys.insert(0, sample_key(0));
        keys.insert(1, sample_key(1));

        let identity = Identity::new_with_id_and_keys(id, keys.clone(), LATEST_PLATFORM_VERSION)
            .expect("new_with_id_and_keys");
        assert_eq!(identity.id(), id);
        assert_eq!(identity.balance(), 0);
        assert_eq!(identity.revision(), 0);
        assert_eq!(identity.public_keys().len(), 2);
    }

    #[test]
    fn into_partial_identity_info_preserves_balance_and_revision() {
        let mut keys: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
        keys.insert(0, sample_key(0));
        let v0 = IdentityV0 {
            id: Identifier::from([5u8; 32]),
            public_keys: keys,
            balance: 123,
            revision: 7,
        };
        let identity: Identity = v0.clone().into();
        let partial = identity.into_partial_identity_info();
        assert_eq!(partial.id, v0.id);
        assert_eq!(partial.balance, Some(123));
        assert_eq!(partial.revision, Some(7));
        assert_eq!(partial.loaded_public_keys.len(), 1);
        assert!(partial.not_found_public_keys.is_empty());
    }

    #[test]
    fn into_partial_identity_info_no_balance_drops_balance() {
        let v0 = IdentityV0 {
            id: Identifier::from([6u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 999,
            revision: 2,
        };
        let identity: Identity = v0.into();
        let partial = identity.into_partial_identity_info_no_balance();
        assert!(partial.balance.is_none());
        assert_eq!(partial.revision, Some(2));
    }

    #[test]
    fn from_v0_conversion_works() {
        let v0 = IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 1,
            revision: 1,
        };
        let identity: Identity = v0.clone().into();
        match identity {
            Identity::V0(inner) => assert_eq!(inner, v0),
        }
    }

    #[test]
    fn clone_and_equality() {
        let id = Identifier::from([3u8; 32]);
        let identity =
            Identity::new_with_id_and_keys(id, BTreeMap::new(), LATEST_PLATFORM_VERSION).unwrap();
        let clone = identity.clone();
        assert_eq!(identity, clone);
    }

    #[cfg(feature = "identity-hashing")]
    #[test]
    fn hash_is_stable_for_same_identity() {
        let id = Identifier::from([8u8; 32]);
        let identity =
            Identity::new_with_id_and_keys(id, BTreeMap::new(), LATEST_PLATFORM_VERSION).unwrap();
        let h1 = identity.hash().unwrap();
        let h2 = identity.hash().unwrap();
        assert_eq!(h1, h2);
        // The hash is a fixed-size SHA256-double, 32 bytes.
        assert_eq!(h1.len(), 32);
    }

    #[cfg(feature = "identity-hashing")]
    #[test]
    fn hash_differs_for_different_identities() {
        let a = Identity::new_with_id_and_keys(
            Identifier::from([0u8; 32]),
            BTreeMap::new(),
            LATEST_PLATFORM_VERSION,
        )
        .unwrap();
        let b = Identity::new_with_id_and_keys(
            Identifier::from([1u8; 32]),
            BTreeMap::new(),
            LATEST_PLATFORM_VERSION,
        )
        .unwrap();
        assert_ne!(a.hash().unwrap(), b.hash().unwrap());
    }

    #[cfg(feature = "state-transitions")]
    #[test]
    fn new_with_input_addresses_and_keys_is_deterministic() {
        use crate::address_funds::PlatformAddress;

        let mut inputs: BTreeMap<PlatformAddress, (u32, u64)> = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x11; 20]), (1, 0));
        inputs.insert(PlatformAddress::P2pkh([0x22; 20]), (2, 0));

        let keys: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();

        let a = Identity::new_with_input_addresses_and_keys(
            &inputs,
            keys.clone(),
            LATEST_PLATFORM_VERSION,
        )
        .unwrap();
        let b = Identity::new_with_input_addresses_and_keys(
            &inputs,
            keys.clone(),
            LATEST_PLATFORM_VERSION,
        )
        .unwrap();
        // Deterministic derivation: same inputs -> same id.
        assert_eq!(a.id(), b.id());
    }

    #[cfg(feature = "state-transitions")]
    #[test]
    fn new_with_input_addresses_and_keys_fails_on_empty_inputs() {
        use crate::address_funds::PlatformAddress;
        let inputs: BTreeMap<PlatformAddress, (u32, u64)> = BTreeMap::new();
        let keys: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();

        let result =
            Identity::new_with_input_addresses_and_keys(&inputs, keys, LATEST_PLATFORM_VERSION);
        assert!(result.is_err());
    }
}
