use crate::bls_adapter::BlsAdapter;
use std::convert::TryInto;

use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::state_transition_factory::StateTransitionFactoryWasm;
use crate::utils::{ToSerdeJSONExt, WithJsError};
use crate::validation::ValidationResultWasm;
use crate::{with_js_error, StateTransitionExecutionContextWasm};
use dpp::state_transition::{
    StateTransitionFacade, StateTransitionFieldTypes, StateTransitionType, ValidateOptions,
};
use dpp::version::ProtocolVersionValidator;
use serde::Deserialize;

use crate::errors::consensus::basic::state_transition::InvalidStateTransitionTypeErrorWasm;
use crate::errors::protocol_error::from_protocol_error;
use crate::errors::value_error::PlatformValueErrorWasm;

use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;

use num_enum::TryFromPrimitiveError;
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
        options: Option<js_sys::Object>,
    ) -> Result<JsValue, JsValue> {
        let options: FromObjectOptions = if let Some(options) = options {
            with_js_error!(serde_wasm_bindgen::from_value(options.into()))?
        } else {
            Default::default()
        };

        let mut raw_state_transition = raw_state_transition.with_serde_to_platform_value()?;
        let state_transition_type: StateTransitionType = raw_state_transition
            .get_integer::<u8>("type")
            .map_err(PlatformValueErrorWasm::from)?
            .try_into()
            .map_err(|e: TryFromPrimitiveError<StateTransitionType>| {
                InvalidStateTransitionTypeErrorWasm::new(e.number)
            })?;

        if state_transition_type == StateTransitionType::DataContractUpdate {
            DataContractUpdateTransition::clean_value(&mut raw_state_transition)
                .map_err(PlatformValueErrorWasm::from)?;
        }

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
        options: Option<js_sys::Object>,
    ) -> Result<JsValue, JsValue> {
        let options: FromObjectOptions = if let Some(options) = options {
            with_js_error!(serde_wasm_bindgen::from_value(options.into()))?
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
        execution_context: &StateTransitionExecutionContextWasm,
        options: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let options: ValidateOptionsWasm = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let (state_transition, execution_context) = if let Ok(state_transition) =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            ) {
            let execution_context: StateTransitionExecutionContext =
                execution_context.to_owned().into();
            (state_transition, execution_context)
        } else {
            let state_transition_json = raw_state_transition.with_serde_to_platform_value()?;
            let execution_context = StateTransitionExecutionContext::default();
            let state_transition = self
                .0
                .create_from_object(state_transition_json.clone(), true)
                .await
                .with_js_error()?;
            (state_transition, execution_context)
        };

        let validation_result = self
            .0
            .validate(&state_transition, &execution_context, options.into())
            .await
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name = validateBasic)]
    pub async fn validate_basic(
        &self,
        raw_state_transition: JsValue,
        execution_context: &StateTransitionExecutionContextWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition_json;

        if let Ok(state_transition) =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            )
        {
            state_transition_json = state_transition.to_cleaned_object(false).with_js_error()?;
        } else {
            state_transition_json = raw_state_transition.with_serde_to_platform_value()?;
        }

        let validation_result = self
            .0
            .validate_basic(&state_transition_json, &execution_context.to_owned().into())
            .await
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name = validateSignature)]
    pub async fn validate_signature(
        &self,
        raw_state_transition: JsValue,
        execution_context: &StateTransitionExecutionContextWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            )?;

        let validation_result = self
            .0
            .validate_signature(state_transition, &execution_context.to_owned().into())
            .await
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name = validateFee)]
    pub async fn validate_fee(
        &self,
        raw_state_transition: JsValue,
        execution_context: &StateTransitionExecutionContextWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            )?;

        let validation_result = self
            .0
            .validate_fee(&state_transition, &execution_context.to_owned().into())
            .await
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name = validateState)]
    pub async fn validate_state(
        &self,
        raw_state_transition: JsValue,
        execution_context: &StateTransitionExecutionContextWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            )?;

        let validation_result = self
            .0
            .validate_state(&state_transition, &execution_context.to_owned().into())
            .await
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen]
    pub async fn apply(&self, state_transition_js: JsValue) -> Result<JsValue, JsValue> {
        let state_transition =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &state_transition_js,
            )?;

        self.0
            .apply(&state_transition)
            .await
            .map(|_| JsValue::undefined())
            .map_err(from_protocol_error)
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
