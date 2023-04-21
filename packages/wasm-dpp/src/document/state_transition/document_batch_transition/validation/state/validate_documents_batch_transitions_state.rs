use dpp::document::validation::state::validate_documents_batch_transition_state;
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::WithJsError,
    validation::ValidationResultWasm,
    DocumentsBatchTransitionWasm, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name = "validateDocumentsBatchTransitionState")]
pub async fn validate_documents_batch_transition_state_wasm(
    state_repository: ExternalStateRepositoryLike,
    state_transition: &DocumentsBatchTransitionWasm,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);

    let validation_result =
        validate_documents_batch_transition_state::validate_document_batch_transition_state(
            &wrapped_state_repository,
            &state_transition.0,
            &execution_context.to_owned().into(),
        )
        .await
        .with_js_error()?;

    Ok(validation_result.map(|_| JsValue::undefined()).into())
}
