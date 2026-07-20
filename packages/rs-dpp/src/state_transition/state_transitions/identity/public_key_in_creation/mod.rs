use crate::identity::IdentityPublicKey;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0Signable;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::PlatformSignable;

use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod accessors;
mod fields;
mod methods;
mod types;
pub mod v0;
mod version;

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(Debug, Encode, Decode, PlatformSignable, Clone, PartialEq, Eq, From)]
//here we want to indicate that IdentityPublicKeyInCreation can be transformed into IdentityPublicKeyInCreationSignable
#[platform_signable(derive_into)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
pub enum IdentityPublicKeyInCreation {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityPublicKeyInCreationV0),
}

impl IdentityPublicKeyInCreation {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => Ok(IdentityPublicKeyInCreationV0::default().into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKeyInCreation::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl From<&IdentityPublicKeyInCreation> for IdentityPublicKey {
    fn from(val: &IdentityPublicKeyInCreation) -> Self {
        match val {
            IdentityPublicKeyInCreation::V0(v0) => v0.into(),
        }
    }
}

impl From<IdentityPublicKeyInCreation> for IdentityPublicKey {
    fn from(val: IdentityPublicKeyInCreation) -> Self {
        match val {
            IdentityPublicKeyInCreation::V0(v0) => v0.into(),
        }
    }
}

impl From<IdentityPublicKey> for IdentityPublicKeyInCreation {
    fn from(val: IdentityPublicKey) -> Self {
        match val {
            IdentityPublicKey::V0(_) => {
                let v0: IdentityPublicKeyInCreationV0 = val.into();
                v0.into()
            }
        }
    }
}

impl From<&IdentityPublicKey> for IdentityPublicKeyInCreation {
    fn from(val: &IdentityPublicKey) -> Self {
        match val {
            IdentityPublicKey::V0(_) => {
                let v0: IdentityPublicKeyInCreationV0 = val.into();
                v0.into()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
    use crate::state_transition::public_key_in_creation::methods::IdentityPublicKeyInCreationMethodsV0;
    use crate::version::LATEST_PLATFORM_VERSION;
    use platform_value::BinaryData;

    fn make_master_key(id: u16) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: id.into(),
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            signature: BinaryData::new(vec![0u8; 65]),
        })
    }

    fn make_high_key(id: u16) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: id.into(),
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![id as u8; 33]),
            signature: BinaryData::new(vec![]),
        })
    }

    fn make_critical_transfer_key(id: u16) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: id.into(),
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![id as u8; 33]),
            signature: BinaryData::new(vec![]),
        })
    }

    #[test]
    fn test_default_versioned() {
        let key = IdentityPublicKeyInCreation::default_versioned(LATEST_PLATFORM_VERSION)
            .expect("should create default");
        match key {
            IdentityPublicKeyInCreation::V0(_) => {}
        }
    }

    #[test]
    fn test_from_into_identity_public_key() {
        let key = make_master_key(0);
        let pk: IdentityPublicKey = key.clone().into();
        let back: IdentityPublicKeyInCreation = pk.into();
        assert_eq!(back.id(), key.id());
        assert_eq!(back.purpose(), key.purpose());
    }

    #[test]
    fn test_from_ref_into_identity_public_key() {
        let key = make_master_key(0);
        let pk: IdentityPublicKey = (&key).into();
        assert_eq!(pk.id(), key.id());
    }

    #[test]
    fn test_from_identity_public_key_ref() {
        let key = make_master_key(0);
        let pk: IdentityPublicKey = key.clone().into();
        let back: IdentityPublicKeyInCreation = (&pk).into();
        assert_eq!(back.id(), key.id());
    }

    #[test]
    fn test_into_identity_public_key_method() {
        let key = make_master_key(0);
        let pk = key.clone().into_identity_public_key();
        assert_eq!(pk.id(), key.id());
    }

    #[test]
    fn test_validate_structure_valid_create() {
        let keys = vec![make_master_key(0), make_high_key(1)];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &keys,
            true,
            LATEST_PLATFORM_VERSION,
        )
        .expect("validation should not error");
        assert!(
            result.is_valid(),
            "valid keys should pass: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_structure_missing_master_in_create() {
        let keys = vec![make_high_key(0)];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &keys,
            true,
            LATEST_PLATFORM_VERSION,
        )
        .expect("validation should not error");
        assert!(!result.is_valid(), "should fail without master key");
    }

    #[test]
    fn test_validate_structure_too_many_master_in_create() {
        let keys = vec![make_master_key(0), make_master_key(1)];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &keys,
            true,
            LATEST_PLATFORM_VERSION,
        )
        .expect("validation should not error");
        // Keys have same data, will be caught as duplicate data
        assert!(
            !result.is_valid(),
            "should fail with duplicated master keys"
        );
    }

    #[test]
    fn test_validate_structure_duplicate_key_ids() {
        // Two keys with the same id
        let keys = vec![make_master_key(0), make_high_key(0)];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &keys,
            true,
            LATEST_PLATFORM_VERSION,
        )
        .expect("validation should not error");
        assert!(!result.is_valid(), "should fail with duplicate key ids");
    }

    #[test]
    fn test_validate_structure_duplicate_key_data() {
        // Two keys with the same data but different ids
        let key1 = make_master_key(0);
        let key2_inner = IdentityPublicKeyInCreationV0 {
            id: 1,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]), // same data as key1
            signature: BinaryData::new(vec![]),
        };
        let key2 = IdentityPublicKeyInCreation::V0(key2_inner);
        let keys = vec![key1, key2];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &keys,
            true,
            LATEST_PLATFORM_VERSION,
        )
        .expect("validation should not error");
        assert!(!result.is_valid(), "should fail with duplicate key data");
    }

    #[test]
    fn test_validate_structure_not_in_create_no_master_required() {
        // When not in create, master key is not required
        let keys = vec![make_high_key(0)];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &keys,
            false,
            LATEST_PLATFORM_VERSION,
        )
        .expect("validation should not error");
        assert!(
            result.is_valid(),
            "should pass without master key when not in create: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_structure_transfer_key_valid() {
        let keys = vec![make_master_key(0), make_critical_transfer_key(1)];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &keys,
            true,
            LATEST_PLATFORM_VERSION,
        )
        .expect("validation should not error");
        assert!(result.is_valid(), "should pass: {:?}", result.errors);
    }

    #[test]
    fn test_validate_structure_invalid_security_level_for_purpose() {
        // Transfer keys must be CRITICAL security level
        let bad_key = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 1,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::HIGH, // invalid for TRANSFER
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![1u8; 33]),
            signature: BinaryData::new(vec![]),
        });
        let keys = vec![make_master_key(0), bad_key];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &keys,
            true,
            LATEST_PLATFORM_VERSION,
        )
        .expect("validation should not error");
        assert!(
            !result.is_valid(),
            "should fail with invalid security level for purpose"
        );
    }

    #[test]
    fn test_duplicated_key_ids_witness() {
        let keys = vec![make_master_key(0), make_high_key(0)];
        let dups =
            IdentityPublicKeyInCreation::duplicated_key_ids_witness(&keys, LATEST_PLATFORM_VERSION)
                .expect("should work");
        assert_eq!(dups.len(), 1);
    }

    #[test]
    fn test_duplicated_key_ids_witness_no_dups() {
        let keys = vec![make_master_key(0), make_high_key(1)];
        let dups =
            IdentityPublicKeyInCreation::duplicated_key_ids_witness(&keys, LATEST_PLATFORM_VERSION)
                .expect("should work");
        assert!(dups.is_empty());
    }

    #[test]
    fn test_duplicated_keys_witness() {
        // Keys with same data
        let key1 = make_master_key(0);
        let key2 = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 1,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            signature: BinaryData::new(vec![]),
        });
        let keys = vec![key1, key2];
        let dups =
            IdentityPublicKeyInCreation::duplicated_keys_witness(&keys, LATEST_PLATFORM_VERSION)
                .expect("should work");
        assert_eq!(dups.len(), 1);
    }

    #[test]
    fn test_hash_with_hash160_key() {
        // ECDSA_HASH160 keys don't need valid secp256k1 data for hashing
        let key = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 20]),
            signature: BinaryData::new(vec![]),
        });
        let hash = key.hash().expect("should hash");
        assert_eq!(hash.len(), 20);
    }

    // Legacy `StateTransitionValueConvert` round-trip / canonical /
    // unknown-version tests deleted in Phase D step 9. The canonical
    // `JsonConvertible` / `ValueConvertible` round-trip is exercised on
    // the outer enum derive (see `json_convertible_tests` below) — these
    // tested methods that no longer exist.

    #[test]
    fn test_default_versioned_unknown() {
        use platform_version::version::PlatformVersion;
        // Use a fresh version with an unknown identity_key_structure_version.
        let mut pv = PlatformVersion::latest().clone();
        pv.dpp.identity_versions.identity_key_structure_version = 255;
        let result = IdentityPublicKeyInCreation::default_versioned(&pv);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_duplicate_in_keys_witness() {
        let key1 = make_master_key(0);
        // Identical data to key1 but different id
        let key2 = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 1,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            signature: BinaryData::new(vec![]),
        });
        let keys = vec![key1, key2];
        let dup_ids =
            IdentityPublicKeyInCreation::duplicated_keys_witness(&keys, LATEST_PLATFORM_VERSION)
                .expect("witness");
        assert_eq!(dup_ids.len(), 1, "one duplicate expected");
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;

    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use platform_value::{platform_value, BinaryData};
    use serde_json::json;

    fn fixture() -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 7,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: true,
            data: BinaryData::new(vec![0x88; 33]),
            signature: BinaryData::new(vec![0x99; 65]),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields whose JSON wire encoding loses size info:
        // `id` (u32 KeyID), `type`/`purpose`/`securityLevel` (u8 repr enums).
        // The value-path assertion below uses explicit suffixes for size lock-in.
        // Note `key_type` is renamed to `"type"` in the wire shape.
        // `securityLevel` = 2 because `SecurityLevel::HIGH as u8 == 2`.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "id": 7,
                "type": 0,
                "purpose": 0,
                "securityLevel": 2,
                "contractBounds": serde_json::Value::Null,
                "readOnly": true,
                "data": "iIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI",
                "signature": "mZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZk=",
            })
        );
        let recovered = IdentityPublicKeyInCreation::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Explicit suffixes lock in sized variants: `id` u32 (KeyID),
        // `type`/`purpose`/`securityLevel` u8 (#[repr(u8)] enums).
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "id": 7u32,
                "type": 0u8,
                "purpose": 0u8,
                "securityLevel": 2u8,
                "contractBounds": platform_value::Value::Null,
                "readOnly": true,
                "data": BinaryData::new(vec![0x88; 33]),
                "signature": BinaryData::new(vec![0x99; 65]),
            })
        );
        let recovered = IdentityPublicKeyInCreation::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
