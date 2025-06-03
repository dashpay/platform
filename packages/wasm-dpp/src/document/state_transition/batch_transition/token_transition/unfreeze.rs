use crate::identifier::IdentifierWrapper;
use dpp::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenUnfreezeTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenUnfreezeTransition)]
#[derive(Debug, Clone)]
pub struct TokenUnfreezeTransitionWasm(TokenUnfreezeTransition);

impl From<TokenUnfreezeTransition> for TokenUnfreezeTransitionWasm {
    fn from(value: TokenUnfreezeTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenUnfreezeTransition)]
impl TokenUnfreezeTransitionWasm {
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
