use crate::batch::token_base_transition::TokenBaseTransitionWasm;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::TokenBurnTransition;
use dpp::state_transition::batch_transition::token_burn_transition::TokenBurnTransitionV0;
use dpp::state_transition::batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenBurnTransition)]
pub struct TokenBurnTransitionWasm(TokenBurnTransition);

impl From<TokenBurnTransition> for TokenBurnTransitionWasm {
    fn from(transition: TokenBurnTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenBurnTransitionWasm> for TokenBurnTransition {
    fn from(transition: TokenBurnTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenBurnTransition)]
impl TokenBurnTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenBurnTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenBurnTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWasm,
        burn_amount: u64,
        public_note: Option<String>,
    ) -> Result<TokenBurnTransitionWasm, JsValue> {
        Ok(TokenBurnTransitionWasm(TokenBurnTransition::V0(
            TokenBurnTransitionV0 {
                base: base.clone().into(),
                burn_amount,
                public_note,
            },
        )))
    }

    #[wasm_bindgen(getter = burnAmount)]
    pub fn get_burn_amount(&self) -> u64 {
        self.0.burn_amount()
    }

    #[wasm_bindgen(getter = base)]
    pub fn get_base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = publicNote)]
    pub fn get_public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(setter = burnAmount)]
    pub fn set_burn_amount(&mut self, amount: u64) {
        self.0.set_burn_amount(amount)
    }

    #[wasm_bindgen(setter = base)]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = publicNote)]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }
}
