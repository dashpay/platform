use js_sys::Array;
use wasm_bindgen::JsValue;

use crate::errors::consensus::consensus_error::from_consensus_error;

pub fn from_protocol_error(protocol_error: dpp::ProtocolError) -> JsValue {
    match protocol_error {
        dpp::ProtocolError::ConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        dpp::ProtocolError::ConsensusErrors(consensus_errors) => {
            Array::from_iter(consensus_errors.into_iter().map(from_consensus_error)).into()
        }
        dpp::ProtocolError::Error(anyhow_error) => {
            format!("Non-protocol error: {}", anyhow_error).into()
        }
        e => format!("ProtocolError conversion not implemented: {}", e).into(),
    }
}
