#![allow(clippy::from_over_into)]

use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};

mod key_type;
mod purpose;
mod security_level;
pub use key_type::KeyType;
pub use purpose::Purpose;
pub use security_level::SecurityLevel;
pub mod accessors;
pub mod conversion;
pub mod fields;
pub mod v0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
pub use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

pub mod methods;
pub use methods::*;
pub mod contract_bounds;
#[cfg(feature = "random-public-keys")]
mod random;

pub type KeyID = u32;
pub type KeyCount = KeyID;
pub type TimestampMillis = u64;

/// A public key belonging to an identity, with its purpose, security level, and
/// key material.
///
/// **Internally-tagged** serde enum (`#[serde(tag = "$formatVersion")]`) that
/// **also** derives native bincode `Encode`/`Decode`. Through bincode, use only
/// the **native** codec (`bincode::encode_to_vec` / `bincode::decode_from_slice`),
/// never `bincode::serde`: the serde bridge is non-self-describing, so it writes
/// a silently-unreadable blob and then fails to decode with `AnyNotSupported`
/// (finding the `$formatVersion` tag needs `deserialize_any`, unsupported by
/// bincode's serde deserializer). The self-describing serde paths
/// (`platform_value` value-conversion, JSON) are fine. This exact class bit the
/// wallet-storage `identity_keys` blob codec, fixed with `IdentityKeyWire`; the
/// same shape later recurred for `AssetLockProof` (#4133, `AssetLockEntryWire`).
#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    From,
    Hash,
    Ord,
    PartialOrd,
)]
#[platform_serialize(limit = 2000, unversioned)] //This is not platform versioned automatically
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[serde(tag = "$formatVersion")]
pub enum IdentityPublicKey {
    #[serde(rename = "0")]
    V0(IdentityPublicKeyV0),
}

#[cfg(feature = "json-conversion")]
impl JsonConvertible for IdentityPublicKey {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use platform_value::{platform_value, BinaryData, Value};
    use serde_json::json;

    fn fixture() -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 9,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            read_only: true,
            data: BinaryData::new(vec![0x55; 20]),
            disabled_at: Some(1_700_000_000_000),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Internally-tagged enum (`tag = "$formatVersion"`); inner V0 has
        // `rename_all = "camelCase"`, plus the explicit `serde(rename = "type")`
        // on `key_type`. Sized-int fields with JSON loss:
        // - `id`: KeyID = u32 (erased to JSON Number)
        // - `key_type`/`purpose`/`security_level`: `#[repr(u8)]` enums with
        //   `Serialize_repr` -> raw u8 discriminants
        // - `disabled_at`: Option<u64>
        // `BinaryData` is base64-encoded in JSON (HR).
        // KeyType::ECDSA_HASH160 = 2, Purpose::TRANSFER = 3,
        // SecurityLevel::CRITICAL = 1.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "id": 9,
                "purpose": 3,
                "securityLevel": 1,
                "contractBounds": serde_json::Value::Null,
                "type": 2,
                "readOnly": true,
                "data": "VVVVVVVVVVVVVVVVVVVVVVVVVVU=",
                "disabledAt": 1_700_000_000_000u64,
            })
        );
        let recovered = IdentityPublicKey::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Explicit suffixes lock the typed-int variants:
        // - `9u32` -> `Value::U32` for the KeyID
        // - `4u8` / `1u8` -> `Value::U8` for the repr(u8) enums
        // - `1_700_000_000_000u64` -> `Value::U64` for the timestamp
        // `BinaryData` becomes a sized-bytes variant in non-HR — for a 20-byte
        // payload it is detected as `Value::Bytes20([u8; 20])`, not the generic
        // `Value::Bytes(Vec<u8>)`. (`platform_value` collapses fixed sizes
        // 20/32/36 to typed variants for round-trip stability.)
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "id": 9u32,
                "purpose": 3u8,
                "securityLevel": 1u8,
                "contractBounds": Value::Null,
                "type": 2u8,
                "readOnly": true,
                "data": Value::Bytes20([0x55; 20]),
                "disabledAt": 1_700_000_000_000u64,
            })
        );
        let recovered = IdentityPublicKey::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}

