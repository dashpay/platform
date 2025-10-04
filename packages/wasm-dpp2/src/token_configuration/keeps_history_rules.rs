use dpp::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::{
    TokenKeepsHistoryRulesV0Getters, TokenKeepsHistoryRulesV0Setters,
};
use dpp::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = "TokenKeepsHistoryRules")]
pub struct TokenKeepsHistoryRulesWASM(TokenKeepsHistoryRules);

impl From<TokenKeepsHistoryRulesWASM> for TokenKeepsHistoryRules {
    fn from(rules: TokenKeepsHistoryRulesWASM) -> Self {
        rules.0
    }
}

impl From<TokenKeepsHistoryRules> for TokenKeepsHistoryRulesWASM {
    fn from(rules: TokenKeepsHistoryRules) -> Self {
        Self(rules)
    }
}

#[wasm_bindgen(js_class = TokenKeepsHistoryRules)]
impl TokenKeepsHistoryRulesWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenKeepsHistoryRules".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenKeepsHistoryRules".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        keeps_transfer_history: bool,
        keeps_freezing_history: bool,
        keeps_minting_history: bool,
        keeps_burning_history: bool,
        keeps_direct_pricing_history: bool,
        keeps_direct_purchase_history: bool,
    ) -> TokenKeepsHistoryRulesWASM {
        TokenKeepsHistoryRulesWASM(TokenKeepsHistoryRules::V0(TokenKeepsHistoryRulesV0 {
            keeps_transfer_history,
            keeps_freezing_history,
            keeps_minting_history,
            keeps_burning_history,
            keeps_direct_pricing_history,
            keeps_direct_purchase_history,
        }))
    }

    #[wasm_bindgen(getter = "keepsTransferHistory")]
    pub fn get_keeps_transfer_history(&self) -> bool {
        self.0.keeps_transfer_history()
    }

    #[wasm_bindgen(getter = "keepsFreezingHistory")]
    pub fn get_keeps_freezing_history(&self) -> bool {
        self.0.keeps_freezing_history()
    }

    #[wasm_bindgen(getter = "keepsMintingHistory")]
    pub fn get_keeps_minting_history(&self) -> bool {
        self.0.keeps_minting_history()
    }

    #[wasm_bindgen(getter = "keepsBurningHistory")]
    pub fn get_keeps_burning_history(&self) -> bool {
        self.0.keeps_burning_history()
    }

    #[wasm_bindgen(getter = "keepsDirectPricingHistory")]
    pub fn get_keeps_direct_pricing_history(&self) -> bool {
        self.0.keeps_direct_pricing_history()
    }

    #[wasm_bindgen(getter = "keepsDirectPurchaseHistory")]
    pub fn get_keeps_direct_purchase_history(&self) -> bool {
        self.0.keeps_direct_purchase_history()
    }

    #[wasm_bindgen(setter = "keepsTransferHistory")]
    pub fn set_keeps_transfer_history(&mut self, keeps_transfer_history: bool) {
        self.0.set_keeps_transfer_history(keeps_transfer_history);
    }

    #[wasm_bindgen(setter = "keepsFreezingHistory")]
    pub fn set_keeps_freezing_history(&mut self, keeps_freezing_history: bool) {
        self.0.set_keeps_freezing_history(keeps_freezing_history);
    }

    #[wasm_bindgen(setter = "keepsMintingHistory")]
    pub fn set_keeps_minting_history(&mut self, keeps_minting_history: bool) {
        self.0.set_keeps_minting_history(keeps_minting_history);
    }

    #[wasm_bindgen(setter = "keepsBurningHistory")]
    pub fn set_keeps_burning_history(&mut self, keeps_burning_history: bool) {
        self.0.set_keeps_burning_history(keeps_burning_history);
    }

    #[wasm_bindgen(setter = "keepsDirectPricingHistory")]
    pub fn set_keeps_direct_pricing_history(&mut self, keeps_direct_pricing_history: bool) {
        self.0
            .set_keeps_direct_pricing_history(keeps_direct_pricing_history);
    }

    #[wasm_bindgen(setter = "keepsDirectPurchaseHistory")]
    pub fn set_keeps_direct_purchase_history(&mut self, keeps_direct_purchase_history: bool) {
        self.0
            .set_keeps_direct_purchase_history(keeps_direct_purchase_history);
    }
}
