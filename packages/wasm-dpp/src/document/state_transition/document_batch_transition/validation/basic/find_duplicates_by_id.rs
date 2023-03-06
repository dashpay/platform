use dpp::document::document_transition::{document_base_transition, document_create_transition};
use dpp::document::validation::basic::find_duplicates_by_id::find_duplicates_by_id;
use dpp::platform_value::btreemap_field_replacement::BTreeValueMapReplacementPathHelper;
use dpp::platform_value::{ReplacementType, Value};
use dpp::ProtocolError;
use itertools::Itertools;
use js_sys::Array;
use serde_json::Value as JsonValue;
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::document_batch_transition::document_transition::to_object;
use crate::utils::{replace_identifiers_with_bytes_without_failing, ToSerdeJSONExt, WithJsError};

#[wasm_bindgen(js_name=findDuplicatesById)]
pub fn find_duplicates_by_id_wasm(js_raw_transitions: Array) -> Result<Vec<JsValue>, JsValue> {
    let raw_transitions: Vec<JsonValue> = js_raw_transitions
        .iter()
        .map(|transition| {
            let mut map = transition.with_serde_to_platform_value_map()?;
            map.replace_at_paths(
                document_base_transition::IDENTIFIER_FIELDS.map(|a| a.to_string()),
                ReplacementType::Identifier,
            );
            let value: Value = map.into();
            value
                .try_into_validating_json()
                .map_err(ProtocolError::ValueError)
                .with_js_error()
        })
        .collect::<Result<Vec<JsonValue>, JsValue>>()?;

    let result: Vec<JsValue> = find_duplicates_by_id(&raw_transitions)
        .with_js_error()?
        .into_iter()
        .map(|raw| {
            to_object(
                raw.into(),
                &JsValue::null(),
                document_base_transition::IDENTIFIER_FIELDS,
                document_create_transition::BINARY_FIELDS,
            )
        })
        .try_collect()?;

    Ok(result)
}
