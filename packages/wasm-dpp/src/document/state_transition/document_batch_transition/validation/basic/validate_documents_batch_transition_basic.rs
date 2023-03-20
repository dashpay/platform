use dpp::document::validation::basic::validate_documents_batch_transition_basic;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::{ToSerdeJSONExt, WithJsError},
    validation::ValidationResultWasm,
    version::ProtocolVersionValidatorWasm,
    StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=validateDocumentsBatchTransitionBasic)]
pub async fn validate_documents_batch_transition_basic_wasm(
    protocol_version_validator: ProtocolVersionValidatorWasm,
    state_repository: ExternalStateRepositoryLike,
    js_raw_state_transition: JsValue,
    execution_context: StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    let raw_state_transition = js_raw_state_transition.with_serde_to_json_value()?;

    let validation_result =
        validate_documents_batch_transition_basic::validate_documents_batch_transition_basic(
            &protocol_version_validator.into(),
            &raw_state_transition,
            Arc::new(wrapped_state_repository),
            &execution_context.into(),
        )
        .await
        .with_js_error()?;

    Ok(validation_result.map(|_| JsValue::undefined()).into())
}
