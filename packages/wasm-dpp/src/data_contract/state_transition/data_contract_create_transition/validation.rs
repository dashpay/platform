use dpp::data_contract::state_transition::data_contract_create_transition::validation::state::validate_data_contract_create_transition_state::validate_data_contract_create_transition_state as dpp_validate_data_contract_create_transition_state;
use wasm_bindgen::prelude::*;

use crate::{
    errors::from_dpp_err,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    validation_result::ValidationResultWasm,
    DataContractCreateTransitionWasm,
};
use crate::validation::ValidationResultWasm;

#[wasm_bindgen(js_name=validateDataContractCreateTransitionState)]
pub async fn validate_data_contract_create_transition_state(
    state_repository: ExternalStateRepositoryLike,
    state_transition: DataContractCreateTransitionWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    dpp_validate_data_contract_create_transition_state(
        &wrapped_state_repository,
        &state_transition.into(),
    )
    .await
    .map(Into::<ValidationResultWasm>::into)
    .map_err(from_dpp_err)
}
