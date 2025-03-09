// use crate::document::state_transition::document_batch_transition::batched_transition::from_document_transition_to_js_value;
use super::*;
use dpp::state_transition::state_transitions::document::batch_transition::batched_transition::{
    action_type::TransitionActionTypeGetter, DocumentTransition,
};
use thiserror::Error;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Document action: '{:?}'", document_transition.action_type())]
pub struct InvalidDocumentActionError {
    document_transition: DocumentTransition,
}

#[wasm_bindgen]
impl InvalidDocumentActionError {
    // #[wasm_bindgen(js_name=getDocumentTransition)]
    // pub fn get_document_transition(&self) -> JsValue {
    //     from_document_transition_to_js_value(self.document_transition.clone())
    // }
}

impl InvalidDocumentActionError {
    pub fn new(document_transition: DocumentTransition) -> Self {
        Self {
            document_transition,
        }
    }
}

#[test]
fn data_contract_create_check_tx() {}
