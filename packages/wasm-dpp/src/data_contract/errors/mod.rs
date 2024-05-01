mod data_contract_generic_error;
mod invalid_data_contract;
mod invalid_document_type;

use wasm_bindgen::prelude::*;

pub use data_contract_generic_error::*;
use dpp::data_contract::errors::DataContractError;

pub fn from_data_contract_to_js_error(e: DataContractError) -> JsValue {
    match e {
        DataContractError::InvalidDocumentTypeError(err) => {
            invalid_document_type::InvalidDocumentTypeInDataContractError::new(
                err.document_type(),
                err.data_contract_id().into(),
            )
            .into()
        }
        other => DataContractGenericError::new(format!("data contract error: {}", other)).into(),
    }
}
