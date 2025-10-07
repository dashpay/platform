use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_emergency_action_transition::TokenEmergencyActionTransitionV0;
use dpp::state_transition::batch_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenEmergencyActionTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::enums::token::emergency_action::TokenEmergencyActionWasm;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenEmergencyActionTransition")]
pub struct TokenEmergencyActionTransitionWasm(TokenEmergencyActionTransition);

impl From<TokenEmergencyActionTransitionWasm> for TokenEmergencyActionTransition {
    fn from(transition: TokenEmergencyActionTransitionWasm) -> Self {
        transition.0
    }
}

impl From<TokenEmergencyActionTransition> for TokenEmergencyActionTransitionWasm {
    fn from(transition: TokenEmergencyActionTransition) -> Self {
        TokenEmergencyActionTransitionWasm(transition)
    }
}

#[wasm_bindgen(js_class = TokenEmergencyActionTransition)]
impl TokenEmergencyActionTransitionWasm {
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
        base: &TokenBaseTransitionWasm,
        emergency_action: TokenEmergencyActionWasm,
        public_note: Option<String>,
    ) -> TokenEmergencyActionTransitionWasm {
        TokenEmergencyActionTransitionWasm(TokenEmergencyActionTransition::V0(
            TokenEmergencyActionTransitionV0 {
                base: base.clone().into(),
                emergency_action: emergency_action.into(),
                public_note,
            },
        ))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "publicNote")]
    pub fn get_public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = "emergencyAction")]
    pub fn get_emergency_action(&self) -> String {
        TokenEmergencyActionWasm::from(self.0.emergency_action()).into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = "publicNote")]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = "emergencyAction")]
    pub fn set_emergency_action(&mut self, action: TokenEmergencyActionWasm) {
        self.0.set_emergency_action(action.into())
    }
}
