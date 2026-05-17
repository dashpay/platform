use dpp::consensus::ConsensusError;
use js_sys::{Array, Reflect};
use wasm_bindgen::{JsError, JsValue};

use crate::errors::consensus::consensus_error::from_consensus_error;

pub(crate) fn from_consensus_errors(consensus_errors: Vec<ConsensusError>) -> JsValue {
    let consensus_errors_array =
        Array::from_iter(consensus_errors.into_iter().map(from_consensus_error));
    let message = match consensus_errors_array.length() {
        0 => "ProtocolError contained no consensus errors".to_string(),
        1 => "ProtocolError contained 1 consensus error".to_string(),
        count => format!("ProtocolError contained {count} consensus errors"),
    };

    let error = JsError::new(&message);
    let error_value = JsValue::from(error);

    let _ = Reflect::set(
        &error_value,
        &JsValue::from_str("name"),
        &JsValue::from_str("ConsensusErrors"),
    );
    let _ = Reflect::set(
        &error_value,
        &JsValue::from_str("consensusErrors"),
        &consensus_errors_array.into(),
    );

    error_value
}

pub fn from_protocol_error(protocol_error: dpp::ProtocolError) -> JsValue {
    match protocol_error {
        dpp::ProtocolError::ConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        dpp::ProtocolError::ConsensusErrors(consensus_errors) => {
            from_consensus_errors(consensus_errors)
        }
        dpp::ProtocolError::Error(anyhow_error) => {
            format!("Non-protocol error: {}", anyhow_error).into()
        }
        e => format!("ProtocolError conversion not implemented: {}", e).into(),
    }
}
