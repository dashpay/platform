use dpp::data_contract::associated_token::token_marketplace_rules::v0::TokenTradeMode;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "TokenTradeMode")]
pub struct TokenTradeModeWasm(TokenTradeMode);

impl From<TokenTradeMode> for TokenTradeModeWasm {
    fn from(trade_mode: TokenTradeMode) -> Self {
        TokenTradeModeWasm(trade_mode)
    }
}

impl From<TokenTradeModeWasm> for TokenTradeMode {
    fn from(trade_mode: TokenTradeModeWasm) -> Self {
        trade_mode.0
    }
}

#[wasm_bindgen(js_class = TokenTradeMode)]
impl TokenTradeModeWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenTradeMode".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenTradeMode".to_string()
    }

    #[wasm_bindgen(js_name = "NotTradeable")]
    pub fn not_tradeable() -> TokenTradeModeWasm {
        TokenTradeModeWasm(TokenTradeMode::NotTradeable)
    }

    #[wasm_bindgen(js_name = "getValue")]
    pub fn get_value(&self) -> String {
        match self.0 {
            TokenTradeMode::NotTradeable => String::from("NotTradeable"),
        }
    }
}
