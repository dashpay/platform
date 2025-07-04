use dpp::state_transition::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenConfigUpdateTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenConfigUpdateTransition)]
#[derive(Debug, Clone)]
pub struct TokenConfigUpdateTransitionWasm(TokenConfigUpdateTransition);

impl From<TokenConfigUpdateTransition> for TokenConfigUpdateTransitionWasm {
    fn from(value: TokenConfigUpdateTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenConfigUpdateTransition)]
impl TokenConfigUpdateTransitionWasm {
    #[wasm_bindgen(js_name=getPublicNote)]
    pub fn public_note(&self) -> Option<String> {
        self.0.public_note().cloned()
    }
}
