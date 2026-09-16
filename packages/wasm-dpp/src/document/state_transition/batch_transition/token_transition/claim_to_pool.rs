use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::state_transition::batch_transition::token_claim_to_pool_transition::v0::v0_methods::TokenClaimToPoolTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenClaimToPoolTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=TokenClaimToPoolTransition)]
#[derive(Debug, Clone)]
pub struct TokenClaimToPoolTransitionWasm(TokenClaimToPoolTransition);

impl From<TokenClaimToPoolTransition> for TokenClaimToPoolTransitionWasm {
    fn from(value: TokenClaimToPoolTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenClaimToPoolTransition)]
impl TokenClaimToPoolTransitionWasm {
    #[wasm_bindgen(js_name=getDistributionType)]
    pub fn distribution_type(&self) -> u8 {
        match self.0.distribution_type() {
            TokenDistributionType::PreProgrammed => 0,
            TokenDistributionType::Perpetual => 1,
        }
    }

    #[wasm_bindgen(js_name=getClaimUpTo)]
    pub fn claim_up_to(&self) -> Option<u64> {
        self.0.claim_up_to()
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
