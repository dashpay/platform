use crate::errors::consensus_error::from_consensus_error_ref;
use dpp::identity::state_transition::asset_lock_proof::PublicKeyHash;
use dpp::validation::ValidationResult;
use js_sys::JsString;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct EmptyValidationData;

impl From<()> for EmptyValidationData {
    fn from(_: ()) -> Self {
        EmptyValidationData
    }
}

#[wasm_bindgen(js_name=ValidationResult)]
pub struct ValidationResultWasm(ValidationResult<JsValue>);

impl<T> From<ValidationResult<T>> for ValidationResultWasm
where
    T: Into<JsValue> + Clone,
    // ValidationResult<JsValue>: From<ValidationResult<T>>,
{
    fn from(validation_result: ValidationResult<T>) -> Self {
        ValidationResultWasm(validation_result.into())
    }
}

// impl From<ValidationResult<()>> for ValidationResultWasm {
//     fn from(validation_result: ValidationResult<()>) {
//         // ValidationResultWasm(validation_result)
//     }
// }

#[wasm_bindgen(js_class=ValidationResult)]
impl ValidationResultWasm {
    /// This is just a test method - doesn't need to be in the resulted binding. Please
    /// remove before shipping
    #[wasm_bindgen(js_name=errorsText)]
    pub fn errors_text(&self) -> Vec<JsString> {
        self.0
            .errors()
            .iter()
            .map(|e| e.to_string().into())
            .collect()
    }

    #[wasm_bindgen(js_name=isValid)]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    #[wasm_bindgen(js_name=getErrors)]
    pub fn errors(&self) -> Vec<JsValue> {
        self.0
            .errors()
            .iter()
            .map(from_consensus_error_ref)
            .collect()
    }
}
