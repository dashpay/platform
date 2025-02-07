use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
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

impl Into<TokenConfiguration> for TokenConfigurationWasm {
    fn into(self) -> TokenConfiguration {
        self.0
    }
}

#[wasm_bindgen(js_class = "TokenConfiguration")]
impl TokenConfigurationWasm {
    #[wasm_bindgen(js_name=keepsHistory)]
    pub fn keeps_history(&self) -> bool {
        self.0.keeps_history()
    }
}
