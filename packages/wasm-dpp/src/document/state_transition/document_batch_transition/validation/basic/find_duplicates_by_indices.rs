use dpp::document::{
    document_transition::{document_base_transition, document_create_transition},
    validation::basic::find_duplicates_by_indices::find_duplicates_by_indices,
};
use itertools::Itertools;
use js_sys::Array;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::{
    document_batch_transition::document_transition::convert_to_object,
    utils::{replace_identifiers_with_bytes_without_failing, ToSerdeJSONExt, WithJsError},
    DataContractWasm,
};

#[wasm_bindgen(js_name=findDuplicatesByIndices)]
pub fn find_duplicates_by_indices_wasm(
    js_raw_transitions: &Array,
    data_contract: &DataContractWasm,
) -> Result<Vec<JsValue>, JsValue> {
    let raw_transitions: Vec<Value> = js_raw_transitions
        .iter()
        .map(|t| {
            t.with_serde_to_json_value().map(|mut v| {
                replace_identifiers_with_bytes_without_failing(
                    &mut v,
                    document_base_transition::IDENTIFIER_FIELDS,
                );
                v
            })
        })
        .try_collect()?;

    let result =
        find_duplicates_by_indices(&raw_transitions, data_contract.inner()).with_js_error()?;

    let duplicates: Vec<JsValue> = result
        .into_iter()
        .map(|v| {
            convert_to_object(
                v.to_owned(),
                &JsValue::NULL,
                document_base_transition::IDENTIFIER_FIELDS,
                document_create_transition::BINARY_FIELDS,
            )
        })
        .try_collect()?;

    Ok(duplicates)
}
