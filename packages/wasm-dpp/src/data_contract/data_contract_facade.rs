use crate::errors::protocol_error::from_protocol_error;

use crate::{
    js_value_to_data_contract_value, DataContractCreateTransitionWasm,
    DataContractUpdateTransitionWasm, DataContractWasm,
};
use dpp::data_contract::DataContractFacade;
use dpp::identifier::Identifier;
use dpp::version::ProtocolVersionValidator;

use crate::entropy_generator::ExternalEntropyGenerator;
use crate::utils::{get_bool_from_options, WithJsError, SKIP_VALIDATION_PROPERTY_NAME};
use crate::validation::ValidationResultWasm;
use dpp::platform_value::Value;
use dpp::ProtocolError;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractFacade)]
#[derive(Clone)]
pub struct DataContractFacadeWasm(Arc<DataContractFacade>);

impl DataContractFacadeWasm {
    pub fn new(
        protocol_version: u32,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        entropy_generator: ExternalEntropyGenerator,
    ) -> Self {
        let inner = DataContractFacade::new_with_entropy_generator(
            protocol_version,
            protocol_version_validator,
            Box::new(entropy_generator),
        );

        Self(Arc::new(inner))
    }
}

#[wasm_bindgen(js_class=DataContractFacade)]
impl DataContractFacadeWasm {
    /// Create Data Contract
    #[wasm_bindgen]
    pub fn create(
        &self,
        owner_id: Vec<u8>,
        documents: JsValue,
        definitions: JsValue,
    ) -> Result<DataContractWasm, JsValue> {
        let id = Identifier::from_bytes(&owner_id)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;

        let definitions: Option<Value> = if definitions.is_object() {
            Some(serde_wasm_bindgen::from_value(definitions)?)
        } else {
            None
        };

        //todo: contract config
        self.0
            .create(
                id,
                serde_wasm_bindgen::from_value(documents)?,
                None,
                definitions,
            )
            .map(Into::into)
            .map_err(from_protocol_error)
    }

    /// Create Data Contract from plain object
    #[wasm_bindgen(js_name=createFromObject)]
    pub async fn create_from_object(
        &self,
        js_raw_data_contract: JsValue,
        options: Option<js_sys::Object>,
    ) -> Result<DataContractWasm, JsValue> {
        let skip_validation = if let Some(options) = options {
            get_bool_from_options(options.into(), SKIP_VALIDATION_PROPERTY_NAME, false)?
        } else {
            false
        };

        self.0
            .create_from_object(
                js_value_to_data_contract_value(js_raw_data_contract)?,
                skip_validation,
            )
            .await
            .map(DataContractWasm::from)
            .map_err(from_protocol_error)
    }

    /// Create Data Contract from buffer
    #[wasm_bindgen(js_name=createFromBuffer)]
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        options: Option<js_sys::Object>,
    ) -> Result<DataContractWasm, JsValue> {
        let skip_validation = if let Some(options) = options {
            get_bool_from_options(options.into(), SKIP_VALIDATION_PROPERTY_NAME, false)?
        } else {
            false
        };
        self.0
            .create_from_buffer(buffer, skip_validation)
            .await
            .map(Into::into)
            .map_err(from_protocol_error)
    }

    /// Create Data Contract Create State Transition
    #[wasm_bindgen(js_name=createDataContractCreateTransition)]
    pub fn create_data_contract_create_transition(
        &self,
        data_contract: &DataContractWasm,
    ) -> Result<DataContractCreateTransitionWasm, JsValue> {
        self.0
            .create_data_contract_create_transition(data_contract.clone().into())
            .map(DataContractCreateTransitionWasm::from)
            .with_js_error()
    }

    /// Create Data Contract Update State Transition
    #[wasm_bindgen(js_name=createDataContractUpdateTransition)]
    pub fn create_data_contract_update_transition(
        &self,
        data_contract: &DataContractWasm,
    ) -> Result<DataContractUpdateTransitionWasm, JsValue> {
        self.0
            .create_data_contract_update_transition(data_contract.to_owned().into())
            .map(Into::into)
            .map_err(from_protocol_error)
    }

    /// Validate Data Contract
    pub async fn validate(
        &self,
        js_raw_data_contract: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let raw_data_contract = js_value_to_data_contract_value(js_raw_data_contract)?;

        self.0
            .validate(raw_data_contract)
            .await
            .map(|v| v.map(|_| JsValue::UNDEFINED))
            .map(Into::into)
            .map_err(from_protocol_error)
    }
}
