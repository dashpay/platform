use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_from_options_optional_with, try_to_string};
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::token_unfreeze_transition::TokenUnfreezeTransitionV0;
use dpp::state_transition::batch_transition::TokenUnfreezeTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_UNFREEZE_OPTIONS_TS: &str = r#"
export interface TokenUnFreezeTransitionOptions {
    base: TokenBaseTransition;
    frozenIdentityId: IdentifierLike;
    publicNote?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenUnFreezeTransitionOptions")]
    pub type TokenUnFreezeTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenUnFreezeTransition")]
pub struct TokenUnFreezeTransitionWasm(TokenUnfreezeTransition);

impl From<TokenUnfreezeTransition> for TokenUnFreezeTransitionWasm {
    fn from(transition: TokenUnfreezeTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenUnFreezeTransitionWasm> for TokenUnfreezeTransition {
    fn from(transition: TokenUnFreezeTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenUnFreezeTransition)]
impl TokenUnFreezeTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenUnFreezeTransitionOptionsJs,
    ) -> WasmDppResult<TokenUnFreezeTransitionWasm> {
        let base: TokenBaseTransitionWasm = try_from_options(&options, "base")?;

        let frozen_identity_id: IdentifierWasm = try_from_options(&options, "frozenIdentityId")?;

        let public_note: Option<String> =
            try_from_options_optional_with(&options, "publicNote", |v| {
                try_to_string(v, "publicNote")
            })?;

        Ok(TokenUnFreezeTransitionWasm(TokenUnfreezeTransition::V0(
            TokenUnfreezeTransitionV0 {
                base: base.into(),
                frozen_identity_id: frozen_identity_id.into(),
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

    #[wasm_bindgen(getter = "frozenIdentityId")]
    pub fn frozen_identity_id(&self) -> IdentifierWasm {
        self.0.frozen_identity_id().into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = "publicNote")]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = "frozenIdentityId")]
    pub fn set_frozen_identity_id(
        &mut self,
        #[wasm_bindgen(js_name = "frozenIdentityId")] frozen_identity_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0
            .set_frozen_identity_id(frozen_identity_id.try_into()?);
        Ok(())
    }
}

impl_try_from_js_value!(TokenUnFreezeTransitionWasm, "TokenUnFreezeTransition");
impl_wasm_type_info!(TokenUnFreezeTransitionWasm, TokenUnFreezeTransition);
