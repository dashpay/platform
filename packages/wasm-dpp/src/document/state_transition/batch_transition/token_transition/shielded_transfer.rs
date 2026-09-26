use dpp::state_transition::batch_transition::token_shielded_transfer_transition::v0::v0_methods::TokenShieldedTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenShieldedTransferTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenShieldedTransferTransition)]
#[derive(Debug, Clone)]
pub struct TokenShieldedTransferTransitionWasm(TokenShieldedTransferTransition);

impl From<TokenShieldedTransferTransition> for TokenShieldedTransferTransitionWasm {
    fn from(value: TokenShieldedTransferTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenShieldedTransferTransition)]
impl TokenShieldedTransferTransitionWasm {
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
