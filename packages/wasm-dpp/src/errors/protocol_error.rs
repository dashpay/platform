use crate::data_contract::errors::InvalidDataContractError;
use wasm_bindgen::JsValue;

use crate::errors::consensus::consensus_error::from_consensus_error;

pub fn from_protocol_error(protocol_error: dpp::ProtocolError) -> JsValue {
    match protocol_error {
        dpp::ProtocolError::ConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        dpp::ProtocolError::InvalidDataContractError(err) => {
            let raw_data_contract = err.raw_data_contract();
            let protocol = serde_wasm_bindgen::to_value(&raw_data_contract);
            protocol.map_or_else(JsValue::from, |raw_contract| {
                InvalidDataContractError::new(err.errors, raw_contract).into()
            })
        }
        dpp::ProtocolError::Error(anyhow_error) => {
            format!("Non-protocol error: {}", anyhow_error).into()
        }
        e => format!("ProtocolError conversion not implemented: {}", e).into(),
    }
}
