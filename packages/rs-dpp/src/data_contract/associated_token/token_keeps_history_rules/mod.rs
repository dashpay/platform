#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};

pub mod accessors;
pub mod v0;

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, Copy, PartialEq, Eq, From)]
#[serde(tag = "$formatVersion")]
pub enum TokenKeepsHistoryRules {
    #[serde(rename = "0")]
    V0(TokenKeepsHistoryRulesV0),
}

use crate::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
use std::fmt;

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;

    fn fixture() -> TokenKeepsHistoryRules {
        TokenKeepsHistoryRules::V0(TokenKeepsHistoryRulesV0 {
            keeps_transfer_history: true,
            keeps_freezing_history: false,
            keeps_minting_history: true,
            keeps_burning_history: false,
            keeps_direct_pricing_history: true,
            keeps_direct_purchase_history: false,
        })
    }

    fn assert_v0_fields(t: &TokenKeepsHistoryRules) {
        let TokenKeepsHistoryRules::V0(rec) = t;
        assert!(rec.keeps_transfer_history, "keeps_transfer_history");
        assert!(!rec.keeps_freezing_history, "keeps_freezing_history");
        assert!(rec.keeps_minting_history, "keeps_minting_history");
        assert!(!rec.keeps_burning_history, "keeps_burning_history");
        assert!(rec.keeps_direct_pricing_history, "keeps_direct_pricing_history");
        assert!(!rec.keeps_direct_purchase_history, "keeps_direct_purchase_history");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = TokenKeepsHistoryRules::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = TokenKeepsHistoryRules::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}

impl fmt::Display for TokenKeepsHistoryRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKeepsHistoryRules::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}
