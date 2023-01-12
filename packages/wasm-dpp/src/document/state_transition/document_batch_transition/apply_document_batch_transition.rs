use dpp::document::state_transition::documents_batch_transition::apply_documents_batch_transition_factory::{self, apply_documents_batch_transition};
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::WithJsError,
    DocumentsBatchTransitionWASM,
};

#[wasm_bindgen(js_name=applyDocumentsBatchTransition)]
pub async fn apply_documents_batch_transition_wasm(
    state_repository: ExternalStateRepositoryLike,
    transition: &DocumentsBatchTransitionWASM,
) -> Result<(), JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);

    let result = apply_documents_batch_transition(&wrapped_state_repository, &transition.0)
        .await
        .with_js_error();

    result
}
