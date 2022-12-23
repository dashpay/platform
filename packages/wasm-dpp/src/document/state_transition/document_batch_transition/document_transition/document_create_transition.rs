use dpp::document::document_transition::DocumentCreateTransition;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DocumentCreateTransition)]
#[derive(Debug, Clone)]
pub struct DocumentCreateTransitionWasm {
    inner: DocumentCreateTransition,
}

impl From<DocumentCreateTransition> for DocumentCreateTransitionWasm {
    fn from(v: DocumentCreateTransition) -> Self {
        Self { inner: v }
    }
}

#[wasm_bindgen(js_class=DocumentCreateTransition)]
impl DocumentCreateTransitionWasm {
    #[wasm_bindgen(js_name=getAction)]
    pub fn action(&self) -> u8 {
        self.inner.base.action as u8
    }
}
