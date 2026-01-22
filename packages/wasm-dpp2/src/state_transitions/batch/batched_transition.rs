use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_from_for_extern_type;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::token_transition::TokenTransitionWasm;
use crate::utils::{IntoWasm, get_class_type};
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const BATCHED_TRANSITION_TYPES_TS: &'static str = r#"
export type BatchedTransitionLike = DocumentTransition | TokenTransition;
"#;

/// Extern type for flexible BatchedTransition input
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "BatchedTransitionLike")]
    pub type BatchedTransitionLikeJs;
}

impl_from_for_extern_type!(BatchedTransitionLikeJs, DocumentTransitionWasm, TokenTransitionWasm);

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "BatchedTransition")]
pub struct BatchedTransitionWasm(BatchedTransition);

impl From<BatchedTransition> for BatchedTransitionWasm {
    fn from(v: BatchedTransition) -> Self {
        BatchedTransitionWasm(v)
    }
}

impl From<BatchedTransitionWasm> for BatchedTransition {
    fn from(v: BatchedTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = BatchedTransition)]
impl BatchedTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        transition: BatchedTransitionLikeJs,
    ) -> WasmDppResult<BatchedTransitionWasm> {
        let js_transition: JsValue = transition.into();
        if js_transition.is_undefined() || !js_transition.is_object() {
            return Err(WasmDppError::invalid_argument("transition is undefined"));
        }

        match get_class_type(&js_transition)?.as_str() {
            "TokenTransition" => Ok(BatchedTransitionWasm::from(BatchedTransition::from(
                TokenTransition::from(
                    js_transition
                        .to_wasm::<TokenTransitionWasm>("TokenTransition")?
                        .clone(),
                ),
            ))),
            "DocumentTransition" => Ok(BatchedTransitionWasm(BatchedTransition::Document(
                DocumentTransition::from(
                    js_transition
                        .to_wasm::<DocumentTransitionWasm>("DocumentTransition")?
                        .clone(),
                ),
            ))),
            _ => Err(WasmDppError::invalid_argument("Invalid transition type")),
        }
    }

    #[wasm_bindgen(js_name = "toTransition")]
    pub fn to_transition(&self) -> BatchedTransitionLikeJs {
        match &self.0 {
            BatchedTransition::Document(document_transition) => {
                DocumentTransitionWasm::from(document_transition.clone()).into()
            }
            BatchedTransition::Token(token_transition) => {
                TokenTransitionWasm::from(token_transition.clone()).into()
            }
        }
    }

    #[wasm_bindgen(getter = "dataContractId")]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        match self.0.clone() {
            BatchedTransition::Document(document_transition) => {
                document_transition.data_contract_id().into()
            }
            BatchedTransition::Token(token_transition) => {
                token_transition.data_contract_id().into()
            }
        }
    }

    #[wasm_bindgen(setter = "dataContractId")]
    pub fn set_data_contract_id(&mut self, contract_id: IdentifierLikeJs) -> WasmDppResult<()> {
        let id_value: JsValue = contract_id.into();
        let contract_id: Identifier = IdentifierWasm::try_from(&id_value)?.into();

        self.0 = match self.0.clone() {
            BatchedTransition::Document(mut document_transition) => {
                document_transition.set_data_contract_id(contract_id);

                BatchedTransition::Document(document_transition)
            }
            BatchedTransition::Token(mut token_transition) => {
                token_transition.set_data_contract_id(contract_id);

                BatchedTransition::Token(token_transition)
            }
        };

        Ok(())
    }
}

impl_wasm_type_info!(BatchedTransitionWasm, BatchedTransition);
