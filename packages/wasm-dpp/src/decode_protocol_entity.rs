use crate::errors::protocol_error::from_protocol_error;
use core::iter::FromIterator;
use dpp::encoding::decode_protocol_entity_factory::DecodeProtocolEntity;
use dpp::ProtocolError;
use js_sys::Array;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=decodeProtocolEntity)]
pub fn decode_protocol_entity(buffer: Vec<u8>) -> Result<Array, JsValue> {
    let (protocol_version, value) = DecodeProtocolEntity::decode_protocol_entity_to_value(buffer)
        .map_err(from_protocol_error)?;

    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    let js_value = value
        .try_to_validating_json()
        .map_err(|e| from_protocol_error(ProtocolError::ValueError(e)))?
        .serialize(&serializer)
        .map_err(|e| from_protocol_error(ProtocolError::EncodingError(e.to_string())))?;
    Ok(Array::from_iter(vec![
        JsValue::from(protocol_version),
        js_value,
    ]))
}
