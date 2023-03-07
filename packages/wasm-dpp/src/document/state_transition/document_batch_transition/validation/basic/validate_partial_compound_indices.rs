use dpp::document::{
    document_transition::document_base_transition,
    validation::basic::validate_partial_compound_indices::validate_partial_compound_indices,
};
use itertools::Itertools;
use js_sys::Array;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::{
    utils::{replace_identifiers_with_bytes_without_failing, ToSerdeJSONExt, WithJsError},
    validation::ValidationResultWasm,
    DataContractWasm,
};

#[wasm_bindgen(js_name=validatePartialCompoundIndices)]
pub fn validate_partial_compound_indices_wasm(
    js_raw_transitions: Array,
    data_contract: &DataContractWasm,
) -> Result<ValidationResultWasm, JsValue> {
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

    let validation_result =
        validate_partial_compound_indices(&raw_transitions, data_contract.inner())
            .with_js_error()?;

    Ok(validation_result.map(|_| JsValue::undefined()).into())
}
