use dpp::consensus::basic::value_error::ValueError;
use dpp::document::validation::basic::validate_documents_batch_transition_basic;
use dpp::document::DocumentsBatchTransition;
use dpp::validation::SimpleConsensusValidationResult;
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
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    let mut value = js_raw_state_transition.with_serde_to_platform_value()?;

    let mut validation_result = SimpleConsensusValidationResult::default();
    if let Some(err) = DocumentsBatchTransition::clean_value(&mut value).err() {
        validation_result.add_error(ValueError::new(err));
        return Ok(validation_result.map(|_| JsValue::undefined()).into());
    }

    validation_result.merge(
        validate_documents_batch_transition_basic::validate_documents_batch_transition_basic(
            &protocol_version_validator.into(),
            &value,
            Arc::new(wrapped_state_repository),
            &execution_context.to_owned().into(),
        )
        .await
        .with_js_error()?,
    );

    Ok(validation_result.map(|_| JsValue::undefined()).into())
}
