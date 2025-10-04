use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_freeze_transition::TokenFreezeTransitionV0;
use dpp::state_transition::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenFreezeTransition;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::identifier::IdentifierWASM;
use crate::batch::token_base_transition::TokenBaseTransitionWASM;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenFreezeTransition)]
pub struct TokenFreezeTransitionWASM(TokenFreezeTransition);

impl From<TokenFreezeTransitionWASM> for TokenFreezeTransition {
    fn from(transition: TokenFreezeTransitionWASM) -> Self {
        transition.0
    }
}

impl From<TokenFreezeTransition> for TokenFreezeTransitionWASM {
    fn from(transition: TokenFreezeTransition) -> Self {
        Self(transition)
    }
}

#[wasm_bindgen(js_class = TokenFreezeTransition)]
impl TokenFreezeTransitionWASM {
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
        base: &TokenBaseTransitionWASM,
        js_identity_to_freeze_id: &JsValue,
        public_note: Option<String>,
    ) -> Result<TokenFreezeTransitionWASM, JsValue> {
        let identity_to_freeze_id: Identifier =
            IdentifierWASM::try_from(js_identity_to_freeze_id)?.into();

        Ok(TokenFreezeTransitionWASM(TokenFreezeTransition::V0(
            TokenFreezeTransitionV0 {
                base: base.clone().into(),
                identity_to_freeze_id,
                public_note,
            },
        )))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> TokenBaseTransitionWASM {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "publicNote")]
    pub fn get_public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = "frozenIdentityId")]
    pub fn get_frozen_identity_id(&self) -> IdentifierWASM {
        self.0.frozen_identity_id().into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWASM) {
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
            .set_frozen_identity_id(IdentifierWASM::try_from(js_frozen_identity_id)?.into());
        Ok(())
    }
}
