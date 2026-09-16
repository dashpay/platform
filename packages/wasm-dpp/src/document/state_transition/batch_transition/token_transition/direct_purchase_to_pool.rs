use dpp::state_transition::batch_transition::token_direct_purchase_to_pool_transition::v0::v0_methods::TokenDirectPurchaseToPoolTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenDirectPurchaseToPoolTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenDirectPurchaseToPoolTransition)]
#[derive(Debug, Clone)]
pub struct TokenDirectPurchaseToPoolTransitionWasm(TokenDirectPurchaseToPoolTransition);

impl From<TokenDirectPurchaseToPoolTransition> for TokenDirectPurchaseToPoolTransitionWasm {
    fn from(value: TokenDirectPurchaseToPoolTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenDirectPurchaseToPoolTransition)]
impl TokenDirectPurchaseToPoolTransitionWasm {
    #[wasm_bindgen(js_name=getTokenCount)]
    pub fn token_count(&self) -> u64 {
        self.0.token_count()
    }

    #[wasm_bindgen(js_name=getTotalAgreedPrice)]
    pub fn total_agreed_price(&self) -> u64 {
        self.0.total_agreed_price()
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
