use dpp::consensus::basic::document::MissingDocumentTransitionTypeError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingDocumentTransitionTypeError)]
pub struct MissingDocumentTransitionTypeErrorWasm {
    inner: MissingDocumentTransitionTypeError,
}

impl From<&MissingDocumentTransitionTypeError> for MissingDocumentTransitionTypeErrorWasm {
    fn from(e: &MissingDocumentTransitionTypeError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=MissingDocumentTransitionTypeError)]
impl MissingDocumentTransitionTypeErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
