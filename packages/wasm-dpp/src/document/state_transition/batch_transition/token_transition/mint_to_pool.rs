use dpp::state_transition::batch_transition::token_mint_to_pool_transition::v0::v0_methods::TokenMintToPoolTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenMintToPoolTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenMintToPoolTransition)]
#[derive(Debug, Clone)]
pub struct TokenMintToPoolTransitionWasm(TokenMintToPoolTransition);

impl From<TokenMintToPoolTransition> for TokenMintToPoolTransitionWasm {
    fn from(value: TokenMintToPoolTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenMintToPoolTransition)]
impl TokenMintToPoolTransitionWasm {
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
