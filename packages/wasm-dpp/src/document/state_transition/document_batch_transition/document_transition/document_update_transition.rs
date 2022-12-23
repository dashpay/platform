use dpp::document::document_transition::DocumentReplaceTransition;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DocumentTransition)]
#[derive(Debug, Clone)]
pub struct DocumentReplaceTransitionWasm {
    inner: DocumentReplaceTransition,
}

impl From<DocumentReplaceTransition> for DocumentReplaceTransitionWasm {
    fn from(v: DocumentReplaceTransition) -> Self {
        Self { inner: v }
    }
}

#[wasm_bindgen(js_class=DocumentTransition)]
impl DocumentReplaceTransitionWasm {
    #[wasm_bindgen(js_name=getAction)]
    pub fn action(&self) -> u8 {
        self.inner.base.action as u8
    }
}
