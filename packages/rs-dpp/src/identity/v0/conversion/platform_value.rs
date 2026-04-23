use crate::identity::conversion::platform_value::IdentityPlatformValueConversionMethodsV0;
use crate::identity::{property_names, IdentityV0};
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use platform_value::Value;

impl IdentityPlatformValueConversionMethodsV0 for IdentityV0 {
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        //same as object for Identities
        let mut value = self.to_object()?;
        if let Some(keys) = value.get_optional_array_mut_ref(property_names::PUBLIC_KEYS)? {
            for key in keys.iter_mut() {
                key.remove_optional_value_if_null("disabledAt")?;
            }
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use platform_value::{BinaryData, Identifier};
    use std::collections::BTreeMap;

    fn sample_with_disabled(disabled_at: Option<u64>) -> IdentityV0 {
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
                data: BinaryData::new(vec![0x11; 33]),
                disabled_at,
            }),
        );
        IdentityV0 {
            id: Identifier::from([0u8; 32]),
            public_keys: keys,
            balance: 0,
            revision: 0,
        }
    }

    fn key_map_at_index(value: &Value, index: usize) -> &Vec<(Value, Value)> {
        let map = value.to_map_ref().expect("map");
        let pks = map
            .iter()
            .find(|(k, _)| k.as_text() == Some("publicKeys"))
            .map(|(_, v)| v)
            .expect("publicKeys");
        let arr = pks.to_array_ref().expect("array");
        arr[index].to_map_ref().expect("key map")
    }

    #[test]
    fn to_cleaned_object_strips_null_disabled_at_from_keys() {
        let id = sample_with_disabled(None);
        let cleaned = id.to_cleaned_object().expect("cleaned");
        let key_map = key_map_at_index(&cleaned, 0);
        assert!(
            !key_map
                .iter()
                .any(|(k, _)| k.as_text() == Some("disabledAt")),
            "disabledAt should have been stripped"
        );
    }

    #[test]
    fn to_cleaned_object_preserves_present_disabled_at() {
        let id = sample_with_disabled(Some(123));
        let cleaned = id.to_cleaned_object().expect("cleaned");
        let key_map = key_map_at_index(&cleaned, 0);
        assert!(key_map
            .iter()
            .any(|(k, _)| k.as_text() == Some("disabledAt")));
    }

    #[test]
    fn to_object_and_cleaned_are_same_for_empty_keys() {
        let id = IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 1,
            revision: 2,
        };
        let object = id.to_object().expect("to_object");
        let cleaned = id.to_cleaned_object().expect("cleaned");
        assert_eq!(object, cleaned);
    }

    // frozen: V0 consensus behavior
    //
    // `IdentityV0::to_object()` (from the `ValueConvertible` derive) serializes through
    // platform_value's non-human-readable path and encodes `BinaryData` as `Value::Bytes`.
    // But `platform_value::from_value(...)` produces inner deserializers that default to
    // `is_human_readable() = true`, so `BinaryData::deserialize` dispatches to its string
    // visitor and fails on `Value::Bytes`. The direct round-trip therefore does NOT work;
    // consumers must go through the explicit conversion helpers (JSON path, etc.).
    #[test]
    fn to_object_then_try_from_fails_v0_frozen() {
        let id = sample_with_disabled(Some(9));
        let value = id.to_object().unwrap();
        let result = IdentityV0::try_from(value);
        assert!(
            result.is_err(),
            "V0 to_object -> TryFrom<Value> round-trip is not expected to succeed"
        );
    }

    #[test]
    fn try_from_ref_value_fails_on_to_object_output_v0_frozen() {
        let id = sample_with_disabled(None);
        let value = id.to_object().unwrap();
        let result = IdentityV0::try_from(&value);
        assert!(result.is_err());
    }
}
