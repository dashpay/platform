// `IdentityPublicKey` value-side conversion now goes exclusively through
// the canonical `ValueConvertible` trait (derived on the outer enum). The
// legacy `IdentityPublicKeyPlatformValueConversionMethodsV0` trait has
// been deleted: it carried `to_object` / `into_object` that were
// byte-identical to canonical, plus a `from_object(value, &platform_version)`
// version-dispatch method that produced identical output to canonical for
// the only currently-defined V0 (canonical dispatches on the value's own
// `$formatVersion` tag, which all V0 values carry).

#[cfg(test)]
mod tests {
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use crate::serialization::ValueConvertible;
    use crate::ProtocolError;
    use platform_value::{BinaryData, Value};

    fn wrapper(disabled_at: Option<u64>) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 9,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0x55; 33]),
            disabled_at,
        })
    }

    #[test]
    fn to_object_includes_format_version_tag() {
        // The outer `IdentityPublicKey` is a tagged enum
        // (`#[serde(tag = "$formatVersion")]`); canonical `to_object`
        // emits `$formatVersion: "0"` next to the V0 fields.
        let key = wrapper(Some(5));
        let value = key.to_object().expect("to_object");
        let map = value.to_map().expect("map");
        assert!(
            map.iter().any(
                |(k, v): &(Value, Value)| k.as_text() == Some("$formatVersion")
                    && v.as_text() == Some("0")
            ),
            "outer enum must surface the $formatVersion tag"
        );
    }

    #[test]
    fn to_object_strips_disabled_at_when_none() {
        // The `skip_serializing_if` attribute on
        // `IdentityPublicKeyV0::disabled_at` strips the field for
        // non-disabled keys directly via the canonical `to_object` path.
        let key = wrapper(None);
        let value = key.to_object().expect("to_object");
        let map = value.to_map().expect("map");
        assert!(!map
            .iter()
            .any(|(k, _): &(Value, Value)| k.as_text() == Some("disabledAt")));
    }

    #[test]
    fn into_object_is_same_as_to_object() {
        let key = wrapper(Some(7));
        let via_ref = key.to_object().unwrap();
        let via_owned = key.into_object().unwrap();
        assert_eq!(via_ref, via_owned);
    }

    #[test]
    fn from_object_roundtrip_via_wrapper() {
        let key = wrapper(None);
        let value = key.to_object().unwrap();
        // Canonical `ValueConvertible::from_object` dispatches on the
        // value's `$formatVersion` tag.
        let back = IdentityPublicKey::from_object(value).unwrap();
        assert_eq!(back, key);
    }

    #[test]
    fn from_object_fails_on_non_map() {
        let result = IdentityPublicKey::from_object(Value::Null);
        assert!(matches!(result, Err(ProtocolError::ValueError(_))));
    }
}
