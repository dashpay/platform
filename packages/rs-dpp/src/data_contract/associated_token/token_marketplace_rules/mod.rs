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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::associated_token::token_marketplace_rules::v0::{
        TokenMarketplaceRulesV0, TokenTradeMode,
    };
    use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;
    use platform_value::platform_value;
    use serde_json::json;

    /// Non-default values per inner field (non-NoOne action takers + flipped
    /// bool flags) so the wire-shape assertion catches silent zero-out / flip.
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

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `TokenTradeMode` is an externally-tagged enum with a single unit
        // variant (`NotTradeable`) that serializes as a bare string.
        // `ChangeControlRules` is a versioned enum with `tag = "$formatVersion"`,
        // and `AuthorizedActionTakers` unit variants serialize as bare strings.
        // No sized integers in this fixture — only Text + Bool.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "tradeMode": "NotTradeable",
                "tradeModeChangeRules": {
                    "$formatVersion": "0",
                    "authorizedToMakeChange": {"$type": "contractOwner"},
                    "adminActionTakers": {"$type": "mainGroup"},
                    "changingAuthorizedActionTakersToNoOneAllowed": true,
                    "changingAdminActionTakersToNoOneAllowed": false,
                    "selfChangingAdminActionTakersAllowed": true,
                },
            })
        );
        let recovered = TokenMarketplaceRules::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // No sized integers here — Text + Bool only. Both wire formats agree.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "tradeMode": "NotTradeable",
                "tradeModeChangeRules": {
                    "$formatVersion": "0",
                    "authorizedToMakeChange": {"$type": "contractOwner"},
                    "adminActionTakers": {"$type": "mainGroup"},
                    "changingAuthorizedActionTakersToNoOneAllowed": true,
                    "changingAdminActionTakersToNoOneAllowed": false,
                    "selfChangingAdminActionTakersAllowed": true,
                },
            })
        );
        let recovered = TokenMarketplaceRules::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
