use dpp::errors::consensus::codes::ErrorWithCode;
use dpp::errors::consensus::ConsensusError;
use wasm_bindgen::prelude::wasm_bindgen;

use dpp::errors::consensus::basic::document::DocumentTransitionsAreAbsentError;

#[wasm_bindgen(js_name=DocumentTransitionsAreAbsentError)]
pub struct DocumentTransitionsAreAbsentErrorWasm {
    inner: DocumentTransitionsAreAbsentError,
}

impl From<&DocumentTransitionsAreAbsentError> for DocumentTransitionsAreAbsentErrorWasm {
    fn from(e: &DocumentTransitionsAreAbsentError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DocumentTransitionsAreAbsentError)]
impl DocumentTransitionsAreAbsentErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
