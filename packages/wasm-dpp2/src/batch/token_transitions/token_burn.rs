use crate::batch::token_base_transition::TokenBaseTransitionWASM;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::TokenBurnTransition;
use dpp::state_transition::batch_transition::token_burn_transition::TokenBurnTransitionV0;
use dpp::state_transition::batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenBurnTransition)]
pub struct TokenBurnTransitionWASM(TokenBurnTransition);

impl From<TokenBurnTransition> for TokenBurnTransitionWASM {
    fn from(transition: TokenBurnTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenBurnTransitionWASM> for TokenBurnTransition {
    fn from(transition: TokenBurnTransitionWASM) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenBurnTransition)]
impl TokenBurnTransitionWASM {
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
        base: &TokenBaseTransitionWASM,
        burn_amount: u64,
        public_note: Option<String>,
    ) -> Result<TokenBurnTransitionWASM, JsValue> {
        Ok(TokenBurnTransitionWASM(TokenBurnTransition::V0(
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
    pub fn get_base(&self) -> TokenBaseTransitionWASM {
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
    pub fn set_base(&mut self, base: TokenBaseTransitionWASM) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = publicNote)]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }
}
