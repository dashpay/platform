use crate::errors::consensus::consensus_error::from_consensus_error_ref;
use dpp::{consensus::ConsensusError, validation::ConsensusValidationResult};
use js_sys::JsString;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ValidationResult)]
#[derive(Debug)]
pub struct ValidationResultWasm(ConsensusValidationResult<JsValue>);

impl<T> From<ConsensusValidationResult<T>> for ValidationResultWasm
where
    T: Into<JsValue> + Clone,
{
    fn from(validation_result: ConsensusValidationResult<T>) -> Self {
        ValidationResultWasm(validation_result.map(Into::into))
    }
}

#[wasm_bindgen(js_class=ValidationResult)]
impl ValidationResultWasm {
    /// This is just a test method - doesn't need to be in the resulted binding. Please
    /// remove before shipping
    #[wasm_bindgen(js_name=errorsText)]
    pub fn errors_text(&self) -> Vec<JsString> {
        self.0.errors.iter().map(|e| e.to_string().into()).collect()
    }

    #[wasm_bindgen(js_name=isValid)]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    #[wasm_bindgen(js_name=getErrors)]
    pub fn errors(&self) -> Vec<JsValue> {
        self.0.errors.iter().map(from_consensus_error_ref).collect()
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&self) -> JsValue {
        self.0
            .data_as_borrowed()
            .unwrap_or(&JsValue::undefined())
            .to_owned()
    }

    #[wasm_bindgen(js_name=getFirstError)]
    pub fn get_first_error(&self) -> JsValue {
        if !self.0.errors.is_empty() {
            from_consensus_error_ref(&self.0.errors[0])
        } else {
            JsValue::undefined()
        }
    }
}

impl ValidationResultWasm {
    pub fn add_error(&mut self, error: impl Into<ConsensusError>) {
        self.0.add_error(error)
    }
}