impl IdentityPublicKey {
    /// Checks if public key security level is MASTER
    pub fn is_master(&self) -> bool {
        self.security_level() == SecurityLevel::MASTER
    }

    /// Generates an identity public key with the maximum possible size based on the platform version.
    ///
    /// This method constructs a key of the largest possible size for the given platform version.
    /// This can be useful for stress testing or benchmarking purposes.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `platform_version`: The platform version which determines the structure of the identity key.
    ///
    /// # Returns
    ///
    /// * `Self`: An instance of the `IdentityPublicKey` struct.
    ///
    pub fn max_possible_size_key(
        id: KeyID,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => Ok(IdentityPublicKeyV0::max_possible_size_key(id).into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::max_possible_size_key".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => Ok(IdentityPublicKeyV0::default().into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::identity::identity_public_key::accessors::v0::{
        IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
    };
    use crate::identity::identity_public_key::contract_bounds::ContractBounds;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use platform_value::{BinaryData, Identifier};
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use rand::SeedableRng;

    #[test]
    fn test_identity_key_serialization_deserialization() {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let key: IdentityPublicKey =
            IdentityPublicKeyV0::random_ecdsa_master_authentication_key_with_rng(
                1,
                &mut rng,
                LATEST_PLATFORM_VERSION,
            )
            .expect("expected a random key")
            .0
            .into();
        let serialized = key.serialize_to_bytes().expect("expected to serialize key");
        let unserialized: IdentityPublicKey =
            PlatformDeserializable::deserialize_from_bytes(serialized.as_slice())
                .expect("expected to deserialize key");
        assert_eq!(key, unserialized)
    }

    // -- is_master --

    fn make_key_v0(security_level: SecurityLevel) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 42,
            purpose: Purpose::AUTHENTICATION,
            security_level,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            data: BinaryData::new(vec![0u8; 33]),
            read_only: false,
            disabled_at: None,
        })
    }

    #[test]
    fn test_is_master_true_when_security_level_master() {
        let key = make_key_v0(SecurityLevel::MASTER);
        assert!(key.is_master());
    }

    #[test]
    fn test_is_master_false_for_other_levels() {
        assert!(!make_key_v0(SecurityLevel::CRITICAL).is_master());
        assert!(!make_key_v0(SecurityLevel::HIGH).is_master());
        assert!(!make_key_v0(SecurityLevel::MEDIUM).is_master());
    }

    // -- max_possible_size_key --

    #[test]
    fn test_max_possible_size_key_v0_returns_bls_sized_data() {
        // BLS12_381 has the largest default size (48 bytes).
        let key = IdentityPublicKey::max_possible_size_key(123, LATEST_PLATFORM_VERSION)
            .expect("max_possible_size_key should work for v0");
        assert_eq!(key.id(), 123);
        assert_eq!(key.purpose(), Purpose::AUTHENTICATION);
        assert_eq!(key.security_level(), SecurityLevel::MASTER);
        assert!(!key.read_only());
        assert!(key.contract_bounds().is_none());
        assert!(key.disabled_at().is_none());
        // data should be default_size() bytes filled with 0xff.
        assert_eq!(key.data().len(), key.key_type().default_size());
        assert!(key.data().as_slice().iter().all(|b| *b == 0xff));
    }

    // -- default_versioned --

    #[test]
    fn test_default_versioned_returns_v0_default() {
        let key = IdentityPublicKey::default_versioned(LATEST_PLATFORM_VERSION)
            .expect("default_versioned should work for v0");
        // The V0 Default should have id=0 and default purpose/security_level/key_type.
        assert_eq!(key.id(), 0);
        assert_eq!(key.purpose(), Purpose::default());
        assert_eq!(key.security_level(), SecurityLevel::default());
        assert_eq!(key.key_type(), KeyType::default());
    }

    // -- accessors on the IdentityPublicKey enum wrapper --

    #[test]
    fn test_wrapper_getters_delegate_to_v0() {
        let inner = IdentityPublicKeyV0 {
            id: 7,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: Some(ContractBounds::SingleContract {
                id: Identifier::from([0x01u8; 32]),
            }),
            key_type: KeyType::BLS12_381,
            data: BinaryData::new(vec![2u8; 48]),
            read_only: true,
            disabled_at: Some(1_700_000_000_000),
        };
        let key = IdentityPublicKey::V0(inner.clone());
        assert_eq!(key.id(), 7);
        assert_eq!(key.purpose(), Purpose::TRANSFER);
        assert_eq!(key.security_level(), SecurityLevel::CRITICAL);
        assert_eq!(key.key_type(), KeyType::BLS12_381);
        assert!(key.read_only());
        assert_eq!(key.data().as_slice(), &vec![2u8; 48][..]);
        assert_eq!(key.disabled_at(), Some(1_700_000_000_000));
        assert!(key.is_disabled());
        assert!(key.contract_bounds().is_some());

        // data_owned returns the underlying data.
        let cloned = key.clone();
        assert_eq!(cloned.data_owned().as_slice(), &vec![2u8; 48][..]);
    }

    #[test]
    fn test_wrapper_setters_delegate_to_v0() {
        let mut key = make_key_v0(SecurityLevel::HIGH);
        key.set_id(99);
        key.set_purpose(Purpose::ENCRYPTION);
        key.set_security_level(SecurityLevel::CRITICAL);
        key.set_key_type(KeyType::BLS12_381);
        key.set_read_only(true);
        key.set_data(BinaryData::new(vec![0xABu8; 48]));
        key.set_disabled_at(1234);

        assert_eq!(key.id(), 99);
        assert_eq!(key.purpose(), Purpose::ENCRYPTION);
        assert_eq!(key.security_level(), SecurityLevel::CRITICAL);
        assert_eq!(key.key_type(), KeyType::BLS12_381);
        assert!(key.read_only());
        assert_eq!(key.data().as_slice(), &vec![0xABu8; 48][..]);
        assert_eq!(key.disabled_at(), Some(1234));
        assert!(key.is_disabled());

        key.remove_disabled_at();
        assert_eq!(key.disabled_at(), None);
        assert!(!key.is_disabled());
    }

    // -- From<IdentityPublicKeyV0> for IdentityPublicKey --
    #[test]
    fn test_from_v0_into_wrapper() {
        let v0 = IdentityPublicKeyV0 {
            id: 5,
            purpose: Purpose::VOTING,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            data: BinaryData::new(vec![0u8; 20]),
            read_only: false,
            disabled_at: None,
        };
        let wrapped: IdentityPublicKey = v0.clone().into();
        assert_eq!(wrapped.id(), 5);
        assert_eq!(wrapped.purpose(), Purpose::VOTING);
        assert_eq!(wrapped.security_level(), SecurityLevel::MEDIUM);
        // Equality on the enum variant.
        assert_eq!(wrapped, IdentityPublicKey::V0(v0));
    }

    // -- IdentityPublicKeyV0::default basic invariants --
    #[test]
    fn test_v0_default_fields() {
        let v0 = IdentityPublicKeyV0::default();
        assert_eq!(v0.id, 0);
        assert_eq!(v0.purpose, Purpose::default());
        assert_eq!(v0.security_level, SecurityLevel::default());
        assert_eq!(v0.key_type, KeyType::default());
        assert!(!v0.read_only);
        assert_eq!(v0.data.as_slice().len(), 0);
        assert!(v0.disabled_at.is_none());
        assert!(v0.contract_bounds.is_none());
    }

    // -- Ordering is derived: Clone + Eq + PartialOrd --
    #[test]
    fn test_keys_are_orderable_and_cloneable() {
        let a = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 1,
            ..Default::default()
        });
        let b = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 2,
            ..Default::default()
        });
        assert!(a < b);
        let a_clone = a.clone();
        assert_eq!(a, a_clone);
    }
}

