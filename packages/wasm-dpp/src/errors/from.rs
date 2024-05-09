use dpp::DashPlatformProtocolInitError;
use wasm_bindgen::JsValue;

use dpp::errors::ProtocolError;

use crate::data_contract::errors::from_data_contract_to_js_error;
use crate::document::errors::from_document_to_js_error;
// use crate::document::errors::from_document_to_js_error;
use crate::errors::value_error::PlatformValueErrorWasm;

use super::data_contract_not_present_error::DataContractNotPresentNotConsensusErrorWasm;
use crate::errors::consensus::consensus_error::from_consensus_error;

pub fn from_dpp_err(pe: ProtocolError) -> JsValue {
    match pe {
        // TODO(versioning): restore this
        ProtocolError::ConsensusError(consensus_error) => from_consensus_error(*consensus_error),
        ProtocolError::DataContractError(e) => from_data_contract_to_js_error(e),

        ProtocolError::Document(e) => from_document_to_js_error(*e),

        // ProtocolError::ParsingJsonError(err) => format!(
        //     "Parsing error at line {}, column {}: {}",
        //     err.line(),
        //     err.column(),
        //     err
        // )
        // .into(),
        //
        ProtocolError::DataContractNotPresentError(err) => {
            DataContractNotPresentNotConsensusErrorWasm::new(err.data_contract_id()).into()
        }
        ProtocolError::ValueError(value_error) => PlatformValueErrorWasm::from(value_error).into(),
        _ => JsValue::from_str(&format!("Error conversion not implemented: {pe:#}",)),
    }
}

pub fn from_dpp_init_error(e: DashPlatformProtocolInitError) -> JsValue {
    match e {
        DashPlatformProtocolInitError::SchemaDeserializationError(e) => e.to_string().into(),
        DashPlatformProtocolInitError::InvalidSchemaError(e) => e.to_string().into(),
        // TODO(versioning): add rest erros
        _ => JsValue::from_str(&format!("Error conversion not implemented: {e:#}",)),
    }
}
