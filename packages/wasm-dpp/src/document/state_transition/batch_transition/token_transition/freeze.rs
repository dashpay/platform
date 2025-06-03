use crate::identifier::IdentifierWrapper;
use dpp::state_transition::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenFreezeTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenFreezeTransition)]
#[derive(Debug, Clone)]
pub struct TokenFreezeTransitionWasm(TokenFreezeTransition);

impl From<TokenFreezeTransition> for TokenFreezeTransitionWasm {
    fn from(value: TokenFreezeTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenFreezeTransition)]
impl TokenFreezeTransitionWasm {
    #[wasm_bindgen(js_name=getFrozenIdentityId)]
    pub fn frozen_identity_id(&self) -> IdentifierWrapper {
        self.0.frozen_identity_id().into()
    }

    #[wasm_bindgen(js_name=getPublicNote)]
    pub fn public_note(&self) -> Option<String> {
        match self.0.public_note() {
            Some(note) => Some(note.clone()),
            None => None,
        }
    }
}
