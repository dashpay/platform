use crate::data_contract::associated_token::token_keeps_history_rules::accessors::v0::{
    TokenKeepsHistoryRulesV0Getters, TokenKeepsHistoryRulesV0Setters,
};
use crate::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;

/// Implementing `TokenKeepsHistoryRulesV0Getters` for `TokenKeepsHistoryRulesV0`
impl TokenKeepsHistoryRulesV0Getters for TokenKeepsHistoryRulesV0 {
    fn keeps_transfer_history(&self) -> bool {
        self.keeps_transfer_history
    }

    fn keeps_freezing_history(&self) -> bool {
        self.keeps_freezing_history
    }

    fn keeps_minting_history(&self) -> bool {
        self.keeps_minting_history
    }

    fn keeps_burning_history(&self) -> bool {
        self.keeps_burning_history
    }

    fn keeps_direct_pricing_history(&self) -> bool {
        self.keeps_direct_pricing_history
    }

    fn keeps_direct_purchase_history(&self) -> bool {
        self.keeps_direct_purchase_history
    }
}

/// Implementing `TokenKeepsHistoryRulesV0Setters` for `TokenKeepsHistoryRulesV0`
impl TokenKeepsHistoryRulesV0Setters for TokenKeepsHistoryRulesV0 {
    fn set_keeps_transfer_history(&mut self, value: bool) {
        self.keeps_transfer_history = value;
    }

    fn set_keeps_freezing_history(&mut self, value: bool) {
        self.keeps_freezing_history = value;
    }

    fn set_keeps_minting_history(&mut self, value: bool) {
        self.keeps_minting_history = value;
    }

    fn set_keeps_burning_history(&mut self, value: bool) {
        self.keeps_burning_history = value;
    }

    fn set_keeps_direct_pricing_history(&mut self, value: bool) {
        self.keeps_direct_pricing_history = value;
    }

    fn set_keeps_direct_purchase_history(&mut self, value: bool) {
        self.keeps_direct_purchase_history = value;
    }

    fn set_all_keeps_history(&mut self, value: bool) {
        self.keeps_transfer_history = value;
        self.keeps_freezing_history = value;
        self.keeps_minting_history = value;
        self.keeps_burning_history = value;
        self.keeps_direct_pricing_history = value;
        self.keeps_direct_purchase_history = value;
    }
}
