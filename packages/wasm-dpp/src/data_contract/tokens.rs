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
