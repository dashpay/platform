use dpp::{
    document::{self, fetch_and_validate_data_contract::fetch_and_validate_data_contract},
    prelude::DataContract,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    util::json_value::{JsonValueExt, ReplaceWith},
    validation::ValidationResult,
};
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::{ToSerdeJSONExt, WithJsError},
    validation::ValidationResultWasm,
    DataContractWasm,
};

#[wasm_bindgen(js_name = fetchAndValidateDataContract)]
pub async fn fetch_and_validate_data_contract_wasm(
    state_repository: ExternalStateRepositoryLike,
    js_raw_document: JsValue,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    let mut document_value = js_raw_document.with_serde_to_json_value()?;

    // Allow to fail as identifier can be type of Buffer  or Identifier
    // We don't need to replace dynamic values because the function doesn't use them
    let _ =
        document_value.replace_identifier_paths(document::IDENTIFIER_FIELDS, ReplaceWith::Bytes);

    // TODO! remove the context. The the providing the context in state repository should be optional
    let ctx = StateTransitionExecutionContext::default();
    let validation_result =
        fetch_and_validate_data_contract(&wrapped_state_repository, &document_value, &ctx)
            .await
            .with_js_error()?;
    let result_with_js_value: ValidationResult<JsValue> = validation_result
        .map(|dc| <DataContractWasm as std::convert::From<DataContract>>::from(dc).into());

    Ok(result_with_js_value.into())
}
