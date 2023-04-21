use dpp::document::state_transition::documents_batch_transition::apply_documents_batch_transition_factory::{apply_documents_batch_transition};
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::WithJsError,
    DocumentsBatchTransitionWasm, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=applyDocumentsBatchTransition)]
pub async fn apply_documents_batch_transition_wasm(
    state_repository: ExternalStateRepositoryLike,
    transition: &DocumentsBatchTransitionWasm,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<(), JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);

    let result = apply_documents_batch_transition(
        &wrapped_state_repository,
        &transition.0,
        &execution_context.to_owned().into(),
    )
    .await
    .with_js_error();

    result
}
