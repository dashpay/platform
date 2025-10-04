use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::batch::token_base_transition::TokenBaseTransitionWasm;
use dpp::state_transition::batch_transition::TokenDestroyFrozenFundsTransition;
use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::TokenDestroyFrozenFundsTransitionV0;
use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::identifier::IdentifierWasm;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenDestroyFrozenFundsTransition)]
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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenDestroyFrozenFundsTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenDestroyFrozenFundsTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWasm,
        js_frozen_identity_id: &JsValue,
        public_note: Option<String>,
    ) -> Result<TokenDestroyFrozenFundsTransitionWasm, JsValue> {
        let frozen_identity_id: Identifier =
            IdentifierWasm::try_from(js_frozen_identity_id)?.into();

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
        js_frozen_identity_id: &JsValue,
    ) -> Result<(), JsValue> {
        self.0
            .set_frozen_identity_id(IdentifierWasm::try_from(js_frozen_identity_id)?.into());
        Ok(())
    }
}
