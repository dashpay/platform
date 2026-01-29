use crate::enums::token::distribution_type::TokenDistributionTypeWasm;
use crate::error::WasmDppResult;
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_optional_with, try_to_string};
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use dpp::state_transition::batch_transition::token_claim_transition::TokenClaimTransitionV0;
use dpp::state_transition::batch_transition::TokenClaimTransition;
use js_sys::Reflect;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_CLAIM_OPTIONS_TS: &str = r#"
export interface TokenClaimTransitionOptions {
    base: TokenBaseTransition;
    distributionType?: TokenDistributionTypeLike;
    publicNote?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenClaimTransitionOptions")]
    pub type TokenClaimTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenClaimTransition")]
pub struct TokenClaimTransitionWasm(TokenClaimTransition);

impl From<TokenClaimTransition> for TokenClaimTransitionWasm {
    fn from(transition: TokenClaimTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenClaimTransitionWasm> for TokenClaimTransition {
    fn from(transition: TokenClaimTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenClaimTransition)]
impl TokenClaimTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenClaimTransitionOptionsJs,
    ) -> WasmDppResult<TokenClaimTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let distribution_type = Reflect::get(&options, &JsValue::from_str("distributionType"))
            .ok()
            .filter(|v| !v.is_undefined())
            .map(|v| TokenDistributionTypeWasm::try_from(v))
            .transpose()?
            .unwrap_or_default();

        let public_note: Option<String> =
            try_from_options_optional_with(&options, "publicNote", |v| {
                try_to_string(v, "publicNote")
            })?;

        Ok(TokenClaimTransitionWasm(TokenClaimTransition::V0(
            TokenClaimTransitionV0 {
                base: base.into(),
                distribution_type: distribution_type.into(),
                public_note,
            },
        )))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "publicNote")]
    pub fn public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = "distributionType")]
    pub fn distribution_type(&self) -> String {
        TokenDistributionTypeWasm::from(self.0.distribution_type()).into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = "publicNote")]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = "distributionType")]
    pub fn set_distribution_type(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "TokenDistributionTypeLike | undefined")]
        distribution_type: &JsValue,
    ) -> WasmDppResult<()> {
        let distribution_type = if distribution_type.is_undefined() {
            TokenDistributionTypeWasm::default()
        } else {
            TokenDistributionTypeWasm::try_from(distribution_type.clone())?
        };

        self.0.set_distribution_type(distribution_type.into());
        Ok(())
    }
}

impl_try_from_js_value!(TokenClaimTransitionWasm, "TokenClaimTransition");
impl_wasm_type_info!(TokenClaimTransitionWasm, TokenClaimTransition);
