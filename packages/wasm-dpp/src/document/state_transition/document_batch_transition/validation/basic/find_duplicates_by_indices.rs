use dpp::document::{
    document_transition::{document_base_transition, document_create_transition},
    validation::basic::find_duplicates_by_indices::find_duplicates_by_indices,
};
use dpp::platform_value::{ReplacementType, Value};
use itertools::Itertools;
use js_sys::Array;
use wasm_bindgen::prelude::*;

use crate::{
    document_batch_transition::document_transition::to_object,
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
        .map(|transition| {
            let mut value = transition.with_serde_to_platform_value()?;
            value.replace_at_paths(
                document_base_transition::IDENTIFIER_FIELDS,
                ReplacementType::Identifier,
            );
            Ok(value)
        })
        .collect::<Result<Vec<Value>, JsValue>>()?;

    let result =
        find_duplicates_by_indices(&raw_transitions, data_contract.inner()).with_js_error()?;

    let duplicates: Vec<JsValue> = result
        .into_iter()
        .map(|v| {
            to_object(
                v.to_owned().into(),
                &JsValue::NULL,
                document_base_transition::IDENTIFIER_FIELDS,
                document_create_transition::BINARY_FIELDS,
            )
        })
        .try_collect()?;

    Ok(duplicates)
}
