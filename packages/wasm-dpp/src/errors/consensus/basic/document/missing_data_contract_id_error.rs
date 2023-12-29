use dpp::consensus::basic::document::MissingDataContractIdBasicError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingDataContractIdError)]
pub struct MissingDataContractIdErrorWasm {
    inner: MissingDataContractIdBasicError,
}

impl From<&MissingDataContractIdBasicError> for MissingDataContractIdErrorWasm {
    fn from(e: &MissingDataContractIdBasicError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=MissingDataContractIdError)]
impl MissingDataContractIdErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
