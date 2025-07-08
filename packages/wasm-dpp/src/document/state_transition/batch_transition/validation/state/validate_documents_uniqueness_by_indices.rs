use dpp::{
    document::validation::state::validate_documents_uniqueness_by_indices,
    prelude::{DocumentTransition, Identifier},
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
};
use js_sys::Array;
use wasm_bindgen::prelude::*;

use crate::{
    document_batch_transition::document_transition::DocumentTransitionWasm,
    identifier::IdentifierWrapper,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::{IntoWasm, WithJsError},
    validation::ValidationResultWasm,
    DataContractWasm, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=validateDocumentsUniquenessByIndices)]
pub async fn validate_uniqueness_by_indices_wasm(
    state_repository: ExternalStateRepositoryLike,
    js_owner_id: &IdentifierWrapper,
    js_document_transitions: Array,
    js_data_contract: &DataContractWasm,
    js_execution_context: &StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    let mut document_transitions: Vec<DocumentTransition> = vec![];
    for js_transition in js_document_transitions.iter() {
        let transition = js_transition.to_wasm::<DocumentTransitionWasm>("DocumentTransition")?;
        document_transitions.push(transition.to_owned().into());
    }
    let execution_context: StateTransitionExecutionContext = js_execution_context.into();
    let owner_id: Identifier = js_owner_id.into();

    let validation_result =
        validate_documents_uniqueness_by_indices::validate_documents_uniqueness_by_indices(
            &wrapped_state_repository,
            &owner_id,
            document_transitions.iter(),
            js_data_contract.inner(),
            &execution_context,
        )
        .await
        .with_js_error()?;

    Ok(validation_result.map(|_| JsValue::undefined()).into())
}
