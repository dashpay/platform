use dpp::state_transition::batch_transition::token_burn_from_pool_transition::v0::v0_methods::TokenBurnFromPoolTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenBurnFromPoolTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenBurnFromPoolTransition)]
#[derive(Debug, Clone)]
pub struct TokenBurnFromPoolTransitionWasm(TokenBurnFromPoolTransition);

impl From<TokenBurnFromPoolTransition> for TokenBurnFromPoolTransitionWasm {
    fn from(value: TokenBurnFromPoolTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenBurnFromPoolTransition)]
impl TokenBurnFromPoolTransitionWasm {
    #[wasm_bindgen(js_name=getAmount)]
    pub fn amount(&self) -> u64 {
        self.0.amount()
    }

    #[wasm_bindgen(js_name=getPublicNote)]
    pub fn public_note(&self) -> Option<String> {
        self.0.public_note().cloned()
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
