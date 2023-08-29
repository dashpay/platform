use crate::identifier::IdentifierWrapper;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;

use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractConfigUpdateError)]
pub struct DataContractConfigUpdateErrorWasm {
    inner: DataContractConfigUpdateError,
}

impl From<&DataContractConfigUpdateError> for DataContractConfigUpdateErrorWasm {
    fn from(e: &DataContractConfigUpdateError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractConfigUpdateError)]
impl DataContractConfigUpdateErrorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(data_contract_id: IdentifierWrapper, message: String) -> Self {
        Self {
            inner: DataContractConfigUpdateError::new(data_contract_id.into(), message),
        }
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> IdentifierWrapper {
        self.inner.data_contract_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
