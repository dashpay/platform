use dpp::{
    document::validation::state::fetch_documents, prelude::DocumentTransition,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
};
use js_sys::Array;
use wasm_bindgen::prelude::*;

use crate::{
    document_batch_transition::document_transition::DocumentTransitionWasm,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::{IntoWasm, WithJsError},
    DocumentWasm, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name = fetchDocuments)]
pub async fn fetch_documents_wasm(
    state_repository: ExternalStateRepositoryLike,
    js_document_transitions: Array,
    js_execution_context: &StateTransitionExecutionContextWasm,
) -> Result<Array, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    let mut document_transitions: Vec<DocumentTransition> = vec![];
    for js_transition in js_document_transitions.iter() {
        let transition = js_transition.to_wasm::<DocumentTransitionWasm>("DocumentTransition")?;
        document_transitions.push(transition.to_owned().into());
    }
    let execution_context: StateTransitionExecutionContext = js_execution_context.into();

    let documents = fetch_documents::fetch_documents(
        &wrapped_state_repository,
        document_transitions,
        &execution_context,
    )
    .await
    .with_js_error()?;

    let array = js_sys::Array::new();
    for document in documents.into_iter().map(DocumentWasm::from) {
        array.push(&document.into());
    }

    Ok(array)
}
