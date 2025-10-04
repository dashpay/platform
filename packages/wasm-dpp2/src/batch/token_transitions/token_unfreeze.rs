use crate::batch::token_base_transition::TokenBaseTransitionWasm;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_unfreeze_transition::TokenUnfreezeTransitionV0;
use dpp::state_transition::batch_transition::TokenUnfreezeTransition;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use crate::identifier::IdentifierWasm;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenUnFreezeTransition)]
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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenUnFreezeTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenUnFreezeTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWasm,
        js_frozen_identity_id: &JsValue,
        public_note: Option<String>,
    ) -> Result<TokenUnFreezeTransitionWasm, JsValue> {
        let frozen_identity_id: Identifier =
            IdentifierWasm::try_from(js_frozen_identity_id)?.into();

        Ok(TokenUnFreezeTransitionWasm(TokenUnfreezeTransition::V0(
            TokenUnfreezeTransitionV0 {
                base: base.clone().into(),
                frozen_identity_id,
                public_note,
            },
        )))
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
