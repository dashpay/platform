mod document_create_transition;
mod document_delete_transition;
mod document_update_transition;

pub use document_create_transition::*;
pub use document_delete_transition::*;
pub use document_update_transition::*;

use dpp::prelude::DocumentTransition;
use wasm_bindgen::prelude::*;

pub fn from_document_transition_to_js_value(document_transition: DocumentTransition) -> JsValue {
    match document_transition {
        DocumentTransition::Create(create_transition) => {
            DocumentCreateTransitionWasm::from(create_transition.clone()).into()
        }
        DocumentTransition::Replace(replace_transition) => {
            DocumentReplaceTransitionWasm::from(replace_transition.clone()).into()
        }
        DocumentTransition::Delete(delete_transition) => {
            DocumentDeleteTransitionWasm::from(delete_transition.clone()).into()
        }
    }
}
