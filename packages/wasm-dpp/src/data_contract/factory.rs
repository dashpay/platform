use std::sync::Arc;

use dpp::data_contract::validation::data_contract_validator::DataContractValidator;
use dpp::data_contract::DataContractFactory;
use dpp::version::ProtocolVersionValidator;
use wasm_bindgen::prelude::*;

use crate::errors::from_dpp_err;
use crate::identifier::IdentifierWrapper;
use crate::utils::to_serde_json_value;
use crate::DataContractWasm;

#[wasm_bindgen(js_name=DataContractFactory)]
pub struct DataContractFactoryWasm(DataContractFactory);

#[wasm_bindgen(js_class=DataContractFactory)]
impl DataContractFactoryWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(protocol_version: u32) -> DataContractFactoryWasm {
        let protocol_validator = ProtocolVersionValidator::new(
            protocol_version,
            dpp::version::LATEST_VERSION,
            dpp::version::COMPATIBILITY_MAP.clone(),
        );
        let data_contract_validator = DataContractValidator::new(Arc::new(protocol_validator));
        DataContractFactoryWasm(DataContractFactory::new(0, data_contract_validator))
    }

    #[wasm_bindgen(js_name=create)]
    pub fn create(
        &self,
        owner_id: IdentifierWrapper,
        documents: JsValue,
    ) -> Result<DataContractWasm, JsValue> {
        let documents = to_serde_json_value(&documents)?;

        let data_contract = self
            .0
            .create(owner_id.inner(), documents)
            .map_err(from_dpp_err)?;

        Ok(data_contract.into())
    }
}
