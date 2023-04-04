use std::{collections::BTreeMap, sync::Arc};

use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;

use dpp::consensus::basic::value_error::ValueError;
use dpp::validation::{AsyncDataValidatorWithContext, SimpleConsensusValidationResult};
use dpp::{
    data_contract::state_transition::data_contract_update_transition::validation::{
        basic::{
            validate_indices_are_backward_compatible as dpp_validate_indices_are_backward_compatible,
            DataContractUpdateTransitionBasicValidator,
        },
        state::validate_data_contract_update_transition_state::validate_data_contract_update_transition_state as dpp_validate_data_contract_update_transition_state,
    },
    platform_value,
    version::ProtocolVersionValidator,
};
use wasm_bindgen::prelude::*;

use crate::utils::WithJsError;
use crate::{
    data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransitionParameters,
    errors::protocol_error::from_protocol_error,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    validation::ValidationResultWasm,
    DataContractUpdateTransitionWasm, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=validateDataContractUpdateTransitionState)]
pub async fn validate_data_contract_update_transition_state(
    state_repository: ExternalStateRepositoryLike,
    state_transition: &DataContractUpdateTransitionWasm,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    let result = dpp_validate_data_contract_update_transition_state(
        &wrapped_state_repository,
        &state_transition.to_owned().into(),
        &execution_context.to_owned().into(),
    )
    .await
    .with_js_error()?;

    Ok(result.map(|_| JsValue::undefined()).into())
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

    let result =
        dpp_validate_indices_are_backward_compatible(old_documents.iter(), new_documents.iter())
            .map_err(from_protocol_error)?;

    Ok(result.map(|_| JsValue::undefined()).into())
}

#[wasm_bindgen(js_name=validateDataContractUpdateTransitionBasic)]
pub async fn validate_data_contract_update_transition_basic(
    state_repository: ExternalStateRepositoryLike,
    raw_parameters: JsValue,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsError> {
    let parameters: DataContractUpdateTransitionParameters =
        serde_wasm_bindgen::from_value(raw_parameters)?;

    let mut value = platform_value::to_value(&parameters)?;
    let mut validation_result = SimpleConsensusValidationResult::default();
    if let Some(err) = DataContractUpdateTransition::clean_value(&mut value).err() {
        validation_result.add_error(ValueError::new(err));
        return Ok(validation_result.map(|_| JsValue::undefined()).into());
    }

    let validator: DataContractUpdateTransitionBasicValidator<ExternalStateRepositoryLikeWrapper> =
        DataContractUpdateTransitionBasicValidator::new(
            Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository)),
            Arc::new(ProtocolVersionValidator::default()),
        )?;

    validation_result.merge(
        validator
            .validate(&value, &execution_context.into())
            .await?,
    );

    Ok(validation_result.map(|_| JsValue::undefined()).into())
}
