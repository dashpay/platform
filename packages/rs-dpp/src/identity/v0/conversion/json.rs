use crate::identity::conversion::json::IdentityJsonConversionMethodsV0;
use crate::identity::conversion::platform_value::IdentityPlatformValueConversionMethodsV0;
use crate::identity::{identity_public_key, IdentityV0, IDENTIFIER_FIELDS_RAW_OBJECT};
use crate::ProtocolError;
use platform_value::{ReplacementType, Value};
use serde_json::Value as JsonValue;
use std::convert::TryInto;

impl IdentityJsonConversionMethodsV0 for IdentityV0 {
    fn to_json_object(&self) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object()?
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }

    /// Creates an identity from a json structure
    fn from_json(json_object: JsonValue) -> Result<Self, ProtocolError> {
        let mut platform_value: Value = json_object.into();

        platform_value
            .replace_at_paths(IDENTIFIER_FIELDS_RAW_OBJECT, ReplacementType::Identifier)?;

        if let Some(public_keys_array) = platform_value.get_optional_array_mut_ref("publicKeys")? {
            for public_key in public_keys_array.iter_mut() {
                public_key.replace_at_paths(
                    identity_public_key::BINARY_DATA_FIELDS,
                    ReplacementType::BinaryBytes,
                )?;
            }
        }

        let identity: Self = platform_value::from_value(platform_value)?;

        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use platform_value::{BinaryData, Identifier};
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
                data: BinaryData::new(vec![0x33; 33]),
                disabled_at: None,
            }),
        );
        IdentityV0 {
            id: Identifier::from([9u8; 32]),
            public_keys: keys,
            balance: 42,
            revision: 1,
        }
    }

    #[test]
    fn to_json_contains_expected_top_level_fields() {
        let id = sample_identity_v0();
        let json = id.to_json().expect("to_json");
        let obj = json.as_object().expect("object");
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("publicKeys"));
        assert!(obj.contains_key("balance"));
        assert!(obj.contains_key("revision"));
    }

    // V0 to_json -> from_json round-trip succeeds.
    //
    // Previously, this combination failed because `BinaryData::Deserialize` was
    // asymmetric — its human-readable visitor accepted only strings and the binary
    // visitor accepted only byte sequences, while `platform_value`'s nested
    // deserializers default to `is_human_readable() = true` even when the value
    // carries `Value::Bytes`. The fix in PR #3235 made `BinaryData::Deserialize`
    // symmetric (accepts both strings and bytes regardless of mode, mirroring the
    // `Identifier` pattern). Bincode `Encode`/`Decode` derives are untouched, so
    // consensus binary format is unchanged — only the serde JSON path now
    // round-trips cleanly.
    #[test]
    fn to_json_then_from_json_round_trips_v0() {
        let id = sample_identity_v0();
        let json = id.to_json().unwrap();
        let back = IdentityV0::from_json(json).expect("v0 round-trip should succeed");
        assert_eq!(id, back);
    }

    #[test]
    fn to_json_object_encodes_identifier_as_bytes_array() {
        // to_json_object goes through try_into_validating_json, which represents
        // identifiers (32 bytes) as a JSON array of numbers.
        let id = sample_identity_v0();
        let json = id.to_json_object().expect("to_json_object");
        let obj = json.as_object().expect("object");
        let id_field =
            obj.get("id").expect("id").as_array().expect(
                "to_json_object should render the identifier as a JSON array of byte values",
            );
        assert_eq!(id_field.len(), 32);
    }

    #[test]
    fn from_json_fails_on_garbage_input() {
        let json = serde_json::json!({ "id": "not-a-valid-identifier" });
        let result = IdentityV0::from_json(json);
        assert!(result.is_err());
    }

    // frozen: V0 consensus behavior
    //
    // The JSON fixture does not carry the inner-enum `$formatVersion` tag that
    // `IdentityPublicKey` deserialization requires, so `from_json` fails on it.
    // This is the canonical V0 shape of the fixture — the intent is to document
    // that `from_json` cannot ingest the legacy fixture form directly.
    #[test]
    fn from_json_fixture_fails_missing_format_version_v0_frozen() {
        use crate::tests::fixtures::identity_fixture_json;
        let json = identity_fixture_json();
        let result = IdentityV0::from_json(json);
        match result {
            Err(e) => {
                let msg = format!("{:?}", e);
                assert!(
                    msg.contains("$formatVersion") || msg.contains("formatVersion"),
                    "expected missing-formatVersion error, got {msg}"
                );
            }
            Ok(_) => panic!("expected from_json on legacy fixture to fail"),
        }
    }

    #[test]
    fn from_json_errors_when_public_keys_field_is_not_array() {
        // publicKeys is expected to be an array; using a string should fail early.
        let json = serde_json::json!({
            "id": "3bufpwQjL5qsvuP4fmCKgXJrKG852DDMYfi9J6XKqPAT",
            "publicKeys": "oops",
            "balance": 0,
            "revision": 0,
        });
        let result = IdentityV0::from_json(json);
        assert!(result.is_err());
    }
}
