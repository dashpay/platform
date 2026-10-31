use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use crate::utils::try_to_u64;
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
    #[wasm_bindgen(js_name = "NotTradeable")]
    pub fn not_tradeable() -> TokenTradeModeWasm {
        TokenTradeModeWasm(TokenTradeMode::NotTradeable)
    }

    /// Holders may list the token and make offers for it (protocol version 15), each fill taking
    /// at least `minFill` base units unless it takes everything an order has left.
    #[wasm_bindgen(js_name = "Tradeable")]
    pub fn tradeable(
        #[wasm_bindgen(js_name = "minFill")] min_fill: &js_sys::BigInt,
    ) -> WasmDppResult<TokenTradeModeWasm> {
        Ok(TokenTradeModeWasm(TokenTradeMode::Tradeable {
            min_fill: try_to_u64(min_fill, "minFill")?,
        }))
    }

    #[wasm_bindgen(getter = "value")]
    pub fn value(&self) -> String {
        match self.0 {
            TokenTradeMode::NotTradeable => String::from("NotTradeable"),
            TokenTradeMode::Tradeable { .. } => String::from("Tradeable"),
        }
    }

    /// The smallest fill of the token's orders, `undefined` when it is not tradeable.
    #[wasm_bindgen(getter = "minFill")]
    pub fn min_fill(&self) -> Option<u64> {
        self.0.min_fill()
    }
}

impl_wasm_type_info!(TokenTradeModeWasm, TokenTradeMode);
