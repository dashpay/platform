mod v0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::identity::IdentityPublicKey;
use crate::ProtocolError;
use serde_json::Value as JsonValue;
pub use v0::IdentityPublicKeyJsonConversionMethodsV0;

impl IdentityPublicKeyJsonConversionMethodsV0 for IdentityPublicKey {
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            IdentityPublicKey::V0(key) => key.to_json(),
        }
    }

    fn to_json_object(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            IdentityPublicKey::V0(key) => key.to_json_object(),
        }
    }

    fn from_json_object(raw_object: JsonValue) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        // V0 is currently the only structure version. The inner enum is
        // tagged with `$formatVersion`, so the value's own tag drives
        // dispatch via serde — no platform_version arg needed.
        IdentityPublicKeyV0::from_json_object(raw_object).map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use platform_value::BinaryData;

    fn wrapper(disabled_at: Option<u64>) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 2,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0x77; 33]),
            disabled_at,
        })
    }

    #[test]
    fn to_json_delegates_to_v0() {
        let key = wrapper(None);
        let json = key.to_json().expect("to_json");
        let IdentityPublicKey::V0(inner) = &key;
        assert_eq!(json, inner.to_json().unwrap());
    }

    #[test]
    fn to_json_object_delegates_to_v0() {
        let key = wrapper(None);
        let json = key.to_json_object().expect("to_json_object");
        let IdentityPublicKey::V0(inner) = &key;
        assert_eq!(json, inner.to_json_object().unwrap());
    }

    #[test]
    fn from_json_object_roundtrip() {
        let key = wrapper(Some(1234));
        let json = key.to_json().expect("to_json");
        let back = IdentityPublicKey::from_json_object(json).unwrap();
        assert_eq!(back, key);
    }

    #[test]
    fn from_json_object_missing_fields_errors() {
        let json = serde_json::json!({ "id": 0 });
        let result = IdentityPublicKey::from_json_object(json);
        assert!(result.is_err());
    }
}
