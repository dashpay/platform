use dpp::data_trigger::DataTriggerExecutionResult;
use js_sys::Array;
use wasm_bindgen::prelude::*;

use crate::errors::consensus::consensus_error::from_state_error;
use crate::utils::Inner;

#[wasm_bindgen(js_name=DataTriggerExecutionResult)]
pub struct DataTriggerExecutionResultWasm(DataTriggerExecutionResult);

impl From<DataTriggerExecutionResult> for DataTriggerExecutionResultWasm {
    fn from(value: DataTriggerExecutionResult) -> Self {
        DataTriggerExecutionResultWasm(value)
    }
}

#[wasm_bindgen(js_class=DataTriggerExecutionResult)]
impl DataTriggerExecutionResultWasm {
    #[wasm_bindgen(js_name=isOk)]
    pub fn is_ok(&self) -> bool {
        self.0.errors.is_empty()
    }

    #[wasm_bindgen(js_name=getErrors)]
    pub fn get_errors(&self) -> Array {
        let errors = self.0.get_errors().iter().map(from_state_error);
        let array_with_errors = Array::new();
        for error in errors {
            array_with_errors.push(&error);
        }
        array_with_errors
    }
}

impl Inner for DataTriggerExecutionResultWasm {
    type InnerItem = DataTriggerExecutionResult;

    fn into_inner(self) -> Self::InnerItem {
        self.0
    }

    fn inner(&self) -> &Self::InnerItem {
        &self.0
    }

    fn inner_mut(&mut self) -> &mut Self::InnerItem {
        &mut self.0
    }
}
