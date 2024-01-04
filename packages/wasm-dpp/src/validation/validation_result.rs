use crate::{
    buffer::Buffer, errors::consensus::consensus_error::from_consensus_error_ref,
    utils::consensus_errors_from_buffers,
};

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
    #[wasm_bindgen(constructor)]
    pub fn new(errors_option: Option<Vec<Buffer>>) -> Result<ValidationResultWasm, JsValue> {
        if let Some(errors) = errors_option {
            let consensus_errors: Vec<ConsensusError> = consensus_errors_from_buffers(errors)?;

            return Ok(Self(ConsensusValidationResult::new_with_errors(
                consensus_errors,
            )));
        }

        Ok(Self(ConsensusValidationResult::new_with_errors(vec![])))
    }

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
    pub fn get_errors(&self) -> Vec<JsValue> {
        self.0.errors.iter().map(from_consensus_error_ref).collect()
    }

    #[wasm_bindgen(getter)]
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

    // TODO: restore?
    // #[wasm_bindgen(js_name = addError)]
    // pub fn add_error_wasm(&mut self, error_buffer: Buffer) -> Result<JsValue, JsValue> {
    //     let error_bytes: Vec<u8> = Uint8Array::new_with_byte_offset_and_length(
    //         &error_buffer.buffer(),
    //         error_buffer.byte_offset(),
    //         error_buffer.length(),
    //     )
    //     .to_vec();
    //
    //     let consensus_error = ConsensusError::deserialize(&error_bytes).with_js_error()?;
    //
    //     self.0.add_error(consensus_error);
    //
    //     Ok(JsValue::undefined())
    // }
}

impl ValidationResultWasm {
    pub fn add_error(&mut self, error: impl Into<ConsensusError>) {
        self.0.add_error(error)
    }
}
