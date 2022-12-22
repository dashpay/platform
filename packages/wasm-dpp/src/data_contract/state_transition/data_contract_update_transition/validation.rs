use std::collections::BTreeMap;

use dpp::{
    data_contract::{
        state_transition::data_contract_update_transition::validation::{
            basic::validate_indices_are_backward_compatible as dpp_validate_indices_are_backward_compatible,
            state::validate_data_contract_update_transition_state::validate_data_contract_update_transition_state as dpp_validate_data_contract_update_transition_state,
        },
        DataContract,
    },
    document::Document,
};
use wasm_bindgen::prelude::*;

use crate::{
    errors::{from_dpp_err, protocol_error::from_protocol_error},
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    validation_result::ValidationResultWasm,
    DataContractUpdateTransitionWasm, DataContractWasm,
};

#[wasm_bindgen(js_name=validateDataContractUpdateTransitionState)]
pub async fn validate_data_contract_update_transition_state(
    state_repository: ExternalStateRepositoryLike,
    state_transition: DataContractUpdateTransitionWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    dpp_validate_data_contract_update_transition_state(
        &wrapped_state_repository,
        &state_transition.into(),
    )
    .await
    .map(Into::into)
    .map_err(from_dpp_err)
}

#[wasm_bindgen(js_name=validateIndicesAreBackwardCompatible)]
pub fn validate_indices_are_backward_compatible(
    old_documents_schema: JsValue,
    new_documents_schema: JsValue,
) -> Result<ValidationResultWasm, JsValue> {
    let old_documents = serde_wasm_bindgen::from_value::<BTreeMap<String, serde_json::Value>>(
        old_documents_schema,
    )?;
    let new_documents = serde_wasm_bindgen::from_value::<BTreeMap<String, serde_json::Value>>(
        new_documents_schema,
    )?;

    dpp_validate_indices_are_backward_compatible(old_documents.iter(), new_documents.iter())
        .map(Into::into)
        .map_err(from_protocol_error)
}
