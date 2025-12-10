use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::error::WasmDppResult;
use crate::identifier::IdentifierWasm;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::token_freeze_transition::TokenFreezeTransitionV0;
use dpp::state_transition::batch_transition::TokenFreezeTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenFreezeTransition)]
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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenFreezeTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenFreezeTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWasm,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_identity_to_freeze_id: &JsValue,
        public_note: Option<String>,
    ) -> WasmDppResult<TokenFreezeTransitionWasm> {
        let identity_to_freeze_id: Identifier =
            IdentifierWasm::try_from(js_identity_to_freeze_id)?.into();

        Ok(TokenFreezeTransitionWasm(TokenFreezeTransition::V0(
            TokenFreezeTransitionV0 {
                base: base.clone().into(),
                identity_to_freeze_id,
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
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_frozen_identity_id: &JsValue,
    ) -> WasmDppResult<()> {
        self.0
            .set_frozen_identity_id(IdentifierWasm::try_from(js_frozen_identity_id)?.into());
        Ok(())
    }
}