#[cfg(all(test, feature = "random-public-keys"))]
mod random_tests {
    use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use crate::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use platform_version::version::LATEST_PLATFORM_VERSION;

    // -- random_key and random_keys with a seed are deterministic. --

    #[test]
    fn test_random_key_with_seed_is_deterministic() {
        let k1 = IdentityPublicKey::random_key(1, Some(42), LATEST_PLATFORM_VERSION);
        let k2 = IdentityPublicKey::random_key(1, Some(42), LATEST_PLATFORM_VERSION);
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_random_keys_returns_requested_count() {
        let keys = IdentityPublicKey::random_keys(0, 5, Some(7), LATEST_PLATFORM_VERSION);
        assert_eq!(keys.len(), 5);
        // Each should have a distinct id in [0, 5).
        let mut ids: Vec<_> = keys.iter().map(|k| k.id()).collect();
        ids.sort();
        assert_eq!(ids, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_random_authentication_key_is_always_authentication_purpose() {
        for id in 0..5u32 {
            let k = IdentityPublicKey::random_authentication_key(
                id,
                Some(id as u64),
                LATEST_PLATFORM_VERSION,
            );
            assert_eq!(k.purpose(), Purpose::AUTHENTICATION);
        }
    }

    #[test]
    fn test_random_authentication_keys_returns_requested_count_and_all_auth() {
        let keys =
            IdentityPublicKey::random_authentication_keys(0, 3, Some(11), LATEST_PLATFORM_VERSION);
        assert_eq!(keys.len(), 3);
        for k in &keys {
            assert_eq!(k.purpose(), Purpose::AUTHENTICATION);
        }
    }

    #[test]
    fn test_random_ecdsa_master_authentication_key_has_expected_attributes() {
        let (key, _priv) = IdentityPublicKey::random_ecdsa_master_authentication_key(
            0,
            Some(1),
            LATEST_PLATFORM_VERSION,
        )
        .expect("expected master authentication key");
        assert_eq!(key.key_type(), KeyType::ECDSA_SECP256K1);
        assert_eq!(key.purpose(), Purpose::AUTHENTICATION);
        assert_eq!(key.security_level(), SecurityLevel::MASTER);
        assert!(!key.read_only());
        assert_eq!(key.data().len(), 33);
    }

    #[test]
    fn test_random_ecdsa_critical_level_authentication_key_attributes() {
        let (key, _priv) = IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
            1,
            Some(2),
            LATEST_PLATFORM_VERSION,
        )
        .expect("expected critical auth key");
        assert_eq!(key.key_type(), KeyType::ECDSA_SECP256K1);
        assert_eq!(key.purpose(), Purpose::AUTHENTICATION);
        assert_eq!(key.security_level(), SecurityLevel::CRITICAL);
    }

    #[test]
    fn test_random_ecdsa_high_level_authentication_key_attributes() {
        let (key, _priv) = IdentityPublicKey::random_ecdsa_high_level_authentication_key(
            2,
            Some(3),
            LATEST_PLATFORM_VERSION,
        )
        .expect("expected high-level auth key");
        assert_eq!(key.key_type(), KeyType::ECDSA_SECP256K1);
        assert_eq!(key.purpose(), Purpose::AUTHENTICATION);
        assert_eq!(key.security_level(), SecurityLevel::HIGH);
    }

    #[test]
    fn test_random_masternode_owner_key_has_owner_purpose_and_is_read_only() {
        let (key, _priv) =
            IdentityPublicKey::random_masternode_owner_key(3, Some(4), LATEST_PLATFORM_VERSION)
                .expect("expected masternode owner key");
        assert_eq!(key.key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(key.purpose(), Purpose::OWNER);
        assert_eq!(key.security_level(), SecurityLevel::CRITICAL);
        assert!(key.read_only());
    }

    #[test]
    fn test_random_masternode_transfer_key_has_transfer_purpose_and_is_read_only() {
        let (key, _priv) =
            IdentityPublicKey::random_masternode_transfer_key(4, Some(5), LATEST_PLATFORM_VERSION)
                .expect("expected masternode transfer key");
        assert_eq!(key.key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(key.purpose(), Purpose::TRANSFER);
        assert_eq!(key.security_level(), SecurityLevel::CRITICAL);
        assert!(key.read_only());
    }

    #[test]
    fn test_main_keys_with_random_authentication_keys_errors_when_count_below_two() {
        use rand::{rngs::StdRng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(1);
        let err =
            IdentityPublicKey::main_keys_with_random_authentication_keys_with_private_keys_with_rng(
                1,
                &mut rng,
                LATEST_PLATFORM_VERSION,
            )
            .unwrap_err();
        match err {
            crate::ProtocolError::PublicKeyGenerationError(msg) => {
                assert!(msg.contains("at least 2"));
            }
            other => panic!("expected PublicKeyGenerationError, got {:?}", other),
        }
    }

    #[test]
    fn test_main_keys_with_random_authentication_keys_count_two() {
        use rand::{rngs::StdRng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(2);
        let keys = IdentityPublicKey::main_keys_with_random_authentication_keys_with_private_keys_with_rng(
            2,
            &mut rng,
            LATEST_PLATFORM_VERSION,
        )
        .expect("expected keys");
        assert_eq!(keys.len(), 2);
        // Position 0 must be Master ECDSA, position 1 must be High ECDSA.
        assert_eq!(keys[0].0.security_level(), SecurityLevel::MASTER);
        assert_eq!(keys[0].0.key_type(), KeyType::ECDSA_SECP256K1);
        assert_eq!(keys[1].0.security_level(), SecurityLevel::HIGH);
        assert_eq!(keys[1].0.key_type(), KeyType::ECDSA_SECP256K1);
    }

    #[test]
    fn test_main_keys_with_random_authentication_keys_count_three() {
        use rand::{rngs::StdRng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(3);
        let keys = IdentityPublicKey::main_keys_with_random_authentication_keys_with_private_keys_with_rng(
            3,
            &mut rng,
            LATEST_PLATFORM_VERSION,
        )
        .expect("expected keys");
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].0.security_level(), SecurityLevel::MASTER);
        assert_eq!(keys[1].0.security_level(), SecurityLevel::CRITICAL);
        assert_eq!(keys[2].0.security_level(), SecurityLevel::HIGH);
    }

    #[test]
    fn test_random_unique_keys_with_rng_matches_count() {
        use rand::{rngs::StdRng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(9);
        let keys =
            IdentityPublicKey::random_unique_keys_with_rng(4, &mut rng, LATEST_PLATFORM_VERSION)
                .expect("expected keys");
        assert_eq!(keys.len(), 4);
        // IDs should be 0..4.
        let ids: Vec<_> = keys.iter().map(|k| k.id()).collect();
        assert_eq!(ids, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_random_keys_with_rng_returns_requested_count() {
        use rand::{rngs::StdRng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(10);
        let keys = IdentityPublicKey::random_keys_with_rng(3, &mut rng, LATEST_PLATFORM_VERSION);
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn test_random_authentication_keys_with_private_keys_with_rng_returns_pairs() {
        use rand::{rngs::StdRng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(11);
        let pairs = IdentityPublicKey::random_authentication_keys_with_private_keys_with_rng(
            5,
            3,
            &mut rng,
            LATEST_PLATFORM_VERSION,
        )
        .expect("expected pairs");
        assert_eq!(pairs.len(), 3);
        let ids: Vec<_> = pairs.iter().map(|(k, _)| k.id()).collect();
        assert_eq!(ids, vec![5, 6, 7]);
        for (k, _) in &pairs {
            assert_eq!(k.purpose(), Purpose::AUTHENTICATION);
        }
    }

    #[test]
    fn test_random_voting_key_with_rng_attributes() {
        use rand::{rngs::StdRng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(13);
        let (k, _priv) =
            IdentityPublicKey::random_voting_key_with_rng(0, &mut rng, LATEST_PLATFORM_VERSION)
                .expect("expected voting key");
        assert_eq!(k.purpose(), Purpose::VOTING);
        assert_eq!(k.key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(k.security_level(), SecurityLevel::MEDIUM);
    }

    #[test]
    fn test_random_key_with_known_attributes_honors_inputs() {
        use crate::identity::contract_bounds::ContractBounds;
        use platform_value::Identifier;
        use rand::{rngs::StdRng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(17);
        let bounds = ContractBounds::SingleContract {
            id: Identifier::from([0x77u8; 32]),
        };
        let (k, _priv) = IdentityPublicKey::random_key_with_known_attributes(
            42,
            &mut rng,
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_SECP256K1,
            Some(bounds.clone()),
            LATEST_PLATFORM_VERSION,
        )
        .expect("expected key");
        assert_eq!(k.id(), 42);
        assert_eq!(k.purpose(), Purpose::TRANSFER);
        assert_eq!(k.security_level(), SecurityLevel::CRITICAL);
        assert_eq!(k.key_type(), KeyType::ECDSA_SECP256K1);
        assert_eq!(k.contract_bounds(), Some(&bounds));
    }
}
