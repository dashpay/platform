mod v0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::identity::IdentityPublicKey;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
pub use v0::*;

impl IdentityPublicKeyPlatformValueConversionMethodsV0 for IdentityPublicKey {
    fn to_object(&self) -> Result<Value, ProtocolError> {
        match self {
            IdentityPublicKey::V0(key) => key.to_object(),
        }
    }

    fn into_object(self) -> Result<Value, ProtocolError> {
        match self {
            IdentityPublicKey::V0(key) => key.into_object(),
        }
    }

    fn from_object(
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => IdentityPublicKeyV0::from_object(value, platform_version).map(Into::into),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::from_object".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use platform_value::BinaryData;
    use platform_version::version::LATEST_PLATFORM_VERSION;

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
    fn to_object_delegates_to_v0() {
        let key = wrapper(Some(5));
        let value = key.to_object().expect("to_object");
        // Should match what V0 produces directly.
        let IdentityPublicKey::V0(inner) = &key;
        assert_eq!(value, inner.to_object().unwrap());
    }

    #[test]
    fn to_object_strips_disabled_at_when_none() {
        // After Phase D step 4, the `skip_serializing_if` attribute on
        // `IdentityPublicKeyV0::disabled_at` strips the field for
        // non-disabled keys directly via the canonical `to_object` path.
        // The dedicated `to_cleaned_object` method has been deleted.
        let key = wrapper(None);
        let value = key.to_object().expect("to_object");
        let map = value.to_map().expect("map");
        assert!(!map.iter().any(|(k, _)| k.as_text() == Some("disabledAt")));
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
        let back = IdentityPublicKey::from_object(value, LATEST_PLATFORM_VERSION).unwrap();
        assert_eq!(back, key);
    }

    #[test]
    fn from_object_fails_on_non_map() {
        let result = IdentityPublicKey::from_object(Value::Null, LATEST_PLATFORM_VERSION);
        assert!(matches!(result, Err(ProtocolError::ValueError(_))));
    }
}
