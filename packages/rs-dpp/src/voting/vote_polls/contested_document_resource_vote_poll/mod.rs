#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
use crate::serialization::PlatformSerializable;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::util::hash::hash_double;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::{Identifier, Value};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_serialize(limit = 100000)]
pub struct ContestedDocumentResourceVotePoll {
    pub contract_id: Identifier,
    pub document_type_name: String,
    pub index_name: String,
    pub index_values: Vec<Value>,
}

impl fmt::Display for ContestedDocumentResourceVotePoll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format the index_values as a comma-separated list
        let index_values_str: Vec<String> =
            self.index_values.iter().map(|v| v.to_string()).collect();
        write!(
            f,
            "ContestedDocumentResourceVotePoll {{ contract_id: {}, document_type_name: {}, index_name: {}, index_values: [{}] }}",
            self.contract_id,
            self.document_type_name,
            self.index_name,
            index_values_str.join(", ")
        )
    }
}

impl Default for ContestedDocumentResourceVotePoll {
    fn default() -> Self {
        ContestedDocumentResourceVotePoll {
            contract_id: Default::default(),
            document_type_name: "".to_string(),
            index_name: "".to_string(),
            index_values: vec![],
        }
    }
}

impl ContestedDocumentResourceVotePoll {
    pub fn sha256_2_hash(&self) -> Result<[u8; 32], ProtocolError> {
        let encoded = self.serialize_to_bytes()?;
        Ok(hash_double(encoded))
    }

    pub fn specialized_balance_id(&self) -> Result<Identifier, ProtocolError> {
        self.unique_id()
    }

    pub fn unique_id(&self) -> Result<Identifier, ProtocolError> {
        self.sha256_2_hash().map(Identifier::new)
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
    use platform_value::platform_value;
    use serde_json::json;

    /// Non-default values per field (real contract id, named type/index, two
    /// index values) so the wire-shape assertion catches silent zero-out /
    /// vec-truncate on round-trip.
    fn fixture() -> ContestedDocumentResourceVotePoll {
        ContestedDocumentResourceVotePoll {
            contract_id: Identifier::new([0xc1; 32]),
            document_type_name: "preorder".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text("alice".to_string()),
            ],
        }
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // This is a plain struct (no `#[serde(tag)]`), so there is no
        // `$formatVersion` on the wire. `Identifier` -> base58 string.
        // `Value::Text` inside the array -> JSON string.
        assert_eq!(
            json,
            json!({
                "contractId": "E3M3d7sy8ZKivUGxBexL9wxE7ebqzGWFqkdeFMedCJFS",
                "documentTypeName": "preorder",
                "indexName": "parentNameAndLabel",
                "indexValues": ["dash", "alice"],
            })
        );
        let recovered = ContestedDocumentResourceVotePoll::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Interpolate the `Identifier` via `platform_value!` so Serialize emits
        // `Value::Identifier` (NOT `Value::Bytes32`). `index_values` is a
        // `Vec<Value>` round-tripped element-wise.
        let id = Identifier::new([0xc1; 32]);
        assert_eq!(
            value,
            platform_value!({
                "contractId": id,
                "documentTypeName": "preorder",
                "indexName": "parentNameAndLabel",
                "indexValues": ["dash", "alice"],
            })
        );
        let recovered = ContestedDocumentResourceVotePoll::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
