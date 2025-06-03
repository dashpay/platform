use dpp::state_transition::batch_transition::TokenClaimTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::state_transition::batch_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use dpp::tokens::emergency_action::TokenEmergencyAction;
use crate::batch_transition::token_transition::burn::TokenBurnTransitionWasm;

#[wasm_bindgen(js_name=TokenClaimTransition)]
#[derive(Debug, Clone)]
pub struct TokenClaimTransitionWasm(TokenClaimTransition);

impl From<TokenClaimTransition> for TokenClaimTransitionWasm {
    fn from(value: TokenClaimTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenClaimTransition)]
impl TokenClaimTransitionWasm {
    #[wasm_bindgen(js_name=getPublicNote)]
    pub fn public_note(&self) -> Option<String> {
        match self.0.public_note() {
            Some(note) => Some(note.clone()),
            None => None,
        }
    }

    #[wasm_bindgen(js_name=getDistributionType)]
    pub fn distribution_type(&self) -> u8 {
        match self.0.distribution_type() {
            TokenDistributionType::PreProgrammed => 0,
            TokenDistributionType::Perpetual => 1
        }
    }
}