use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::token_freeze_transition::TokenFreezeTransitionV0;
use dpp::state_transition::batch_transition::TokenFreezeTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenFreezeTransition")]
pub struct TokenFreezeTransitionWasm(TokenFreezeTransition);

impl From<TokenFreezeTransitionWasm> for TokenFreezeTransition {
    fn from(transition: TokenFreezeTransitionWasm) -> Self {
        transition.0
    }
}

impl From<TokenFreezeTransition> for TokenFreezeTransitionWasm {
    fn from(transition: TokenFreezeTransition) -> Self {
        Self(transition)
    }
}

#[wasm_bindgen(js_class = TokenFreezeTransition)]
impl TokenFreezeTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        base: &TokenBaseTransitionWasm,
        #[wasm_bindgen(js_name = "identityToFreezeId")] identity_to_freeze_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "publicNote")] public_note: Option<String>,
    ) -> WasmDppResult<TokenFreezeTransitionWasm> {
        let identity_to_freeze_id: Identifier = identity_to_freeze_id.try_into()?;

        Ok(TokenFreezeTransitionWasm(TokenFreezeTransition::V0(
            TokenFreezeTransitionV0 {
                base: base.clone().into(),
                identity_to_freeze_id,
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

impl_wasm_type_info!(TokenFreezeTransitionWasm, TokenFreezeTransition);
