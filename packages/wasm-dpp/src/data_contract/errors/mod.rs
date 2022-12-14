mod data_contract_already_exists;
mod invalid_data_contract;
mod invalid_document_type;

use wasm_bindgen::prelude::*;

use crate::errors::consensus_error::from_consensus_error;
pub use data_contract_already_exists::*;
use dpp::data_contract::errors::DataContractError;
pub use invalid_data_contract::*;

pub fn from_data_contract_to_js_error(e: DataContractError) -> JsValue {
    match e {
        DataContractError::InvalidDataContractError {
            errors,
            raw_data_contract,
        } => {
            let js_errors = errors.into_iter().map(from_consensus_error).collect();

            InvalidDataContractError::new(js_errors, raw_data_contract.into()).into()
        }
        DataContractError::InvalidDocumentTypeError {
            doc_type,
            data_contract,
        } => invalid_document_type::InvalidDocumentTypeInDataContractError::new(
            doc_type,
            data_contract.into(),
        )
        .into(),
        _ => todo!(),
    }
}
