use crate::bls_adapter::BlsAdapter;

use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::state_transition_factory::StateTransitionFactoryWasm;
use crate::utils::{ToSerdeJSONExt, WithJsError};
use crate::validation::ValidationResultWasm;
use crate::with_js_error;
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::state_transition::{StateTransitionFacade, StateTransitionLike, ValidateOptions};
use dpp::version::ProtocolVersionValidator;
use serde::Deserialize;

use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[derive(Clone)]
#[wasm_bindgen(js_name = StateTransitionFacade)]
pub struct StateTransitionFacadeWasm(
    StateTransitionFacade<ExternalStateRepositoryLikeWrapper, BlsAdapter>,
);

impl StateTransitionFacadeWasm {
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
        bls_adapter: BlsAdapter,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
    ) -> Result<StateTransitionFacadeWasm, JsValue> {
        let state_repository_wrapper = ExternalStateRepositoryLikeWrapper::new(state_repository);

        let state_transition_facade = StateTransitionFacade::new(
            state_repository_wrapper,
            bls_adapter,
            protocol_version_validator,
        )
        .with_js_error()?;

        Ok(StateTransitionFacadeWasm(state_transition_facade))
    }
}

#[wasm_bindgen(js_class = StateTransitionFacade)]
impl StateTransitionFacadeWasm {
    #[wasm_bindgen(js_name = createFromObject)]
    pub async fn create_from_object(
        &self,
        raw_state_transition: JsValue,
        options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let options: FromObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let raw_state_transition = raw_state_transition.with_serde_to_platform_value()?;

        let result = self
            .0
            .create_from_object(
                raw_state_transition,
                options.skip_validation.unwrap_or(false),
            )
            .await;

        StateTransitionFactoryWasm::state_transition_wasm_from_factory_result(result)
    }

    #[wasm_bindgen(js_name = createFromBuffer)]
    pub async fn create_from_buffer(
        &self,
        state_transition_buffer: Vec<u8>,
        options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let options: FromObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let result = self
            .0
            .create_from_buffer(
                &state_transition_buffer,
                options.skip_validation.unwrap_or(false),
            )
            .await;

        StateTransitionFactoryWasm::state_transition_wasm_from_factory_result(result)
    }

    #[wasm_bindgen]
    pub async fn validate(
        &self,
        raw_state_transition: JsValue,
        options: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let options: ValidateOptionsWasm = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let (state_transition, state_transition_json, execution_context) =
            if let Ok(state_transition) =
                super::super::conversion::create_state_transition_from_wasm_instance(
                    &raw_state_transition,
                )
            {
                let execution_context = state_transition.get_execution_context().to_owned();
                // TODO: revisit after https://github.com/dashpay/platform/pull/809 is merged
                //  we use this workaround to produce JSON value for validation because
                //  state_transition.to_object() returns value that does not pass basic validation
                let state_transition_json =
                    super::super::conversion::state_transition_wasm_to_object(
                        &raw_state_transition,
                    )?
                    .with_serde_to_platform_value()?;
                (state_transition, state_transition_json, execution_context)
            } else {
                let state_transition_json = raw_state_transition.with_serde_to_platform_value()?;
                let execution_context = StateTransitionExecutionContext::default();
                let state_transition = self
                    .0
                    .create_from_object(state_transition_json.clone(), true)
                    .await
                    .with_js_error()?;
                (state_transition, state_transition_json, execution_context)
            };

        let validation_result = self
            .0
            .validate(
                &state_transition,
                &state_transition_json,
                &execution_context,
                options.into(),
            )
            .await
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name = validateBasic)]
    pub async fn validate_basic(
        &self,
        raw_state_transition: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition_json;
        let execution_context;

        if let Ok(state_transition) =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            )
        {
            execution_context = state_transition.get_execution_context().to_owned();
            // TODO: revisit after https://github.com/dashpay/platform/pull/809 is merged
            //  we use this workaround to produce JSON value for validation because
            //  state_transition.to_object() returns value that does not pass basic validation
            state_transition_json =
                super::super::conversion::state_transition_wasm_to_object(&raw_state_transition)?
                    .with_serde_to_platform_value()?;
        } else {
            state_transition_json = raw_state_transition.with_serde_to_platform_value()?;
            execution_context = StateTransitionExecutionContext::default();
        }

        let validation_result = self
            .0
            .validate_basic(&state_transition_json, &execution_context)
            .await
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name = validateSignature)]
    pub async fn validate_signature(
        &self,
        raw_state_transition: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            )?;

        let validation_result = self
            .0
            .validate_signature(state_transition)
            .await
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name = validateFee)]
    pub async fn validate_fee(
        &self,
        raw_state_transition: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            )?;

        let validation_result = self
            .0
            .validate_fee(&state_transition)
            .await
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name = validateState)]
    pub async fn validate_state(
        &self,
        raw_state_transition: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            )?;

        let validation_result = self
            .0
            .validate_state(&state_transition)
            .await
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FromObjectOptions {
    pub skip_validation: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidateOptionsWasm {
    pub basic: Option<bool>,
    pub signature: Option<bool>,
    pub fee: Option<bool>,
    pub state: Option<bool>,
}

impl From<ValidateOptionsWasm> for ValidateOptions {
    fn from(options: ValidateOptionsWasm) -> Self {
        Self {
            basic: options.basic.unwrap_or(true),
            signature: options.signature.unwrap_or(true),
            fee: options.fee.unwrap_or(true),
            state: options.state.unwrap_or(true),
        }
    }
}
