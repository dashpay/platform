use crate::errors::protocol_error::from_protocol_error;
use crate::{DataContractCreateTransitionWasm, DataContractUpdateTransitionWasm, DataContractWasm};
use dpp::data_contract::DataContractFacade;
use dpp::identifier::Identifier;
use dpp::version::ProtocolVersionValidator;

use crate::validation::ValidationResultWasm;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractFacade)]
#[derive(Clone)]
pub struct DataContractFacadeWasm(Arc<DataContractFacade>);

impl DataContractFacadeWasm {
    pub fn new(
        protocol_version: u32,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
    ) -> Self {
        let inner = DataContractFacade::new(protocol_version, protocol_version_validator);

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
    ) -> Result<DataContractWasm, JsValue> {
        let id = Identifier::from_bytes(&owner_id).map_err(from_protocol_error)?;

        self.0
            .create(id, serde_wasm_bindgen::from_value(documents)?)
            .map(Into::into)
            .map_err(from_protocol_error)
    }

    /// Create Data Contract from plain object
    #[wasm_bindgen(js_name=createFromObject)]
    pub async fn create_from_object(
        &self,
        js_raw_data_contract: JsValue,
        skip_validation: bool,
    ) -> Result<DataContractWasm, JsValue> {
        let raw_data_contract = serde_wasm_bindgen::from_value(js_raw_data_contract)?;
        self.0
            .create_from_object(raw_data_contract, skip_validation)
            .await
            .map(Into::into)
            .map_err(from_protocol_error)
    }

    /// Create Data Contract from buffer
    #[wasm_bindgen(js_name=createFromBuffer)]
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<DataContractWasm, JsValue> {
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
        data_contract: DataContractWasm,
    ) -> Result<DataContractCreateTransitionWasm, JsValue> {
        self.0
            .create_data_contract_create_transition(data_contract.into())
            .map(Into::into)
            .map_err(from_protocol_error)
    }

    /// Create Data Contract Update State Transition
    #[wasm_bindgen(js_name=createDataContractUpdateTransition)]
    pub fn create_data_contract_update_transition(
        &self,
        data_contract: DataContractWasm,
    ) -> Result<DataContractUpdateTransitionWasm, JsValue> {
        self.0
            .create_data_contract_update_transition(data_contract.into())
            .map(Into::into)
            .map_err(from_protocol_error)
    }

    /// Validate Data Contract
    pub async fn validate(
        &self,
        data_contract_json: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let raw_data_contract = serde_wasm_bindgen::from_value(data_contract_json)?;

        self.0
            .validate(raw_data_contract)
            .await
            .map(Into::into)
            .map_err(from_protocol_error)
    }
}
