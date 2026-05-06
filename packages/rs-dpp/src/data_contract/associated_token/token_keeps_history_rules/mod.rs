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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
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

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "keepsTransferHistory": true,
                "keepsFreezingHistory": false,
                "keepsMintingHistory": true,
                "keepsBurningHistory": false,
                "keepsDirectPricingHistory": true,
                "keepsDirectPurchaseHistory": false,
            })
        );
        let recovered = TokenKeepsHistoryRules::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "keepsTransferHistory": true,
                "keepsFreezingHistory": false,
                "keepsMintingHistory": true,
                "keepsBurningHistory": false,
                "keepsDirectPricingHistory": true,
                "keepsDirectPurchaseHistory": false,
            })
        );
        let recovered = TokenKeepsHistoryRules::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
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
