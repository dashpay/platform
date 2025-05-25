use crate::data_contract::associated_token::token_marketplace_rules::accessors::v0::{
    TokenMarketplaceRulesV0Getters, TokenMarketplaceRulesV0Setters,
};
use crate::data_contract::associated_token::token_marketplace_rules::v0::{
    TokenMarketplaceRulesV0, TokenTradeMode,
};
use crate::data_contract::change_control_rules::ChangeControlRules;

/// Implementing `TokenMarketplaceRulesV0Getters` for `TokenMarketplaceRulesV0`
impl TokenMarketplaceRulesV0Getters for TokenMarketplaceRulesV0 {
    fn trade_mode(&self) -> &TokenTradeMode {
        &self.trade_mode
    }

    fn trade_mode_change_rules(&self) -> &ChangeControlRules {
        &self.trade_mode_change_rules
    }

    fn trade_mode_change_rules_mut(&mut self) -> &mut ChangeControlRules {
        &mut self.trade_mode_change_rules
    }
}

/// Implementing `TokenMarketplaceRulesV0Setters` for `TokenMarketplaceRulesV0`
impl TokenMarketplaceRulesV0Setters for TokenMarketplaceRulesV0 {
    fn set_trade_mode(&mut self, trade_mode: TokenTradeMode) {
        self.trade_mode = trade_mode;
    }

    fn set_trade_mode_change_rules(&mut self, rules: ChangeControlRules) {
        self.trade_mode_change_rules = rules;
    }
}
