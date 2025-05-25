use crate::data_contract::associated_token::token_marketplace_rules::v0::TokenTradeMode;
use crate::data_contract::change_control_rules::ChangeControlRules;

/// Trait for read accessors for `TokenMarketplaceRulesV0`
pub trait TokenMarketplaceRulesV0Getters {
    /// Returns the current trade mode for the token
    fn trade_mode(&self) -> &TokenTradeMode;

    /// Returns the change control rules for modifying the trade mode
    fn trade_mode_change_rules(&self) -> &ChangeControlRules;

    /// Returns the change control rules as mutable for modifying the trade mode
    fn trade_mode_change_rules_mut(&mut self) -> &mut ChangeControlRules;
}

/// Trait for mutation accessors for `TokenMarketplaceRulesV0`
pub trait TokenMarketplaceRulesV0Setters {
    /// Sets a new trade mode for the token
    fn set_trade_mode(&mut self, trade_mode: TokenTradeMode);

    /// Sets new change control rules for the trade mode
    fn set_trade_mode_change_rules(&mut self, rules: ChangeControlRules);
}
