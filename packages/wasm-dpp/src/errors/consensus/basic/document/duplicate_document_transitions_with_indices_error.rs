use crate::buffer::Buffer;
use dpp::consensus::basic::document::DuplicateDocumentTransitionsWithIndicesError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use std::iter::FromIterator;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicateDocumentTransitionsWithIndicesError)]
pub struct DuplicateDocumentTransitionsWithIndicesErrorWasm {
    inner: DuplicateDocumentTransitionsWithIndicesError,
}

impl From<&DuplicateDocumentTransitionsWithIndicesError>
    for DuplicateDocumentTransitionsWithIndicesErrorWasm
{
    fn from(e: &DuplicateDocumentTransitionsWithIndicesError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DuplicateDocumentTransitionsWithIndicesError)]
impl DuplicateDocumentTransitionsWithIndicesErrorWasm {
    #[wasm_bindgen(js_name=getDocumentTransitionReferences)]
    pub fn get_references(&self) -> js_sys::Array {
        self.inner
            .references()
            .iter()
            .map(|v| {
                js_sys::Array::from_iter(vec![
                    JsValue::from(v.0.clone()),
                    JsValue::from(Buffer::from_bytes(v.1.as_ref())),
                ])
            })
            .collect()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
