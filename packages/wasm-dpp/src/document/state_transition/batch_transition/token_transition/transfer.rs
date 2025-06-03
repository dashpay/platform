use crate::identifier::IdentifierWrapper;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenTransferTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenTransferTransition)]
#[derive(Debug, Clone)]
pub struct TokenTransferTransitionWasm(TokenTransferTransition);

impl From<TokenTransferTransition> for TokenTransferTransitionWasm {
    fn from(value: TokenTransferTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenTransferTransition)]
impl TokenTransferTransitionWasm {
    #[wasm_bindgen(js_name=getRecipientId)]
    pub fn recipient_id(&self) -> IdentifierWrapper {
        self.0.recipient_id().into()
    }

    #[wasm_bindgen(js_name=getPublicNote)]
    pub fn public_note(&self) -> Option<String> {
        match self.0.public_note() {
            Some(note) => Some(note.clone()),
            None => None,
        }
    }

    #[wasm_bindgen(js_name=getAmount)]
    pub fn amount(&self) -> u64 {
        self.0.amount()
    }
}
