use wasm_bindgen::JsValue;

use super::consensus_error::from_consensus_error;

pub fn from_protocol_error(e: dpp::ProtocolError) -> JsValue {
    match e {
        dpp::ProtocolError::AbstractConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        _ => unimplemented!(),
    }
}
