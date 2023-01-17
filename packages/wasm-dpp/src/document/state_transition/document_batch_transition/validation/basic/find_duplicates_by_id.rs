use dpp::document::document_transition::{document_base_transition, document_create_transition};
use dpp::document::validation::basic::find_duplicates_by_id::find_duplicates_by_id;
use dpp::util::json_value::{JsonValueExt, ReplaceWith};
use itertools::Itertools;
use js_sys::Array;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::document_batch_transition::document_transition::convert_to_object;
use crate::utils::{ToSerdeJSONExt, WithJsError};

#[wasm_bindgen(js_name=findDuplicatesById)]
pub fn find_duplicates_by_id_wasm(js_raw_transitions: Array) -> Result<Vec<JsValue>, JsValue> {
    let raw_transitions: Vec<Value> = js_raw_transitions
        .iter()
        .map(|t| {
            t.with_serde_to_json_value().map(|mut v| {
                let _ = v.replace_identifier_paths(
                    document_base_transition::IDENTIFIER_FIELDS,
                    ReplaceWith::Bytes,
                );
                v
            })
        })
        .try_collect()?;

    let result: Vec<JsValue> = find_duplicates_by_id(&raw_transitions)
        .with_js_error()?
        .into_iter()
        .map(|raw| {
            convert_to_object(
                raw,
                &JsValue::null(),
                document_base_transition::IDENTIFIER_FIELDS,
                document_create_transition::BINARY_FIELDS,
            )
        })
        .try_collect()?;

    Ok(result)
}
