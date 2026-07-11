use crate::identity::{Identity, IdentityV0};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::TryFromPlatformVersioned;

impl TryFromPlatformVersioned<Value> for Identity {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => {
                let identity_v0: IdentityV0 =
                    platform_value::from_value(value).map_err(ProtocolError::ValueError)?;
                Ok(identity_v0.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::try_from_owned_value".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl TryFromPlatformVersioned<&Value> for Identity {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => {
                let identity_v0: IdentityV0 =
                    platform_value::from_value(value.clone()).map_err(ProtocolError::ValueError)?;
                Ok(identity_v0.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::try_from_owned_value".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::accessors::IdentityGettersV0;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::IdentityPublicKey;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::serialization::ValueConvertible;
    use platform_value::{platform_value, BinaryData, Identifier};
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use std::collections::BTreeMap;

    fn sample_identity_v0() -> IdentityV0 {
        let mut keys: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
        keys.insert(
            0,
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: 0,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MASTER,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(vec![0x01; 33]),
                disabled_at: None,
            }),
        );
        IdentityV0 {
            id: Identifier::from([42u8; 32]),
            public_keys: keys,
            balance: 7,
            revision: 2,
        }
    }

    // A `platform_value::Value` that `try_from_platform_versioned` accepts and
    // deserializes into an `IdentityV0` via `platform_value::from_value::<IdentityV0>`.
    //
    // NOTE: this is *not* a byte-for-byte mirror of `IdentityV0::to_object()`.
    // `to_object()` produces `Value::Bytes` for `BinaryData` fields (e.g. the
    // public-key `data`), while this fixture encodes `data` as a base64 STRING.
    // Both shapes round-trip through the serde deserializer because the inner
    // `platform_value` deserializer behaves as `is_human_readable() = true` for
    // nested fields, which accepts the base64-string representation of
    // `BinaryData`. This fixture deliberately exercises the human-readable path.
    //
    // Each inner public key carries the adjacency-tag `$formatVersion: "0"` that
    // `IdentityPublicKey`'s serde enum representation requires.
    //
    // frozen: V0 consensus behavior.
    fn tagged_raw_value() -> Value {
        use platform_value::string_encoding::{encode, Encoding};
        let data_b64 = encode(&[0x22u8; 33], Encoding::Base64);
        platform_value!({
            "id": Identifier::from([7u8; 32]),
            "publicKeys": [
                {
                    "$formatVersion": "0",
                    "id": 0u32,
                    "type": 0u8,
                    "purpose": 0u8,
                    "securityLevel": 0u8,
                    "contractBounds": Value::Null,
                    "data": data_b64,
                    "readOnly": false,
                    "disabledAt": Value::Null,
                }
            ],
            "balance": 100u64,
            "revision": 1u64,
        })
    }

    #[test]
    fn try_from_platform_versioned_owned_value_parses_legacy_shape() {
        let value = tagged_raw_value();
        let identity = Identity::try_from_platform_versioned(value, LATEST_PLATFORM_VERSION)
            .expect("should parse legacy raw object");
        assert_eq!(identity.balance(), 100);
        assert_eq!(identity.revision(), 1);
        assert_eq!(identity.public_keys().len(), 1);
    }

    #[test]
    fn try_from_platform_versioned_ref_value_parses_legacy_shape() {
        let value = tagged_raw_value();
        let identity = Identity::try_from_platform_versioned(&value, LATEST_PLATFORM_VERSION)
            .expect("should parse legacy raw object from &Value");
        assert_eq!(identity.balance(), 100);
    }

    #[test]
    fn try_from_platform_versioned_errors_on_garbage_owned() {
        let value = Value::Null;
        let result = Identity::try_from_platform_versioned(value, LATEST_PLATFORM_VERSION);
        assert!(matches!(result, Err(ProtocolError::ValueError(_))));
    }

    #[test]
    fn try_from_platform_versioned_errors_on_garbage_ref() {
        let value = Value::Text("not a map".to_string());
        let result = Identity::try_from_platform_versioned(&value, LATEST_PLATFORM_VERSION);
        assert!(matches!(result, Err(ProtocolError::ValueError(_))));
    }

    // After Phase D step 5, `IdentityPlatformValueConversionMethodsV0` has
    // been deleted; the canonical `ValueConvertible::to_object` produces the
    // tagged `$formatVersion: "0"` form for the Identity enum wrapper.
    #[test]
    fn identity_wrapper_to_object_includes_format_version_tag() {
        let identity: Identity = sample_identity_v0().into();
        let value = identity.to_object().expect("to_object");
        let map = value.to_map_ref().expect("map");
        assert!(
            map.iter()
                .any(|(k, _)| k.as_text() == Some("$formatVersion")),
            "Identity enum wrapper must keep its format version tag"
        );
    }

    #[test]
    fn identity_wrapper_to_object_differs_from_v0_inner_shape() {
        // Sanity check: the Identity wrapper's to_object includes `$formatVersion`,
        // whereas IdentityV0's own to_object is a flat map.
        let v0 = sample_identity_v0();
        let wrapper: Identity = v0.clone().into();
        let inner_value = v0.to_object().unwrap();
        let outer_value = wrapper.to_object().unwrap();
        assert_ne!(inner_value, outer_value);
    }
}
