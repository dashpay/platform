mod accessors;

use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::change_control_rules::ChangeControlRules;
use bincode::Encode;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Serialize, Deserialize, Decode, Encode, Default, Debug, Clone, PartialEq, Eq, PartialOrd,
)]
pub enum TokenTradeMode {
    #[default]
    NotTradeable,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
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
