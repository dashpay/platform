use crate::data_contract::errors::InvalidDataContractError;
use wasm_bindgen::JsValue;

use super::consensus_error::from_consensus_error;

pub fn from_protocol_error(protocol_error: dpp::ProtocolError) -> JsValue {
    match protocol_error {
        dpp::ProtocolError::AbstractConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        dpp::ProtocolError::InvalidDataContractError {
            errors,
            raw_data_contract,
        } => {
            let protocol = serde_wasm_bindgen::to_value(&raw_data_contract);
            protocol.map_or_else(JsValue::from, |raw_contract| {
                InvalidDataContractError::new(errors, raw_contract).into()
            })
        }
        dpp::ProtocolError::Error(anyhow_error) => {
            format!("Non-protocol error: {}", anyhow_error).into()
        }
        _ => todo!("Implement protocol error conversions"),
    }
}
