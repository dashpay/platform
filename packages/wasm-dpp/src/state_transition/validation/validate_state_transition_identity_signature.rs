use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::utils::WithJsError;
use crate::validation::ValidationResultWasm;

use crate::errors::consensus::consensus_error::from_consensus_error;
use dpp::consensus::basic::state_transition::InvalidStateTransitionTypeError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::state_transition::validation::validate_state_transition_identity_signature::validate_state_transition_identity_signature;
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned};

use std::sync::Arc;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

struct StValidator {
    bls: BlsAdapter,
    state_repository: Arc<ExternalStateRepositoryLikeWrapper>,
}

impl StValidator {
    pub async fn validate(
        &self,
        state_transition: &mut impl StateTransitionIdentitySigned,
    ) -> Result<ValidationResultWasm, JsValue> {
        let result = validate_state_transition_identity_signature(
            self.state_repository.clone(),
            state_transition,
            &self.bls,
            &StateTransitionExecutionContext::default(), // TODO(v0.24-backport): is it fine using default context here?
        )
        .await
        .with_js_error()?;
        Ok(ValidationResultWasm::from(
            result.map(|_v| JsValue::undefined()),
        ))
    }
}

#[wasm_bindgen(js_name=validateStateTransitionIdentitySignature)]
pub async fn validate_state_transition_identity_signature_wasm(
    external_state_repository: ExternalStateRepositoryLike,
    js_state_transition: &JsValue,
    bls_adapter: JsBlsAdapter,
) -> Result<ValidationResultWasm, JsValue> {
    let bls = BlsAdapter(bls_adapter);
    let state_repository = Arc::new(ExternalStateRepositoryLikeWrapper::new(
        external_state_repository,
    ));

    let validator = StValidator {
        bls,
        state_repository,
    };

    let state_transition =
        super::super::conversion::create_state_transition_from_wasm_instance(js_state_transition)?;

    match state_transition {
        StateTransition::DataContractCreate(mut state_transition) => {
            validator.validate(&mut state_transition).await
        }
        StateTransition::DataContractUpdate(mut state_transition) => {
            validator.validate(&mut state_transition).await
        }
        StateTransition::DocumentsBatch(mut state_transition) => {
            validator.validate(&mut state_transition).await
        }
        // TODO: We should use protocol error here, not consensus
        StateTransition::IdentityCreate(state_transition) => Err(from_consensus_error(
            ConsensusError::BasicError(BasicError::InvalidStateTransitionTypeError(
                InvalidStateTransitionTypeError::new(state_transition.transition_type as u8),
            )),
        )),
        // TODO: We should use protocol error here, not consensus
        StateTransition::IdentityTopUp(state_transition) => Err(from_consensus_error(
            ConsensusError::BasicError(BasicError::InvalidStateTransitionTypeError(
                InvalidStateTransitionTypeError::new(state_transition.transition_type as u8),
            )),
        )),
        StateTransition::IdentityCreditWithdrawal(mut state_transition) => {
            validator.validate(&mut state_transition).await
        }
        StateTransition::IdentityUpdate(mut state_transition) => {
            validator.validate(&mut state_transition).await
        }
    }
}
