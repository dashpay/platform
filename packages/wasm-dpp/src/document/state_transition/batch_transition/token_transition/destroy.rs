use crate::identifier::IdentifierWrapper;
use wasm_bindgen::prelude::wasm_bindgen;
use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenDestroyFrozenFundsTransition;

#[wasm_bindgen(js_name=TokenDestroyFrozenFundsTransition)]
#[derive(Debug, Clone)]
pub struct TokenDestroyFrozenFundsTransitionWasm(TokenDestroyFrozenFundsTransition);

impl From<TokenDestroyFrozenFundsTransition> for TokenDestroyFrozenFundsTransitionWasm {
    fn from(value: TokenDestroyFrozenFundsTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen]
impl TokenDestroyFrozenFundsTransitionWasm {
    #[wasm_bindgen(js_name=getFrozenIdentityId)]
    pub fn frozen_identity_id(&self) -> IdentifierWrapper {
        self.0.frozen_identity_id().into()
    }
}
