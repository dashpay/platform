use dpp::state_transition::batch_transition::TokenBurnTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use dpp::state_transition::batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;

#[wasm_bindgen(js_name=TokenBurnTransition)]
#[derive(Debug, Clone)]
pub struct TokenBurnTransitionWasm(TokenBurnTransition);

impl From<TokenBurnTransition> for TokenBurnTransitionWasm {
    fn from(value: TokenBurnTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenBurnTransition)]
impl TokenBurnTransitionWasm {
    #[wasm_bindgen(js_name=getPublicNote)]
    pub fn public_note(&self) -> Option<String> {
        match self.0.public_note() {
            Some(note) => Some(note.clone()),
            None => None,
        }
    }

    #[wasm_bindgen(js_name=getBurnAmount)]
    pub fn amount(&self) -> u64 {
        self.0.burn_amount()
    }
}