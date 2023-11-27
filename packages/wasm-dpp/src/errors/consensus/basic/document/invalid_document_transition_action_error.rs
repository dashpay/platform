use dpp::consensus::basic::document::InvalidDocumentTransitionActionError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDocumentTransitionActionError)]
pub struct InvalidDocumentTransitionActionErrorWasm {
    inner: InvalidDocumentTransitionActionError,
}

impl From<&InvalidDocumentTransitionActionError> for InvalidDocumentTransitionActionErrorWasm {
    fn from(e: &InvalidDocumentTransitionActionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidDocumentTransitionActionError)]
impl InvalidDocumentTransitionActionErrorWasm {
    #[wasm_bindgen(js_name=getAction)]
    pub fn get_action(&self) -> String {
        self.inner.action().to_string()
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
