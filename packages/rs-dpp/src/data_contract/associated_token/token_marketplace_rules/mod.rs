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
pub enum TokenMarketplaceRules {
    #[serde(rename = "0")]
    V0(TokenMarketplaceRulesV0),
}

use crate::data_contract::associated_token::token_marketplace_rules::v0::TokenMarketplaceRulesV0;
use std::fmt;

impl fmt::Display for TokenMarketplaceRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenMarketplaceRules::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::associated_token::token_marketplace_rules::v0::{
        TokenMarketplaceRulesV0, TokenTradeMode,
    };
    use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;

    /// Non-default values per inner field (non-NoOne action takers + flipped
    /// bool flags) so per-property assertions catch silent zero-out / flip.
    fn fixture() -> TokenMarketplaceRules {
        TokenMarketplaceRules::V0(TokenMarketplaceRulesV0 {
            trade_mode: TokenTradeMode::NotTradeable,
            trade_mode_change_rules: ChangeControlRules::V0(ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                admin_action_takers: AuthorizedActionTakers::MainGroup,
                changing_authorized_action_takers_to_no_one_allowed: true,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: true,
            }),
        })
    }

    fn assert_v0_fields(r: &TokenMarketplaceRules) {
        let TokenMarketplaceRules::V0(rec) = r;
        assert!(
            matches!(rec.trade_mode, TokenTradeMode::NotTradeable),
            "trade_mode = NotTradeable"
        );
        let ChangeControlRules::V0(rules) = &rec.trade_mode_change_rules;
        assert!(
            matches!(rules.authorized_to_make_change, AuthorizedActionTakers::ContractOwner),
            "authorized_to_make_change = ContractOwner"
        );
        assert!(
            matches!(rules.admin_action_takers, AuthorizedActionTakers::MainGroup),
            "admin_action_takers = MainGroup"
        );
        assert!(
            rules.changing_authorized_action_takers_to_no_one_allowed,
            "changing_authorized_action_takers_to_no_one_allowed"
        );
        assert!(
            !rules.changing_admin_action_takers_to_no_one_allowed,
            "changing_admin_action_takers_to_no_one_allowed (false)"
        );
        assert!(
            rules.self_changing_admin_action_takers_allowed,
            "self_changing_admin_action_takers_allowed"
        );
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = TokenMarketplaceRules::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = TokenMarketplaceRules::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
