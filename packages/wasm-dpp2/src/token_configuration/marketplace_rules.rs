use crate::token_configuration::change_control_rules::ChangeControlRulesWASM;
use crate::token_configuration::trade_mode::TokenTradeModeWASM;
use dpp::data_contract::associated_token::token_marketplace_rules::TokenMarketplaceRules;
use dpp::data_contract::associated_token::token_marketplace_rules::accessors::v0::{
    TokenMarketplaceRulesV0Getters, TokenMarketplaceRulesV0Setters,
};
use dpp::data_contract::associated_token::token_marketplace_rules::v0::TokenMarketplaceRulesV0;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, PartialEq, Debug)]
#[wasm_bindgen(js_name = "TokenMarketplaceRules")]
pub struct TokenMarketplaceRulesWASM(TokenMarketplaceRules);

impl From<TokenMarketplaceRules> for TokenMarketplaceRulesWASM {
    fn from(rules: TokenMarketplaceRules) -> Self {
        TokenMarketplaceRulesWASM(rules)
    }
}

impl From<TokenMarketplaceRulesWASM> for TokenMarketplaceRules {
    fn from(rules: TokenMarketplaceRulesWASM) -> Self {
        rules.0
    }
}

#[wasm_bindgen(js_class = TokenMarketplaceRules)]
impl TokenMarketplaceRulesWASM {
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
        trade_mode: &TokenTradeModeWASM,
        trade_mode_change_rules: &ChangeControlRulesWASM,
    ) -> TokenMarketplaceRulesWASM {
        TokenMarketplaceRulesWASM(TokenMarketplaceRules::V0({
            TokenMarketplaceRulesV0 {
                trade_mode: trade_mode.clone().into(),
                trade_mode_change_rules: trade_mode_change_rules.clone().into(),
            }
        }))
    }

    #[wasm_bindgen(getter = "tradeMode")]
    pub fn trade_mode(&self) -> TokenTradeModeWASM {
        self.0.trade_mode().clone().into()
    }

    #[wasm_bindgen(getter = "tradeModeChangeRules")]
    pub fn trade_mode_change_rules(&self) -> ChangeControlRulesWASM {
        self.0.trade_mode_change_rules().clone().into()
    }

    #[wasm_bindgen(setter = "tradeMode")]
    pub fn set_trade_mode(&mut self, trade_mode: &TokenTradeModeWASM) {
        self.0.set_trade_mode(trade_mode.clone().into());
    }

    #[wasm_bindgen(setter = "tradeModeChangeRules")]
    pub fn set_trade_mode_change_rules(
        &mut self,
        trade_mode_change_rules: &ChangeControlRulesWASM,
    ) {
        self.0
            .set_trade_mode_change_rules(trade_mode_change_rules.clone().into());
    }
}
