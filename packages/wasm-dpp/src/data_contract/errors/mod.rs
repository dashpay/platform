use wasm_bindgen::prelude::*;

pub use data_contract_already_exists::*;
use dpp::data_contract::errors::DataContractError;
pub use invalid_data_contract::*;

use crate::mocks;

mod data_contract_already_exists;
mod invalid_data_contract;

pub fn from_data_contract_to_js_error(e: DataContractError) -> JsValue {
    match e {
        DataContractError::InvalidDataContractError {
            errors,
            raw_data_contract,
        } => {
            let js_errors = errors
                .into_iter()
                .map(mocks::from_consensus_to_js_error)
                .collect();

            InvalidDataContractError::new(js_errors, raw_data_contract.into()).into()
        }
        _ => unimplemented!(),
    }
}
