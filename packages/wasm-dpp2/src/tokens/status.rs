use dpp::tokens::status::v0::TokenStatusV0Accessors;
use dpp::tokens::status::TokenStatus;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "TokenStatus")]
#[derive(Clone, Debug, PartialEq)]
pub struct TokenStatusWasm(TokenStatus);

impl From<TokenStatus> for TokenStatusWasm {
    fn from(status: TokenStatus) -> Self {
        Self(status)
    }
}

impl From<TokenStatusWasm> for TokenStatus {
    fn from(status: TokenStatusWasm) -> Self {
        status.0
    }
}

#[wasm_bindgen(js_class = TokenStatus)]
impl TokenStatusWasm {
    #[wasm_bindgen(getter = "paused")]
    pub fn paused(&self) -> bool {
        match &self.0 {
            TokenStatus::V0(v0) => v0.paused(),
        }
    }
}
