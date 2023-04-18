use crate::errors::consensus::consensus_error::from_consensus_error_ref;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDataContractError)]
#[derive(Debug)]
pub struct InvalidDataContractError {
    // we have to store it as JsValue as the errors of 'class' Consensus are of different types
    errors: Vec<ConsensusError>,
    raw_data_contract: JsValue,
}

impl InvalidDataContractError {
    pub fn new(errors: Vec<ConsensusError>, raw_data_contract: JsValue) -> Self {
        InvalidDataContractError {
            errors,
            raw_data_contract,
        }
    }
}

#[wasm_bindgen(js_class=InvalidDataContractError)]
impl InvalidDataContractError {
    #[wasm_bindgen(js_name=getErrors)]
    pub fn get_errors(&self) -> Vec<JsValue> {
        self.errors.iter().map(from_consensus_error_ref).collect()
    }

    #[wasm_bindgen(js_name=getRawDataContract)]
    pub fn get_raw_data_contract(&self) -> JsValue {
        self.raw_data_contract.clone()
    }

    #[wasm_bindgen(js_name=getMessage)]
    pub fn get_message(&self) -> String {
        let extended_message =
            if let Some(error_message) = self.errors.first().map(|e| e.to_string()) {
                let narrowed_message = if self.errors.len() > 1 {
                    format!(" and {} more", self.errors.len() - 1)
                } else {
                    "".to_owned()
                };
                format!(": \"{error_message}\"{narrowed_message}")
            } else {
                "".to_owned()
            };

        format!("Data contract decode error{extended_message}")
    }
}
