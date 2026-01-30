use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use crate::utils::{try_from_options, try_from_options_optional_with, try_to_string};
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use dpp::state_transition::batch_transition::token_config_update_transition::TokenConfigUpdateTransitionV0;
use dpp::state_transition::batch_transition::TokenConfigUpdateTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_CONFIG_UPDATE_OPTIONS_TS: &str = r#"
export interface TokenConfigUpdateTransitionOptions {
    base: TokenBaseTransition;
    updateTokenConfigurationItem: TokenConfigurationChangeItem;
    publicNote?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenConfigUpdateTransitionOptions")]
    pub type TokenConfigUpdateTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenConfigUpdateTransition")]
pub struct TokenConfigUpdateTransitionWasm(TokenConfigUpdateTransition);

impl From<TokenConfigUpdateTransitionWasm> for TokenConfigUpdateTransition {
    fn from(transition: TokenConfigUpdateTransitionWasm) -> Self {
        transition.0
    }
}

impl From<TokenConfigUpdateTransition> for TokenConfigUpdateTransitionWasm {
    fn from(transition: TokenConfigUpdateTransition) -> Self {
        TokenConfigUpdateTransitionWasm(transition)
    }
}

#[wasm_bindgen(js_class = TokenConfigUpdateTransition)]
impl TokenConfigUpdateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenConfigUpdateTransitionOptionsJs,
    ) -> WasmDppResult<TokenConfigUpdateTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let update_token_configuration_item: TokenConfigurationChangeItemWasm =
            try_from_options(&options, "updateTokenConfigurationItem")?;

        let public_note: Option<String> =
            try_from_options_optional_with(&options, "publicNote", |v| {
                try_to_string(v, "publicNote")
            })?;

        Ok(TokenConfigUpdateTransitionWasm(
            TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
                base: base.into(),
                update_token_configuration_item: update_token_configuration_item.into(),
                public_note,
            }),
        ))
    }

    #[wasm_bindgen(getter = base)]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = publicNote)]
    pub fn public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = updateTokenConfigurationItem)]
    pub fn update_token_configuration_item(&self) -> TokenConfigurationChangeItemWasm {
        self.0.update_token_configuration_item().clone().into()
    }

    #[wasm_bindgen(setter = base)]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = publicNote)]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = updateTokenConfigurationItem)]
    pub fn set_update_token_configuration_item(&mut self, item: &TokenConfigurationChangeItemWasm) {
        self.0
            .set_update_token_configuration_item(item.clone().into())
    }
}

impl_try_from_js_value!(TokenConfigUpdateTransitionWasm, "TokenConfigUpdateTransition");
impl_wasm_type_info!(TokenConfigUpdateTransitionWasm, TokenConfigUpdateTransition);
