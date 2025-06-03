use dpp::state_transition::batch_transition::TokenEmergencyActionTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use dpp::state_transition::batch_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use dpp::tokens::emergency_action::TokenEmergencyAction;

#[wasm_bindgen(js_name=TokenEmergencyActionTransition)]
#[derive(Debug, Clone)]
pub struct TokenEmergencyActionTransitionWasm(TokenEmergencyActionTransition);

impl From<TokenEmergencyActionTransition> for TokenEmergencyActionTransitionWasm {
    fn from(value: TokenEmergencyActionTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = TokenEmergencyActionTransition)]
impl TokenEmergencyActionTransitionWasm {
    #[wasm_bindgen(js_name=getPublicNote)]
    pub fn public_note(&self) -> Option<String> {
        match self.0.public_note() {
            Some(note) => Some(note.clone()),
            None => None,
        }
    }

    #[wasm_bindgen(js_name=getEmergencyAction)]
    pub fn emergency_action(&self) -> u8 {
        match self.0.emergency_action() {
            TokenEmergencyAction::Pause => 0,
            TokenEmergencyAction::Resume => 1
        }
    }
}