use wasm_bindgen::{JsError, JsValue};

use super::consensus_error::from_consensus_error;

pub fn from_protocol_error(protocol_error: dpp::ProtocolError) -> JsValue {
    match protocol_error {
        dpp::ProtocolError::AbstractConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        dpp::ProtocolError::Error(anyhow_error) => {
            format!("Non-protocol error: {}", anyhow_error).into()
        }
        _ => todo!("Implement protocol error conversions"),
    }
}
