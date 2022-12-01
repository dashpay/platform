use thiserror::Error;
use wasm_bindgen::prelude::*;

use crate::DataContractWasm;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Data Contract")]
pub struct InvalidDataContractError {
    // we have to store it as JsValue as the errors of 'class' Consensus are of different types
    errors: Vec<JsValue>,
    raw_data_contract: DataContractWasm,
}

#[wasm_bindgen]
impl InvalidDataContractError {
    #[wasm_bindgen(constructor)]
    pub fn new(errors: Vec<JsValue>, raw_data_contract: DataContractWasm) -> Self {
        InvalidDataContractError {
            errors,
            raw_data_contract,
        }
    }
    #[wasm_bindgen]
    pub fn get_errors(&self) -> Vec<JsValue> {
        self.errors.clone().into_iter().map(JsValue::from).collect()
    }

    #[wasm_bindgen]
    pub fn get_data_contract(&self) -> DataContractWasm {
        self.raw_data_contract.clone()
    }
}
