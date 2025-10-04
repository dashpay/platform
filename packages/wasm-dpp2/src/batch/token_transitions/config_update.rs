use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::batch::token_base_transition::TokenBaseTransitionWASM;
use dpp::state_transition::batch_transition::TokenConfigUpdateTransition;
use dpp::state_transition::batch_transition::token_config_update_transition::TokenConfigUpdateTransitionV0;
use dpp::state_transition::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use crate::token_configuration_change_item::TokenConfigurationChangeItemWASM;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenConfigUpdateTransition")]
pub struct TokenConfigUpdateTransitionWASM(TokenConfigUpdateTransition);

impl From<TokenConfigUpdateTransitionWASM> for TokenConfigUpdateTransition {
    fn from(transition: TokenConfigUpdateTransitionWASM) -> Self {
        transition.0
    }
}

impl From<TokenConfigUpdateTransition> for TokenConfigUpdateTransitionWASM {
    fn from(transition: TokenConfigUpdateTransition) -> Self {
        TokenConfigUpdateTransitionWASM(transition)
    }
}

#[wasm_bindgen(js_class = TokenConfigUpdateTransition)]
impl TokenConfigUpdateTransitionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenConfigUpdateTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenConfigUpdateTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWASM,
        update_token_configuration_item: &TokenConfigurationChangeItemWASM,
        public_note: Option<String>,
    ) -> Result<TokenConfigUpdateTransitionWASM, JsValue> {
        Ok(TokenConfigUpdateTransitionWASM(
            TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
                base: base.clone().into(),
                update_token_configuration_item: update_token_configuration_item.clone().into(),
                public_note,
            }),
        ))
    }

    #[wasm_bindgen(getter = base)]
    pub fn get_base(&self) -> TokenBaseTransitionWASM {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = publicNote)]
    pub fn get_public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = updateTokenConfigurationItem)]
    pub fn get_update_token_configuration_item(&self) -> TokenConfigurationChangeItemWASM {
        self.0.update_token_configuration_item().clone().into()
    }

    #[wasm_bindgen(setter = base)]
    pub fn set_base(&mut self, base: TokenBaseTransitionWASM) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = publicNote)]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = updateTokenConfigurationItem)]
    pub fn set_update_token_configuration_item(&mut self, item: &TokenConfigurationChangeItemWASM) {
        self.0
            .set_update_token_configuration_item(item.clone().into())
    }
}
