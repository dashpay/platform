use std::convert::TryFrom;
use std::sync::Arc;

use dpp::data_contract::CreatedDataContract;
use dpp::{
    data_contract::{
        validation::data_contract_validator::DataContractValidator, DataContractFactory,
    },
    platform_value,
    prelude::Identifier,
    version::ProtocolVersionValidator,
    ProtocolError,
};
use wasm_bindgen::prelude::*;

use crate::entropy_generator::ExternalEntropyGenerator;
use crate::utils::{ToSerdeJSONExt, WithJsError};

use crate::{
    data_contract::errors::InvalidDataContractError,
    errors::{from_dpp_err, protocol_error::from_protocol_error},
    js_value_to_data_contract_value,
    validation::ValidationResultWasm,
    with_js_error, DataContractCreateTransitionWasm, DataContractParameters, DataContractWasm,
};

#[wasm_bindgen(js_name=DataContractValidator)]
pub struct DataContractValidatorWasm(DataContractValidator);

impl From<DataContractValidator> for DataContractValidatorWasm {
    fn from(v: DataContractValidator) -> Self {
        DataContractValidatorWasm(v)
    }
}

impl From<DataContractValidatorWasm> for DataContractValidator {
    fn from(val: DataContractValidatorWasm) -> Self {
        val.0
    }
}

impl Default for DataContractValidatorWasm {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class=DataContractValidator)]
impl DataContractValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        DataContractValidator::new(std::sync::Arc::new(ProtocolVersionValidator::default())).into()
    }

    #[wasm_bindgen]
    pub fn validate(&self, raw_data_contract: JsValue) -> Result<ValidationResultWasm, JsValue> {
        let parameters: DataContractParameters =
            with_js_error!(serde_wasm_bindgen::from_value(raw_data_contract))?;
        let platform_object = platform_value::to_value(parameters).expect("Implements Serialize");

        let validation_result = self
            .0
            .validate(&platform_object)
            .map_err(from_protocol_error)?;
        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}

#[wasm_bindgen(js_name=DataContractFactory)]
pub struct DataContractFactoryWasm(DataContractFactory);

impl From<DataContractFactory> for DataContractFactoryWasm {
    fn from(v: DataContractFactory) -> Self {
        DataContractFactoryWasm(v)
    }
}

impl From<DataContractFactoryWasm> for DataContractFactory {
    fn from(val: DataContractFactoryWasm) -> Self {
        val.0
    }
}

#[wasm_bindgen(js_class=DataContractFactory)]
impl DataContractFactoryWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        protocol_version: u32,
        validate_data_contract: DataContractValidatorWasm,
        external_entropy_generator_arg: Option<ExternalEntropyGenerator>,
    ) -> DataContractFactoryWasm {
        if let Some(external_entropy_generator) = external_entropy_generator_arg {
            DataContractFactory::new_with_entropy_generator(
                protocol_version,
                Arc::new(validate_data_contract.into()),
                Box::new(external_entropy_generator),
            )
        } else {
            DataContractFactory::new(protocol_version, Arc::new(validate_data_contract.into()))
        }
        .into()
    }

    #[wasm_bindgen(js_name=create)]
    pub fn create(
        &self,
        owner_id: Vec<u8>,
        documents: JsValue,
        config: JsValue,
    ) -> Result<DataContractWasm, JsValue> {
        let documents_object: platform_value::Value =
            with_js_error!(serde_wasm_bindgen::from_value(documents))?;

        let contract_config = if config.is_object() && !config.is_falsy() {
            let raw_config = config.with_serde_to_json_value()?;
            Some(serde_json::from_value(raw_config).with_js_error()?)
        } else {
            None
        };

        let identifier = Identifier::from_bytes(&owner_id)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;
        //todo: contract config
        self.0
            .create(identifier, documents_object, contract_config, None)
            .map(Into::into)
            .with_js_error()
    }

    #[wasm_bindgen(js_name=createFromObject)]
    pub async fn create_from_object(
        &self,
        object: JsValue,
        skip_validation: Option<bool>,
    ) -> Result<DataContractWasm, JsValue> {
        let parameters_value = js_value_to_data_contract_value(object.clone())?;
        let result = self
            .0
            .create_from_object(parameters_value, skip_validation.unwrap_or(false))
            .await;
        match result {
            Ok(data_contract) => Ok(data_contract.into()),
            Err(dpp::ProtocolError::InvalidDataContractError(err)) => {
                Err(InvalidDataContractError::new(err.errors, object).into())
            }
            Err(other) => Err(from_dpp_err(other)),
        }
    }

    #[wasm_bindgen(js_name=createFromBuffer)]
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: Option<bool>,
    ) -> Result<DataContractWasm, JsValue> {
        self.0
            .create_from_buffer(buffer, skip_validation.unwrap_or(false))
            .await
            .map(Into::into)
            .map_err(from_protocol_error)
    }

    #[wasm_bindgen(js_name=createDataContractCreateTransition)]
    pub async fn create_data_contract_create_transition(
        &self,
        data_contract: &DataContractWasm,
    ) -> Result<DataContractCreateTransitionWasm, JsValue> {
        self.0
            .create_data_contract_create_transition(
                CreatedDataContract::try_from(data_contract).with_js_error()?,
            )
            .map(Into::into)
            .with_js_error()
    }
}
