use crate::identifier::IdentifierWrapper;
use dpp::state_transition::batch_transition::token_unshield_transition::v0::v0_methods::TokenUnshieldTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenUnshieldTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenUnshieldTransition)]
#[derive(Debug, Clone)]
pub struct TokenUnshieldTransitionWasm(TokenUnshieldTransition);

impl From<TokenUnshieldTransition> for TokenUnshieldTransitionWasm {
    fn from(value: TokenUnshieldTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenUnshieldTransition)]
impl TokenUnshieldTransitionWasm {
    #[wasm_bindgen(js_name=getAmount)]
    pub fn amount(&self) -> u64 {
        self.0.amount()
    }

    #[wasm_bindgen(js_name=getRecipientId)]
    pub fn recipient_id(&self) -> IdentifierWrapper {
        self.0.recipient_id().into()
    }

    #[wasm_bindgen(js_name=getActionsCount)]
    pub fn actions_count(&self) -> u32 {
        self.0.actions().len() as u32
    }

    #[wasm_bindgen(js_name=getAnchor)]
    pub fn anchor(&self) -> Vec<u8> {
        self.0.anchor().to_vec()
    }

    #[wasm_bindgen(js_name=getProof)]
    pub fn proof(&self) -> Vec<u8> {
        self.0.proof().to_vec()
    }

    #[wasm_bindgen(js_name=getBindingSignature)]
    pub fn binding_signature(&self) -> Vec<u8> {
        self.0.binding_signature().to_vec()
    }
}
