use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentifierError)]
pub struct InvalidIdentifierErrorWasm {
    inner: InvalidIdentifierError,
}

impl From<&InvalidIdentifierError> for InvalidIdentifierErrorWasm {
    fn from(e: &InvalidIdentifierError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentifierError)]
impl InvalidIdentifierErrorWasm {
    #[wasm_bindgen(js_name=getIdentifierName)]
    pub fn get_identifier_name(&self) -> String {
        self.inner.identifier_name().to_string()
    }

    #[wasm_bindgen(js_name=getIdentifierError)]
    pub fn get_error(&self) -> String {
        self.inner.message().to_string()
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
