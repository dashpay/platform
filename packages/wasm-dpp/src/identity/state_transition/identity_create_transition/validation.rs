use dpp::identity::state_transition::identity_create_transition::validation::state::validate_identity_create_transition_state;
use wasm_bindgen::prelude::*;

use crate::{
    errors::from_dpp_err,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    validation::ValidationResultWasm,
    IdentityCreateTransitionWasm,
};

#[wasm_bindgen(js_name=validateIdentityCreateTransitionState)]
pub async fn validate_identity_create_transition_state_wasm(
    state_repository: ExternalStateRepositoryLike,
    state_transition: &IdentityCreateTransitionWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    validate_identity_create_transition_state(
        &wrapped_state_repository,
        state_transition.to_owned().clone().into(),
    )
    .await
    .map(Into::<ValidationResultWasm>::into)
    .map_err(|e| from_dpp_err(e.into()))
}
