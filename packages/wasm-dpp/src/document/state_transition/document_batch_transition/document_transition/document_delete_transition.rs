use dpp::document::document_transition::DocumentDeleteTransition;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DocumentDeleteTransition)]
#[derive(Debug, Clone)]
pub struct DocumentDeleteTransitionWasm {
    inner: DocumentDeleteTransition,
}

impl From<DocumentDeleteTransition> for DocumentDeleteTransitionWasm {
    fn from(v: DocumentDeleteTransition) -> Self {
        Self { inner: v }
    }
}

#[wasm_bindgen(js_class=DocumentDeleteTransition)]
impl DocumentDeleteTransitionWasm {
    #[wasm_bindgen(js_name=getAction)]
    pub fn action(&self) -> u8 {
        self.inner.base.action as u8
    }
}
