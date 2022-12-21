use std::convert::TryInto;

use dpp::{
    data_contract::{
        validation::data_contract_validator::DataContractValidator, DataContractFactory,
        EntropyGenerator,
    },
    prelude::Identifier,
    version::ProtocolVersionValidator,
};
use wasm_bindgen::prelude::*;

use crate::{
    data_contract::errors::InvalidDataContractError,
    errors::{
        consensus_error::from_consensus_error, from_dpp_err, protocol_error::from_protocol_error,
        RustConversionError,
    },
    validation_result::ValidationResultWasm,
    with_js_error, DataContractCreateTransitionWasm, DataContractParameters, DataContractWasm,
};

#[wasm_bindgen(js_name=DataContractValidator)]
pub struct DataContractValidatorWasm(DataContractValidator);

impl From<DataContractValidator> for DataContractValidatorWasm {
    fn from(v: DataContractValidator) -> Self {
        DataContractValidatorWasm(v)
    }
}

impl Into<DataContractValidator> for DataContractValidatorWasm {
    fn into(self) -> DataContractValidator {
        self.0
    }
}

#[wasm_bindgen(js_class=DataContractValidator)]
impl DataContractValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        DataContractValidator::new(std::sync::Arc::new(ProtocolVersionValidator::default())).into()
    }

    #[wasm_bindgen(js_name=validate)]
    pub fn validate(&self, raw_data_contract: JsValue) -> Result<ValidationResultWasm, JsValue> {
        let parameters: DataContractParameters =
            with_js_error!(serde_wasm_bindgen::from_value(raw_data_contract.clone()))?;
        let json_object = serde_json::to_value(parameters).expect("Implements Serialize");
        self.0
            .validate(&json_object)
            .map(Into::into)
            .map_err(from_protocol_error)
    }
}

#[wasm_bindgen(js_name=DataContractFactory)]
pub struct DataContractFactoryWasm(DataContractFactory);

impl From<DataContractFactory> for DataContractFactoryWasm {
    fn from(v: DataContractFactory) -> Self {
        DataContractFactoryWasm(v)
    }
}

impl Into<DataContractFactory> for DataContractFactoryWasm {
    fn into(self) -> DataContractFactory {
        self.0
    }
}

#[wasm_bindgen]
extern "C" {
    pub type ExternalEntropyGenerator;

    #[wasm_bindgen(structural, method)]
    pub fn generate(this: &ExternalEntropyGenerator) -> Vec<u8>;
}

impl EntropyGenerator for ExternalEntropyGenerator {
    fn generate(&self) -> [u8; 32] {
        // TODO: think about changing API to return an error but does it worth it for JS?
        let res = ExternalEntropyGenerator::generate(self)
            .try_into()
            .expect("Bad entropy generator provided: should return 32 bytes");
        res
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
                validate_data_contract.into(),
                Box::new(external_entropy_generator),
            )
        } else {
            DataContractFactory::new(protocol_version, validate_data_contract.into()).into()
        }
        .into()
    }

    #[wasm_bindgen(js_name=create)]
    pub fn create(
        &self,
        owner_id: Vec<u8>,
        documents: JsValue,
    ) -> Result<DataContractWasm, JsValue> {
        let documents_json: serde_json::Value =
            with_js_error!(serde_wasm_bindgen::from_value(documents))?;
        let identifier = Identifier::from_bytes(&owner_id).map_err(from_dpp_err)?;
        self.0
            .create(identifier, documents_json)
            .map(Into::into)
            .map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=createFromObject)]
    pub async fn create_from_object(
        &self,
        object: JsValue,
        skip_validation: Option<bool>,
    ) -> Result<DataContractWasm, JsValue> {
        let parameters: DataContractParameters =
            with_js_error!(serde_wasm_bindgen::from_value(object.clone()))?;
        let parameters_json = serde_json::to_value(parameters).expect("Implements Serialize");
        let result = self
            .0
            .create_from_object(parameters_json, skip_validation.unwrap_or(false))
            .await;
        match result {
            Ok(data_contract) => Ok(data_contract.into()),
            Err(dpp::ProtocolError::InvalidDataContractError { errors, .. }) => {
                let js_errors = errors.into_iter().map(from_consensus_error).collect();
                Err(InvalidDataContractError::new(js_errors, object).into())
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
        data_contract: DataContractWasm,
    ) -> Result<DataContractCreateTransitionWasm, JsValue> {
        self.0
            .create_data_contract_create_transition(data_contract.into())
            .map(Into::into)
            .map_err(from_dpp_err)
    }
}
