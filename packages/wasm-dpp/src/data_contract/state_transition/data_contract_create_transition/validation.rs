use std::sync::Arc;

use dpp::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;

use dpp::consensus::basic::value_error::ValueError;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::{
    data_contract::state_transition::data_contract_create_transition::validation::state::{
        validate_data_contract_create_transition_basic::DataContractCreateTransitionBasicValidator,
        validate_data_contract_create_transition_state::validate_data_contract_create_transition_state as dpp_validate_data_contract_create_transition_state,
    },
    platform_value,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    validation::DataValidatorWithContext,
    version::ProtocolVersionValidator,
};
use wasm_bindgen::prelude::*;

use crate::utils::WithJsError;
use crate::validation::ValidationResultWasm;
use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    DataContractCreateTransitionWasm, StateTransitionExecutionContextWasm,
};

use super::DataContractCreateTransitionParameters;

#[wasm_bindgen(js_name=validateDataContractCreateTransitionState)]
pub async fn validate_data_contract_create_transition_state(
    state_repository: ExternalStateRepositoryLike,
    state_transition: DataContractCreateTransitionWasm,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    let validation_result = dpp_validate_data_contract_create_transition_state(
        &wrapped_state_repository,
        &state_transition.into(),
        &execution_context.to_owned().into(),
    )
    .await
    .with_js_error()?;
    Ok(validation_result.map(|_| JsValue::undefined()).into())
}

#[wasm_bindgen(js_name=validateDataContractCreateTransitionBasic)]
pub async fn validate_data_contract_create_transition_basic(
    raw_parameters: JsValue,
) -> Result<ValidationResultWasm, JsError> {
    let parameters: DataContractCreateTransitionParameters =
        serde_wasm_bindgen::from_value(raw_parameters)?;

    let mut value = platform_value::to_value(&parameters)?;
    let mut validation_result = SimpleConsensusValidationResult::default();
    if let Some(err) = DataContractCreateTransition::clean_value(&mut value).err() {
        validation_result.add_error(ValueError::new(err));
        return Ok(validation_result.map(|_| JsValue::undefined()).into());
    }

    let validator = DataContractCreateTransitionBasicValidator::new(Arc::new(
        ProtocolVersionValidator::default(),
    ))?;

    validation_result
        .merge(validator.validate(&value, &StateTransitionExecutionContext::default())?);

    Ok(validation_result.map(|_| JsValue::undefined()).into())
}
