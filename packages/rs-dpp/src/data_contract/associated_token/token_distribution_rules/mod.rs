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
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From)]
#[serde(tag = "$formatVersion")]
pub enum TokenDistributionRules {
    #[serde(rename = "0")]
    V0(TokenDistributionRulesV0),
}

use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use std::fmt;

impl fmt::Display for TokenDistributionRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenDistributionRules::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;

    fn fixture() -> TokenDistributionRules {
        let ccr = || ChangeControlRules::V0(ChangeControlRulesV0::default());
        TokenDistributionRules::V0(TokenDistributionRulesV0 {
            perpetual_distribution: None,
            perpetual_distribution_rules: ccr(),
            pre_programmed_distribution: None,
            new_tokens_destination_identity: Some(platform_value::Identifier::new([0x42; 32])),
            new_tokens_destination_identity_rules: ccr(),
            minting_allow_choosing_destination: true,
            minting_allow_choosing_destination_rules: ccr(),
            change_direct_purchase_pricing_rules: ccr(),
        })
    }

    #[test]
    fn json_round_trip() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = TokenDistributionRules::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = TokenDistributionRules::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
