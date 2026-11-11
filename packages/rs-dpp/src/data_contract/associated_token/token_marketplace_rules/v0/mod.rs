mod accessors;

use crate::balances::credits::TokenAmount;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::change_control_rules::ChangeControlRules;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{DecodeUntrusted, Encode};
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Serialize,
    Deserialize,
    Decode,
    Encode,
    Default,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    DecodeUntrusted,
)]
pub enum TokenTradeMode {
    /// Holders cannot list the token for sale or make offers for it through the platform.
    #[default]
    NotTradeable,
    /// Holders may list the token at a price and make offers for it: listed tokens and offered
    /// credits are held in escrow until the order is filled or cancelled. Admitted from protocol
    /// version 15 (`TOKEN_MARKETPLACE_INITIAL_PROTOCOL_VERSION`), and only on a transferable
    /// token, since every fill moves the token from one identity to another.
    Tradeable {
        /// The smallest amount, in base units, a fill of a listing or an offer of the token
        /// takes unless it takes everything the order still has open, and the least a fill
        /// leaves open: so no order is left holding less than a trade. An order of less is
        /// refused. Every fill reads it as the token has it when the fill executes, so a
        /// change applies to the open orders too. From 1 to `i64::MAX`.
        #[cfg_attr(
            feature = "json-conversion",
            serde(with = "crate::serialization::json_safe_u64")
        )]
        #[serde(rename = "minFill")]
        min_fill: TokenAmount,
    },
}

impl TokenTradeMode {
    /// Whether holders may trade the token through the platform's listings and offers.
    pub fn is_tradeable(&self) -> bool {
        match self {
            TokenTradeMode::NotTradeable => false,
            TokenTradeMode::Tradeable { .. } => true,
        }
    }

    /// The smallest fill of the token's orders, `None` when the token is not tradeable.
    pub fn min_fill(&self) -> Option<TokenAmount> {
        match self {
            TokenTradeMode::NotTradeable => None,
            TokenTradeMode::Tradeable { min_fill } => Some(*min_fill),
        }
    }
}

// Manual impls because TokenTradeMode is a flat enum (not versioned V0/V1).
// serde emits the bare "NotTradeable" string on both wire formats, and
// `{ "Tradeable": { "minFill": .. } }` for a tradeable token (pinned by
// TokenMarketplaceRules' wire-shape tests).
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenTradeMode {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenTradeMode {}

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub struct TokenMarketplaceRulesV0 {
    pub trade_mode: TokenTradeMode,
    #[serde(default = "default_change_control_rules")]
    pub trade_mode_change_rules: ChangeControlRules,
}

fn default_change_control_rules() -> ChangeControlRules {
    ChangeControlRules::V0(ChangeControlRulesV0 {
        authorized_to_make_change: AuthorizedActionTakers::NoOne,
        admin_action_takers: AuthorizedActionTakers::NoOne,
        changing_authorized_action_takers_to_no_one_allowed: false,
        changing_admin_action_takers_to_no_one_allowed: false,
        self_changing_admin_action_takers_allowed: false,
    })
}

impl fmt::Display for TokenMarketplaceRulesV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenMarketplaceRulesV0 {{\n  \
            trade_mode: {:?},\n  \
            trade_mode_change_rules: {},\n\
            }}",
            self.trade_mode, self.trade_mode_change_rules,
        )
    }
}
