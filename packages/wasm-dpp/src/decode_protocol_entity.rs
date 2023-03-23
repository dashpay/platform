use crate::errors::protocol_error::from_protocol_error;
use core::iter::FromIterator;
use dpp::decode_protocol_entity_factory::DecodeProtocolEntity;
use js_sys::Array;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=decodeProtocolEntity)]
pub fn decode_protocol_entity(buffer: Vec<u8>) -> Result<Array, JsValue> {
  DecodeProtocolEntity::decode_protocol_entity(buffer)
    .map(|(protocol_version, json_value)| {
      let serializer = serde_wasm_bindgen::Serializer::json_compatible();
      let js_value = json_value
        .serialize(&serializer)
        .expect("implements Serialize");
      Array::from_iter(vec![JsValue::from(protocol_version), js_value])
    })
    .map_err(from_protocol_error)
