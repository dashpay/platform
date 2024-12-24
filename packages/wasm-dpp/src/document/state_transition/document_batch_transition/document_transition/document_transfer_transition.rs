use wasm_bindgen::prelude::wasm_bindgen;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransferTransition;

#[wasm_bindgen(js_name=DocumentTransferTransition)]
#[derive(Debug, Clone)]
pub struct DocumentTransferTransitionWasm {
    inner: DocumentTransferTransition,
}

impl From<DocumentTransferTransition> for DocumentTransferTransitionWasm {
    fn from(v: DocumentTransferTransition) -> Self {
        Self { inner: v }
    }
}

impl From<DocumentTransferTransitionWasm> for DocumentTransferTransition {
    fn from(v: DocumentTransferTransitionWasm) -> Self {
        v.inner
    }
}

#[wasm_bindgen(js_class=DocumentTransferTransition)]
impl DocumentTransferTransitionWasm {
}

impl DocumentTransferTransitionWasm {
}
