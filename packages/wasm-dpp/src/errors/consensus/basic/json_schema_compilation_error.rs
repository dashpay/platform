use dpp::consensus::basic::json_schema_compilation_error::JsonSchemaCompilationError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=JsonSchemaCompilationError)]
pub struct JsonSchemaCompilationErrorWasm {
    inner: JsonSchemaCompilationError,
}

impl From<&JsonSchemaCompilationError> for JsonSchemaCompilationErrorWasm {
    fn from(e: &JsonSchemaCompilationError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=JsonSchemaCompilationError)]
impl JsonSchemaCompilationErrorWasm {
    #[wasm_bindgen(js_name=getError)]
    pub fn get_error(&self) -> String {
        self.inner.compilation_error().to_string()
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
