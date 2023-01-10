use wasm_bindgen::{JsError, JsValue};

use super::consensus_error::from_consensus_error;

pub fn from_protocol_error(protocol_error: dpp::ProtocolError) -> JsValue {
    match protocol_error {
        dpp::ProtocolError::AbstractConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        e => JsError::new(&format!("Protocol error: {e}")).into(),
    }
}
