use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_optional_with, try_from_options_with, try_to_string, try_to_u64};
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use dpp::state_transition::batch_transition::token_burn_transition::TokenBurnTransitionV0;
use dpp::state_transition::batch_transition::TokenBurnTransition;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_BURN_OPTIONS_TS: &str = r#"
export interface TokenBurnTransitionOptions {
    base: TokenBaseTransition;
    burnAmount: bigint;
    publicNote?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenBurnTransitionOptions")]
    pub type TokenBurnTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenBurnTransition")]
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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenBurnTransitionOptionsJs,
    ) -> WasmDppResult<TokenBurnTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let burn_amount =
            try_from_options_with(&options, "burnAmount", |v| try_to_u64(v, "burnAmount"))?;

        let public_note: Option<String> =
            try_from_options_optional_with(&options, "publicNote", |v| {
                try_to_string(v, "publicNote")
            })?;

        Ok(TokenBurnTransitionWasm(TokenBurnTransition::V0(
            TokenBurnTransitionV0 {
                base: base.into(),
                burn_amount,
                public_note,
            },
        )))
    }

    #[wasm_bindgen(getter = burnAmount)]
    pub fn burn_amount(&self) -> u64 {
        self.0.burn_amount()
    }

    #[wasm_bindgen(getter = base)]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = publicNote)]
    pub fn public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(setter = burnAmount)]
    pub fn set_burn_amount(&mut self, amount: JsValue) -> WasmDppResult<()> {
        self.0.set_burn_amount(try_to_u64(&amount, "burnAmount")?);
        Ok(())
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

impl_try_from_js_value!(TokenBurnTransitionWasm, "TokenBurnTransition");
impl_wasm_type_info!(TokenBurnTransitionWasm, TokenBurnTransition);
