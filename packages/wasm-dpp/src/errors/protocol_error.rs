use wasm_bindgen::JsValue;

use super::consensus_error::from_consensus_error;

pub fn from_protocol_error(e: dpp::ProtocolError) -> JsValue {
    web_sys::console::log_1(&format!("{e:?}").into());
    match e {
        dpp::ProtocolError::AbstractConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        _ => unimplemented!(),
    }
}
