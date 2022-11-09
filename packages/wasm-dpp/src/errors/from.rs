use wasm_bindgen::JsValue;

use dpp::errors::ProtocolError;

use crate::data_contract::errors::from_data_contract_to_js_error;
use crate::document::errors::from_document_to_js_error;

pub fn from_dpp_err(pe: ProtocolError) -> JsValue {
    match pe {
        ProtocolError::DataContractError(e) => from_data_contract_to_js_error(e),

        ProtocolError::Document(e) => from_document_to_js_error(*e),

        ProtocolError::ParsingJsonError(err) => format!(
            "Parsing error at line {}, column {}: {}",
            err.line(),
            err.column(),
            err
        )
        .into(),

        _ => JsValue::from_str(&*format!("Kek: {:#}", pe,)),
    }
}
