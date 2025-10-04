use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_emergency_action_transition::TokenEmergencyActionTransitionV0;
use dpp::state_transition::batch_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenEmergencyActionTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::enums::token::emergency_action::TokenEmergencyActionWASM;
use crate::batch::token_base_transition::TokenBaseTransitionWASM;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenEmergencyActionTransition")]
pub struct TokenEmergencyActionTransitionWASM(TokenEmergencyActionTransition);

impl From<TokenEmergencyActionTransitionWASM> for TokenEmergencyActionTransition {
    fn from(transition: TokenEmergencyActionTransitionWASM) -> Self {
        transition.0
    }
}

impl From<TokenEmergencyActionTransition> for TokenEmergencyActionTransitionWASM {
    fn from(transition: TokenEmergencyActionTransition) -> Self {
        TokenEmergencyActionTransitionWASM(transition)
    }
}

#[wasm_bindgen(js_class = TokenEmergencyActionTransition)]
impl TokenEmergencyActionTransitionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenEmergencyActionTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenEmergencyActionTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWASM,
        emergency_action: TokenEmergencyActionWASM,
        public_note: Option<String>,
    ) -> TokenEmergencyActionTransitionWASM {
        TokenEmergencyActionTransitionWASM(TokenEmergencyActionTransition::V0(
            TokenEmergencyActionTransitionV0 {
                base: base.clone().into(),
                emergency_action: emergency_action.into(),
                public_note,
            },
        ))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> TokenBaseTransitionWASM {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "publicNote")]
    pub fn get_public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = "emergencyAction")]
    pub fn get_emergency_action(&self) -> String {
        TokenEmergencyActionWASM::from(self.0.emergency_action()).into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWASM) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = "publicNote")]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = "emergencyAction")]
    pub fn set_emergency_action(&mut self, action: TokenEmergencyActionWASM) {
        self.0.set_emergency_action(action.into())
    }
}
