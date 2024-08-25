use dpp::consensus::basic::value_error::ValueError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ValueError, inspectable)]
#[derive(Debug)]
pub struct ValueErrorWasm {
    inner: ValueError,
}

impl From<&ValueError> for ValueErrorWasm {
    fn from(e: &ValueError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=ValueError)]
impl ValueErrorWasm {
    #[wasm_bindgen(js_name=getMessage)]
    pub fn get_message(&self) -> String {
        self.inner.value_error().to_string()
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
