use crate::enums::token::emergency_action::TokenEmergencyActionWasm;
use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_optional_with, try_to_string};
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use dpp::state_transition::batch_transition::token_emergency_action_transition::TokenEmergencyActionTransitionV0;
use dpp::state_transition::batch_transition::TokenEmergencyActionTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_EMERGENCY_ACTION_OPTIONS_TS: &str = r#"
export interface TokenEmergencyActionTransitionOptions {
    base: TokenBaseTransition;
    emergencyAction: TokenEmergencyActionLike;
    publicNote?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenEmergencyActionTransitionOptions")]
    pub type TokenEmergencyActionTransitionOptionsJs;
}

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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenEmergencyActionTransitionOptionsJs,
    ) -> WasmDppResult<TokenEmergencyActionTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let emergency_action: TokenEmergencyActionWasm =
            try_from_options(&options, "emergencyAction")?;

        let public_note: Option<String> =
            try_from_options_optional_with(&options, "publicNote", |v| {
                try_to_string(v, "publicNote")
            })?;

        Ok(TokenEmergencyActionTransitionWasm(
            TokenEmergencyActionTransition::V0(TokenEmergencyActionTransitionV0 {
                base: base.into(),
                emergency_action: emergency_action.into(),
                public_note,
            }),
        ))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "publicNote")]
    pub fn public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = "emergencyAction")]
    pub fn emergency_action(&self) -> String {
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

impl_try_from_js_value!(
    TokenEmergencyActionTransitionWasm,
    "TokenEmergencyActionTransition"
);
impl_wasm_type_info!(
    TokenEmergencyActionTransitionWasm,
    TokenEmergencyActionTransition
);
