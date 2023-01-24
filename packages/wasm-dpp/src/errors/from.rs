use wasm_bindgen::JsValue;

use dpp::errors::ProtocolError;

use crate::data_contract::errors::from_data_contract_to_js_error;
use crate::document::errors::from_document_to_js_error;

use super::consensus::basic::document::DataContractNotPresentErrorWasm;
use super::consensus_error::from_consensus_error;
use super::data_contract_not_present_error::DataContractNotPresentNotConsensusErrorWasm;

pub fn from_dpp_err(pe: ProtocolError) -> JsValue {
    match pe {
        ProtocolError::AbstractConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        ProtocolError::DataContractError(e) => from_data_contract_to_js_error(e),

        ProtocolError::Document(e) => from_document_to_js_error(*e),

        ProtocolError::ParsingJsonError(err) => format!(
            "Parsing error at line {}, column {}: {}",
            err.line(),
            err.column(),
            err
        )
        .into(),

        ProtocolError::DataContractNotPresentError { data_contract_id } => {
            DataContractNotPresentNotConsensusErrorWasm::new(data_contract_id).into()
        }
        _ => JsValue::from_str(&format!("Error conversion not implemented: {pe:#}",)),
    }
}
