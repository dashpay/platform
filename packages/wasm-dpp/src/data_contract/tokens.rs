use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
use dpp::data_contract::TokenConfiguration;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone)]
#[wasm_bindgen(js_name = "TokenConfiguration")]
pub struct TokenConfigurationWasm(TokenConfiguration);

impl From<TokenConfiguration> for TokenConfigurationWasm {
    fn from(value: TokenConfiguration) -> Self {
        Self(value)
    }
}

impl From<TokenConfigurationWasm> for TokenConfiguration {
    fn from(val: TokenConfigurationWasm) -> Self {
        val.0
    }
}

#[wasm_bindgen(js_class = "TokenConfiguration")]
impl TokenConfigurationWasm {
    #[wasm_bindgen(js_name=keepsHistory)]
    pub fn keeps_history(&self) -> TokenKeepsHistoryRulesWasm {
        TokenKeepsHistoryRulesWasm(*self.0.keeps_history())
    }
}

#[derive(Debug, Clone)]
#[wasm_bindgen(js_name = "TokenKeepsHistoryRules")]
pub struct TokenKeepsHistoryRulesWasm(TokenKeepsHistoryRules);

#[wasm_bindgen(js_class = "TokenKeepsHistoryRules")]
impl TokenKeepsHistoryRulesWasm {
    /// Whether transfer history is recorded.
    #[wasm_bindgen(js_name=keepsTransferHistory)]
    pub fn keeps_transfer_history(&self) -> bool {
        self.0.keeps_transfer_history()
    }

    /// Whether freezing history is recorded.
    #[wasm_bindgen(js_name=keepsFreezingHistory)]
    pub fn keeps_freezing_history(&self) -> bool {
        self.0.keeps_freezing_history()
    }

    /// Whether minting history is recorded.
    #[wasm_bindgen(js_name=keepsMintingHistory)]
    pub fn keeps_minting_history(&self) -> bool {
        self.0.keeps_minting_history()
    }

    /// Whether burning history is recorded.
    #[wasm_bindgen(js_name=keepsBurningHistory)]
    pub fn keeps_burning_history(&self) -> bool {
        self.0.keeps_burning_history()
    }
}
