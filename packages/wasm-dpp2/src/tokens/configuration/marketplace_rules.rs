use crate::tokens::configuration::change_control_rules::ChangeControlRulesWasm;
use crate::tokens::configuration::trade_mode::TokenTradeModeWasm;
use dpp::data_contract::associated_token::token_marketplace_rules::TokenMarketplaceRules;
use dpp::data_contract::associated_token::token_marketplace_rules::accessors::v0::{
    TokenMarketplaceRulesV0Getters, TokenMarketplaceRulesV0Setters,
};
use dpp::data_contract::associated_token::token_marketplace_rules::v0::TokenMarketplaceRulesV0;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, PartialEq, Debug)]
#[wasm_bindgen(js_name = "TokenMarketplaceRules")]
pub struct TokenMarketplaceRulesWasm(TokenMarketplaceRules);

impl From<TokenMarketplaceRules> for TokenMarketplaceRulesWasm {
    fn from(rules: TokenMarketplaceRules) -> Self {
        TokenMarketplaceRulesWasm(rules)
    }
}

impl From<TokenMarketplaceRulesWasm> for TokenMarketplaceRules {
    fn from(rules: TokenMarketplaceRulesWasm) -> Self {
        rules.0
    }
}

#[wasm_bindgen(js_class = TokenMarketplaceRules)]
impl TokenMarketplaceRulesWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenMarketplaceRules".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenMarketplaceRules".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        trade_mode: &TokenTradeModeWasm,
        trade_mode_change_rules: &ChangeControlRulesWasm,
    ) -> TokenMarketplaceRulesWasm {
        TokenMarketplaceRulesWasm(TokenMarketplaceRules::V0({
            TokenMarketplaceRulesV0 {
                trade_mode: trade_mode.clone().into(),
                trade_mode_change_rules: trade_mode_change_rules.clone().into(),
            }
        }))
    }

    #[wasm_bindgen(getter = "tradeMode")]
    pub fn trade_mode(&self) -> TokenTradeModeWasm {
        self.0.trade_mode().clone().into()
    }

    #[wasm_bindgen(getter = "tradeModeChangeRules")]
    pub fn trade_mode_change_rules(&self) -> ChangeControlRulesWasm {
        self.0.trade_mode_change_rules().clone().into()
    }

    #[wasm_bindgen(setter = "tradeMode")]
    pub fn set_trade_mode(&mut self, trade_mode: &TokenTradeModeWasm) {
        self.0.set_trade_mode(trade_mode.clone().into());
    }

    #[wasm_bindgen(setter = "tradeModeChangeRules")]
    pub fn set_trade_mode_change_rules(
        &mut self,
        trade_mode_change_rules: &ChangeControlRulesWasm,
    ) {
        self.0
            .set_trade_mode_change_rules(trade_mode_change_rules.clone().into());
    }
}
