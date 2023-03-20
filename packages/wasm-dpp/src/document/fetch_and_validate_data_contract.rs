use dpp::{
    document::{self, fetch_and_validate_data_contract::fetch_and_validate_data_contract},
    prelude::DataContract,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    validation::ValidationResult,
};
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::{replace_identifiers_with_bytes_without_failing, ToSerdeJSONExt, WithJsError},
    validation::ValidationResultWasm,
    DataContractWasm,
};

#[wasm_bindgen(js_name=FetchAndValidateDataContractFactory)]
pub struct DataContractFetcherAndValidatorWasm {
    state_repository: ExternalStateRepositoryLikeWrapper,
}

impl DataContractFetcherAndValidatorWasm {
    pub(crate) fn new_with_state_repository_wrapper(
        state_repository: ExternalStateRepositoryLikeWrapper,
    ) -> Self {
        Self { state_repository }
    }
}

#[wasm_bindgen(js_class=FetchAndValidateDataContractFactory)]
impl DataContractFetcherAndValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(state_repository: ExternalStateRepositoryLike) -> Self {
        let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
        Self {
            state_repository: wrapped_state_repository,
        }
    }

    #[wasm_bindgen(js_name=validate)]
    pub async fn validate(
        &self,
        js_raw_document: &JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        fetch_and_validate_data_contract_inner(&self.state_repository, js_raw_document).await
    }
}

#[wasm_bindgen(js_name = fetchAndValidateDataContract)]
pub async fn fetch_and_validate_data_contract_wasm(
    state_repository: ExternalStateRepositoryLike,
    js_raw_document: JsValue,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    fetch_and_validate_data_contract_inner(&wrapped_state_repository, &js_raw_document).await
}

async fn fetch_and_validate_data_contract_inner(
    state_repository: &ExternalStateRepositoryLikeWrapper,
    js_raw_document: &JsValue,
) -> Result<ValidationResultWasm, JsValue> {
    let mut document_value = js_raw_document.with_serde_to_json_value()?;
    replace_identifiers_with_bytes_without_failing(
        &mut document_value,
        document::IDENTIFIER_FIELDS,
    );

    // TODO! remove the context. The the providing the context in state repository should be optional
    let ctx = StateTransitionExecutionContext::default();
    let validation_result =
        fetch_and_validate_data_contract(state_repository, &document_value, &ctx)
            .await
            .with_js_error()?;
    let result_with_js_value: ValidationResult<JsValue> = validation_result
        .map(|dc| <DataContractWasm as std::convert::From<DataContract>>::from(dc).into());

    Ok(result_with_js_value.into())
}
