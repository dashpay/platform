use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::batch::token_base_transition::TokenBaseTransitionWASM;
use dpp::state_transition::batch_transition::TokenClaimTransition;
use dpp::state_transition::batch_transition::token_claim_transition::TokenClaimTransitionV0;
use dpp::state_transition::batch_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::enums::token::distribution_type::TokenDistributionTypeWASM;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenClaimTransitionWASM")]
pub struct TokenClaimTransitionWASM(TokenClaimTransition);

impl From<TokenClaimTransition> for TokenClaimTransitionWASM {
    fn from(transition: TokenClaimTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenClaimTransitionWASM> for TokenClaimTransition {
    fn from(transition: TokenClaimTransitionWASM) -> Self {
        transition.0
    }
}

#[wasm_bindgen]
impl TokenClaimTransitionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenClaimTransitionWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenClaimTransitionWASM".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWASM,
        js_distribution_type: &JsValue,
        public_note: Option<String>,
    ) -> Result<TokenClaimTransitionWASM, JsValue> {
        let distribution_type = match js_distribution_type.is_undefined() {
            true => TokenDistributionTypeWASM::default(),
            false => TokenDistributionTypeWASM::try_from(js_distribution_type.clone())?,
        };

        Ok(TokenClaimTransitionWASM(TokenClaimTransition::V0(
            TokenClaimTransitionV0 {
                base: base.clone().into(),
                distribution_type: distribution_type.into(),
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

    #[wasm_bindgen(getter = "distributionType")]
    pub fn get_distribution_type(&self) -> String {
        TokenDistributionTypeWASM::from(self.0.distribution_type()).into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWASM) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = "publicNote")]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = "distributionType")]
    pub fn set_distribution_type(&mut self, js_distribution_type: &JsValue) -> Result<(), JsValue> {
        let distribution_type = match js_distribution_type.is_undefined() {
            true => TokenDistributionTypeWASM::default(),
            false => TokenDistributionTypeWASM::try_from(js_distribution_type.clone())?,
        };

        Ok(self.0.set_distribution_type(distribution_type.into()))
    }
}
