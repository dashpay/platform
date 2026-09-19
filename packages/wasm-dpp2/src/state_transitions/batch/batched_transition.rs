use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_from_for_extern_type;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::token_transition::TokenTransitionWasm;
use crate::utils::get_class_type;
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::{
    BatchedTransition, BatchedTransitionV1, DocumentTransitionV1,
};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const BATCHED_TRANSITION_TYPES_TS: &str = r#"
export type BatchedTransitionLike = DocumentTransition | TokenTransition;
"#;

/// Extern type for flexible BatchedTransition input
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "BatchedTransitionLike")]
    pub type BatchedTransitionLikeJs;
}

impl_from_for_extern_type!(
    BatchedTransitionLikeJs,
    DocumentTransitionWasm,
    TokenTransitionWasm
);

/// Wraps the batched transition shell of batch format 2, which knows every
/// kind the older batch formats carry plus the erase kind, so one class
/// mirrors a batched transition of any batch format.
#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "BatchedTransition")]
pub struct BatchedTransitionWasm(BatchedTransitionV1);

impl From<BatchedTransitionV1> for BatchedTransitionWasm {
    fn from(v: BatchedTransitionV1) -> Self {
        BatchedTransitionWasm(v)
    }
}

impl From<BatchedTransition> for BatchedTransitionWasm {
    fn from(v: BatchedTransition) -> Self {
        BatchedTransitionWasm(v.into())
    }
}

impl From<BatchedTransitionWasm> for BatchedTransitionV1 {
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
            "TokenTransition" => Ok(BatchedTransitionWasm(BatchedTransitionV1::Token(
                TokenTransition::from(TokenTransitionWasm::try_from(&js_transition)?),
            ))),
            "DocumentTransition" => Ok(BatchedTransitionWasm(BatchedTransitionV1::Document(
                DocumentTransitionV1::from(DocumentTransitionWasm::try_from(&js_transition)?),
            ))),
            _ => Err(WasmDppError::invalid_argument("Invalid transition type")),
        }
    }

    #[wasm_bindgen(js_name = "toTransition")]
    pub fn to_transition(&self) -> BatchedTransitionLikeJs {
        match &self.0 {
            BatchedTransitionV1::Document(document_transition) => {
                DocumentTransitionWasm::from(document_transition.clone()).into()
            }
            BatchedTransitionV1::Token(token_transition) => {
                TokenTransitionWasm::from(token_transition.clone()).into()
            }
        }
    }

    #[wasm_bindgen(getter = "dataContractId")]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        match self.0.clone() {
            BatchedTransitionV1::Document(document_transition) => {
                document_transition.data_contract_id().into()
            }
            BatchedTransitionV1::Token(token_transition) => {
                token_transition.data_contract_id().into()
            }
        }
    }

    #[wasm_bindgen(setter = "dataContractId")]
    pub fn set_data_contract_id(
        &mut self,
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        let contract_id: Identifier = contract_id.try_into()?;

        self.0 = match self.0.clone() {
            BatchedTransitionV1::Document(mut document_transition) => {
                document_transition.set_data_contract_id(contract_id);

                BatchedTransitionV1::Document(document_transition)
            }
            BatchedTransitionV1::Token(mut token_transition) => {
                token_transition.set_data_contract_id(contract_id);

                BatchedTransitionV1::Token(token_transition)
            }
        };

        Ok(())
    }
}

impl_wasm_type_info!(BatchedTransitionWasm, BatchedTransition);
