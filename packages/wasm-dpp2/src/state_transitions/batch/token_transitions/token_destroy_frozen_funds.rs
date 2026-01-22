use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::TokenDestroyFrozenFundsTransitionV0;
use dpp::state_transition::batch_transition::TokenDestroyFrozenFundsTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenDestroyFrozenFundsTransition")]
pub struct TokenDestroyFrozenFundsTransitionWasm(TokenDestroyFrozenFundsTransition);

impl From<TokenDestroyFrozenFundsTransition> for TokenDestroyFrozenFundsTransitionWasm {
    fn from(transition: TokenDestroyFrozenFundsTransition) -> Self {
        TokenDestroyFrozenFundsTransitionWasm(transition)
    }
}

impl From<TokenDestroyFrozenFundsTransitionWasm> for TokenDestroyFrozenFundsTransition {
    fn from(transition: TokenDestroyFrozenFundsTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenDestroyFrozenFundsTransition)]
impl TokenDestroyFrozenFundsTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        base: &TokenBaseTransitionWasm,
        frozen_identity_id: IdentifierLikeJs,
        public_note: Option<String>,
    ) -> WasmDppResult<TokenDestroyFrozenFundsTransitionWasm> {
        let frozen_identity_id: Identifier = frozen_identity_id.try_into()?;

        Ok(TokenDestroyFrozenFundsTransitionWasm(
            TokenDestroyFrozenFundsTransition::V0(TokenDestroyFrozenFundsTransitionV0 {
                base: base.clone().into(),
                frozen_identity_id,
                public_note,
            }),
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

    #[wasm_bindgen(getter = "frozenIdentityId")]
    pub fn get_frozen_identity_id(&self) -> IdentifierWasm {
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
        frozen_identity_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0
            .set_frozen_identity_id(frozen_identity_id.try_into()?);
        Ok(())
    }
}

impl_wasm_type_info!(
    TokenDestroyFrozenFundsTransitionWasm,
    TokenDestroyFrozenFundsTransition
);
