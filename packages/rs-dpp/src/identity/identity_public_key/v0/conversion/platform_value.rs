use crate::identity::identity_public_key::conversion::platform_value::IdentityPublicKeyPlatformValueConversionMethodsV0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use std::convert::{TryFrom, TryInto};

impl IdentityPublicKeyPlatformValueConversionMethodsV0 for IdentityPublicKeyV0 {
    fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn into_object(self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn from_object(
        value: Value,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        value.try_into().map_err(ProtocolError::ValueError)
    }
}

impl TryFrom<&IdentityPublicKeyV0> for Value {
    type Error = platform_value::Error;

    fn try_from(value: &IdentityPublicKeyV0) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

impl TryFrom<IdentityPublicKeyV0> for Value {
    type Error = platform_value::Error;

    fn try_from(value: IdentityPublicKeyV0) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

impl TryFrom<Value> for IdentityPublicKeyV0 {
    type Error = platform_value::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::contract_bounds::ContractBounds;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use platform_value::{BinaryData, Identifier};
    use platform_version::version::LATEST_PLATFORM_VERSION;

    fn sample_v0(disabled_at: Option<u64>) -> IdentityPublicKeyV0 {
        IdentityPublicKeyV0 {
            id: 3,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0xAB; 33]),
            disabled_at,
        }
    }

    #[test]
    fn to_object_roundtrip_to_v0() {
        let key = sample_v0(Some(1_700_000_000_000));
        let value = key.to_object().expect("to_object should succeed");
        // The inner serializes its fields with camelCase (no $formatVersion tag).
        assert!(value.is_map());
        let roundtripped =
            IdentityPublicKeyV0::from_object(value, LATEST_PLATFORM_VERSION).expect("from_object");
        assert_eq!(roundtripped, key);
    }

    // After Phase D step 4, `to_object` itself strips `disabledAt: null`
    // for non-disabled keys via `#[serde(skip_serializing_if = "Option::is_none")]`.
    // The dedicated `to_cleaned_object` method has been deleted.
    #[test]
    fn to_object_removes_disabled_at_when_none() {
        let key = sample_v0(None);
        let value = key.to_object().expect("to_object");
        let map = value.to_map().expect("expected a map");
        assert!(!map.iter().any(|(k, _)| k.as_text() == Some("disabledAt")));
    }

    #[test]
    fn to_object_keeps_disabled_at_when_some() {
        let key = sample_v0(Some(42));
        let value = key.to_object().expect("to_object");
        let map = value.to_map().expect("expected a map");
        assert!(map.iter().any(|(k, _)| k.as_text() == Some("disabledAt")));
    }

    #[test]
    fn into_object_produces_same_as_to_object() {
        let key = sample_v0(Some(1));
        let via_ref = key.to_object().unwrap();
        let via_owned = key.clone().into_object().unwrap();
        assert_eq!(via_ref, via_owned);
    }

    #[test]
    fn from_object_rejects_non_map() {
        // platform_value deserialization should fail on a bare string.
        let value = Value::Text("not a key".to_string());
        let result = IdentityPublicKeyV0::from_object(value, LATEST_PLATFORM_VERSION);
        assert!(matches!(result, Err(ProtocolError::ValueError(_))));
    }

    #[test]
    fn try_from_owned_value_succeeds() {
        let key = sample_v0(None);
        let value: Value = platform_value::to_value(&key).unwrap();
        let from: IdentityPublicKeyV0 = value.try_into().unwrap();
        assert_eq!(from, key);
    }

    #[test]
    fn try_from_ref_public_key_into_value_succeeds() {
        let key = sample_v0(None);
        let value: Value = (&key).try_into().expect("try_from &IdentityPublicKeyV0");
        assert!(value.is_map());
    }

    #[test]
    fn try_from_owned_public_key_into_value_succeeds() {
        let key = sample_v0(None);
        let value: Value = key
            .clone()
            .try_into()
            .expect("try_from IdentityPublicKeyV0");
        // Round-trip back.
        let back: IdentityPublicKeyV0 = value.try_into().unwrap();
        assert_eq!(back, key);
    }

    #[test]
    fn from_object_with_contract_bounds_roundtrip() {
        let mut key = sample_v0(None);
        key.contract_bounds = Some(ContractBounds::SingleContract {
            id: Identifier::from([0x12u8; 32]),
        });
        let value = key.to_object().unwrap();
        let back = IdentityPublicKeyV0::from_object(value, LATEST_PLATFORM_VERSION).unwrap();
        assert_eq!(back, key);
    }
}
