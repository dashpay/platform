use crate::data_contract::associated_token::token_marketplace_rules::accessors::v0::{
    TokenMarketplaceRulesV0Getters, TokenMarketplaceRulesV0Setters,
};
use crate::data_contract::associated_token::token_marketplace_rules::v0::TokenTradeMode;
use crate::data_contract::associated_token::token_marketplace_rules::TokenMarketplaceRules;
use crate::data_contract::change_control_rules::ChangeControlRules;

pub mod v0;
/// Implementing `TokenMarketplaceRulesV0Getters` for `TokenMarketplaceRules`
impl TokenMarketplaceRulesV0Getters for TokenMarketplaceRules {
    fn trade_mode(&self) -> &TokenTradeMode {
        match self {
            TokenMarketplaceRules::V0(inner) => inner.trade_mode(),
        }
    }

    fn trade_mode_change_rules(&self) -> &ChangeControlRules {
        match self {
            TokenMarketplaceRules::V0(inner) => inner.trade_mode_change_rules(),
        }
    }

    fn trade_mode_change_rules_mut(&mut self) -> &mut ChangeControlRules {
        match self {
            TokenMarketplaceRules::V0(inner) => inner.trade_mode_change_rules_mut(),
        }
    }
}

/// Implementing `TokenMarketplaceRulesV0Setters` for `TokenMarketplaceRules`
impl TokenMarketplaceRulesV0Setters for TokenMarketplaceRules {
    fn set_trade_mode(&mut self, trade_mode: TokenTradeMode) {
        match self {
            TokenMarketplaceRules::V0(inner) => inner.set_trade_mode(trade_mode),
        }
    }

    fn set_trade_mode_change_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenMarketplaceRules::V0(inner) => inner.set_trade_mode_change_rules(rules),
        }
    }
}
