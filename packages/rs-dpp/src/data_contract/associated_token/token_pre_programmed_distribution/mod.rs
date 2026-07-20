use crate::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod accessors;

pub mod v0;

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From)]
#[serde(tag = "$formatVersion")]
pub enum TokenPreProgrammedDistribution {
    #[serde(rename = "0")]
    V0(TokenPreProgrammedDistributionV0),
}

impl fmt::Display for TokenPreProgrammedDistribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenPreProgrammedDistribution::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
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
    use crate::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use platform_value::{Identifier, Value};
    use serde_json::json;
    use std::collections::BTreeMap;

    /// Non-default fixture with two distinct timestamps and two recipients per
    /// timestamp so the wire-shape assertion catches silent map-flatten /
    /// key-swap on round-trip.
    fn fixture() -> TokenPreProgrammedDistribution {
        let mut early = BTreeMap::new();
        early.insert(Identifier::new([0xab; 32]), 1000u64);
        early.insert(Identifier::new([0xcd; 32]), 2000u64);

        let mut late = BTreeMap::new();
        late.insert(Identifier::new([0xef; 32]), 3000u64);

        let mut distributions = BTreeMap::new();
        distributions.insert(1_700_000_000_000u64, early);
        distributions.insert(1_800_000_000_000u64, late);
        TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 { distributions })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `distributions` is `BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>`.
        // JSON requires string keys so the u64 timestamps render as quoted strings
        // ("1700000000000") and the Identifier keys render as base58 strings.
        // Inner amounts are `u64`; JSON erases the size — the value-path assertion
        // below uses `1000u64` etc. to lock in `Value::U64`.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "distributions": {
                    "1700000000000": {
                        "CZ8YUVdk7znjrUmnb5n7kgySk9yRAsQDYmyCxzfSky9t": 1000,
                        "ErNbLjU6E8tSbZH3REsMeTDP3Z8G52k6YedWwvBpAJ7v": 2000,
                    },
                    "1800000000000": {
                        "H9ceCyJSLGz9LdnJFPxbYDTKLxH6yC5yYXHpvqiBZd5x": 3000,
                    },
                },
            })
        );
        let recovered = TokenPreProgrammedDistribution::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // platform_value preserves typed keys: the outer `BTreeMap<u64, ...>`
        // emits `Value::U64` keys (NOT stringified); the inner
        // `BTreeMap<Identifier, ...>` emits `Value::Identifier` keys. Inner
        // values are `Value::U64` because `TokenAmount` is u64. We construct
        // the expected map directly because `platform_value!{...}` only emits
        // string-keyed maps.
        let expected = Value::Map(vec![
            (
                Value::Text("$formatVersion".to_string()),
                Value::Text("0".to_string()),
            ),
            (
                Value::Text("distributions".to_string()),
                Value::Map(vec![
                    (
                        Value::U64(1_700_000_000_000),
                        Value::Map(vec![
                            (Value::Identifier([0xab; 32]), Value::U64(1000)),
                            (Value::Identifier([0xcd; 32]), Value::U64(2000)),
                        ]),
                    ),
                    (
                        Value::U64(1_800_000_000_000),
                        Value::Map(vec![(Value::Identifier([0xef; 32]), Value::U64(3000))]),
                    ),
                ]),
            ),
        ]);
        assert_eq!(value, expected);
        let recovered = TokenPreProgrammedDistribution::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
