use dpp::state_transition::batch_transition::token_shield_transition::v0::v0_methods::TokenShieldTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenShieldTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenShieldTransition)]
#[derive(Debug, Clone)]
pub struct TokenShieldTransitionWasm(TokenShieldTransition);

impl From<TokenShieldTransition> for TokenShieldTransitionWasm {
    fn from(value: TokenShieldTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenShieldTransition)]
impl TokenShieldTransitionWasm {
    #[wasm_bindgen(js_name=getAmount)]
    pub fn amount(&self) -> u64 {
        self.0.amount()
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
