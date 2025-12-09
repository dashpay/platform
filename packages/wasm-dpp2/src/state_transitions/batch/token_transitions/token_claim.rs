use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::enums::token::distribution_type::TokenDistributionTypeWasm;
use crate::error::WasmDppResult;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use dpp::state_transition::batch_transition::token_claim_transition::TokenClaimTransitionV0;
use dpp::state_transition::batch_transition::TokenClaimTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenClaimTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenClaimTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWasm,
        js_distribution_type: &JsValue,
        public_note: Option<String>,
    ) -> WasmDppResult<TokenClaimTransitionWasm> {
        let distribution_type = match js_distribution_type.is_undefined() {
            true => TokenDistributionTypeWasm::default(),
            false => TokenDistributionTypeWasm::try_from(js_distribution_type.clone())?,
        };

        Ok(TokenClaimTransitionWasm(TokenClaimTransition::V0(
            TokenClaimTransitionV0 {
                base: base.clone().into(),
                distribution_type: distribution_type.into(),
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

    #[wasm_bindgen(getter = "distributionType")]
    pub fn get_distribution_type(&self) -> String {
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
    pub fn set_distribution_type(&mut self, js_distribution_type: &JsValue) -> WasmDppResult<()> {
        let distribution_type = match js_distribution_type.is_undefined() {
            true => TokenDistributionTypeWasm::default(),
            false => TokenDistributionTypeWasm::try_from(js_distribution_type.clone())?,
        };

        self.0.set_distribution_type(distribution_type.into());
        Ok(())
    }
}
