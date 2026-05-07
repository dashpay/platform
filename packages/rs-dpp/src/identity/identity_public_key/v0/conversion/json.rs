use crate::identity::identity_public_key::conversion::json::IdentityPublicKeyJsonConversionMethodsV0;
use crate::identity::identity_public_key::fields::BINARY_DATA_FIELDS;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{ReplacementType, Value};
use serde_json::Value as JsonValue;
use std::convert::{TryFrom, TryInto};

impl IdentityPublicKeyJsonConversionMethodsV0 for IdentityPublicKeyV0 {
    fn to_json_object(&self) -> Result<JsonValue, ProtocolError> {
        // Inline `platform_value::to_value` (the body of the deleted
        // legacy `to_object` method). `IdentityPublicKeyV0` derives
        // `Serialize`, and after Phase D step 4 (`skip_serializing_if`
        // on `disabled_at`) the output is already cleaned of
        // `disabledAt: null` for non-disabled keys.
        let value = platform_value::to_value(self).map_err(ProtocolError::ValueError)?;
        value
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let value = platform_value::to_value(self).map_err(ProtocolError::ValueError)?;
        value.try_into().map_err(ProtocolError::ValueError)
    }

    fn from_json_object(
        raw_object: JsonValue,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut value: Value = raw_object.into();
        // Legacy ingest: rewrite binary fields from byte-array form to
        // `Value::Bytes` before deserializing. Older JS clients sometimes
        // emit data fields as JSON arrays of u8 (the validating-JSON shape
        // produced by `to_json_object`) rather than canonical base64.
        value.replace_at_paths(BINARY_DATA_FIELDS, ReplacementType::BinaryBytes)?;
        platform_value::from_value(value).map_err(ProtocolError::ValueError)
    }
}

impl TryFrom<&str> for IdentityPublicKeyV0 {
    type Error = ProtocolError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut platform_value: Value = serde_json::from_str::<JsonValue>(value)
            .map_err(|e| ProtocolError::StringDecodeError(e.to_string()))?
            .into();
        platform_value.replace_at_paths(BINARY_DATA_FIELDS, ReplacementType::BinaryBytes)?;
        platform_value::from_value(platform_value).map_err(ProtocolError::ValueError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use platform_value::BinaryData;
    use platform_version::version::LATEST_PLATFORM_VERSION;

    fn sample_v0(disabled_at: Option<u64>) -> IdentityPublicKeyV0 {
        IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0x01; 33]),
            disabled_at,
        }
    }

    #[test]
    fn to_json_returns_object() {
        let key = sample_v0(None);
        let json = key.to_json().expect("to_json succeeds");
        let obj = json.as_object().expect("expected json object");
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("type"));
        assert!(obj.contains_key("purpose"));
        assert!(obj.contains_key("securityLevel"));
        assert!(obj.contains_key("readOnly"));
        assert!(obj.contains_key("data"));
        // disabledAt is absent because it was None (to_cleaned_object removes it).
        assert!(!obj.contains_key("disabledAt"));
    }

    #[test]
    fn to_json_includes_disabled_at_when_some() {
        let key = sample_v0(Some(1_000_000));
        let json = key.to_json().expect("to_json succeeds");
        let obj = json.as_object().expect("object");
        assert!(obj.contains_key("disabledAt"));
    }

    #[test]
    fn to_json_object_data_is_byte_array() {
        // to_json_object goes through try_into_validating_json, which renders bytes as arrays.
        let key = sample_v0(None);
        let json = key.to_json_object().expect("to_json_object succeeds");
        let obj = json.as_object().expect("object");
        let data = obj.get("data").expect("data field").as_array().expect(
            "to_json_object should encode binary data as a JSON array of bytes (validating form)",
        );
        assert_eq!(data.len(), 33);
    }

    #[test]
    fn from_json_object_roundtrip() {
        let key = sample_v0(None);
        let json = key.to_json().unwrap();
        let back = IdentityPublicKeyV0::from_json_object(json, LATEST_PLATFORM_VERSION).unwrap();
        assert_eq!(back, key);
    }

    #[test]
    fn try_from_str_parses_canonical_json() {
        // Data is base64 of 33 0xAB bytes.
        let bytes = [0xABu8; 33];
        let b64 = platform_value::string_encoding::encode(
            &bytes,
            platform_value::string_encoding::Encoding::Base64,
        );
        let s = format!(
            r#"{{
                "id": 7,
                "type": 0,
                "purpose": 0,
                "securityLevel": 0,
                "readOnly": false,
                "data": "{}"
            }}"#,
            b64
        );
        let key: IdentityPublicKeyV0 = s.as_str().try_into().expect("parse succeeds");
        assert_eq!(key.id, 7);
        assert_eq!(key.data.as_slice(), &vec![0xABu8; 33][..]);
    }

    #[test]
    fn try_from_str_fails_on_invalid_json() {
        let result = IdentityPublicKeyV0::try_from("not valid json");
        match result {
            Err(ProtocolError::StringDecodeError(_)) => {}
            other => panic!("expected StringDecodeError, got {:?}", other),
        }
    }

    #[test]
    fn try_from_str_fails_when_data_is_not_base64() {
        let s = r#"{
            "id": 1,
            "type": 0,
            "purpose": 0,
            "securityLevel": 0,
            "readOnly": false,
            "data": "!!!not-base64!!!"
        }"#;
        let result = IdentityPublicKeyV0::try_from(s);
        assert!(result.is_err());
    }

    #[test]
    fn from_json_object_fails_on_missing_fields() {
        let json = serde_json::json!({ "id": 1 });
        let result = IdentityPublicKeyV0::from_json_object(json, LATEST_PLATFORM_VERSION);
        assert!(result.is_err());
    }
}
