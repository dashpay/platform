use js_sys::Array;
use js_sys::Error;
use js_sys::Reflect;
use wasm_bindgen::JsValue;

use crate::errors::consensus::consensus_error::from_consensus_error;

pub(crate) fn consensus_errors_to_js_error(
    consensus_errors: Vec<dpp::consensus::ConsensusError>,
) -> JsValue {
    let is_empty = consensus_errors.is_empty();
    let errors = Array::from_iter(consensus_errors.into_iter().map(from_consensus_error));
    let error = Error::new(if is_empty {
        "Protocol error contained an empty consensus error list"
    } else {
        "Multiple consensus errors"
    });
    error.set_name("ConsensusErrors");
    let error_value: JsValue = error.into();
    let _ = Reflect::set(&error_value, &JsValue::from_str("errors"), &errors.into());
    error_value
}

pub fn from_protocol_error(protocol_error: dpp::ProtocolError) -> JsValue {
    match protocol_error {
        dpp::ProtocolError::ConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        dpp::ProtocolError::ConsensusErrors(consensus_errors) => {
            consensus_errors_to_js_error(consensus_errors)
        }
        dpp::ProtocolError::Error(anyhow_error) => {
            format!("Non-protocol error: {}", anyhow_error).into()
        }
        e => format!("ProtocolError conversion not implemented: {}", e).into(),
    }
}
