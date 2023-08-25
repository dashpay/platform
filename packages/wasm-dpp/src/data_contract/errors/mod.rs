mod data_contract_generic_error;
mod invalid_data_contract;
mod invalid_document_type;

use wasm_bindgen::prelude::*;

pub use data_contract_generic_error::*;
use dpp::data_contract::errors::DataContractError;
use dpp::ProtocolError;
pub use invalid_data_contract::*;

pub fn from_data_contract_to_js_error(e: DataContractError) -> JsValue {
    match e {
        DataContractError::InvalidDataContractError {
            errors,
            raw_data_contract,
        } => InvalidDataContractError::new(
            errors,
            serde_wasm_bindgen::to_value(&raw_data_contract)
                .expect("statically known structure should be a valid JSON"),
        )
        .into(),
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
