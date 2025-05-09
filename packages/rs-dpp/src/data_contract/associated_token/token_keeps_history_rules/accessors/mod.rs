use crate::data_contract::associated_token::token_keeps_history_rules::accessors::v0::{
    TokenKeepsHistoryRulesV0Getters, TokenKeepsHistoryRulesV0Setters,
};
use crate::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;

pub mod v0;

/// Implementing `TokenKeepsHistoryRulesV0Getters` for `TokenKeepsHistoryRules`
impl TokenKeepsHistoryRulesV0Getters for TokenKeepsHistoryRules {
    fn keeps_transfer_history(&self) -> bool {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_transfer_history,
        }
    }

    fn keeps_freezing_history(&self) -> bool {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_freezing_history,
        }
    }

    fn keeps_minting_history(&self) -> bool {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_minting_history,
        }
    }

    fn keeps_burning_history(&self) -> bool {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_burning_history,
        }
    }

    fn keeps_direct_pricing_history(&self) -> bool {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_direct_pricing_history,
        }
    }

    fn keeps_direct_purchase_history(&self) -> bool {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_direct_purchase_history,
        }
    }
}

/// Implementing `TokenKeepsHistoryRulesV0Setters` for `TokenKeepsHistoryRules`
impl TokenKeepsHistoryRulesV0Setters for TokenKeepsHistoryRules {
    fn set_keeps_transfer_history(&mut self, value: bool) {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_transfer_history = value,
        }
    }

    fn set_keeps_freezing_history(&mut self, value: bool) {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_freezing_history = value,
        }
    }

    fn set_keeps_minting_history(&mut self, value: bool) {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_minting_history = value,
        }
    }

    fn set_keeps_burning_history(&mut self, value: bool) {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_burning_history = value,
        }
    }

    fn set_keeps_direct_pricing_history(&mut self, value: bool) {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_direct_pricing_history = value,
        }
    }

    fn set_keeps_direct_purchase_history(&mut self, value: bool) {
        match self {
            TokenKeepsHistoryRules::V0(v0) => v0.keeps_direct_purchase_history = value,
        }
    }

    fn set_all_keeps_history(&mut self, value: bool) {
        match self {
            TokenKeepsHistoryRules::V0(v0) => {
                v0.keeps_transfer_history = value;
                v0.keeps_freezing_history = value;
                v0.keeps_minting_history = value;
                v0.keeps_burning_history = value;
                v0.keeps_direct_pricing_history = value;
                v0.keeps_direct_purchase_history = value;
            }
        }
    }
}
