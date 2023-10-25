use crate::identifier::IdentifierWrapper;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractIsReadonlyError)]
pub struct DataContractIsReadonlyErrorWasm {
    inner: DataContractIsReadonlyError,
}

impl From<&DataContractIsReadonlyError> for DataContractIsReadonlyErrorWasm {
    fn from(e: &DataContractIsReadonlyError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractIsReadonlyError)]
impl DataContractIsReadonlyErrorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(data_contract_id: IdentifierWrapper) -> Self {
        Self {
            inner: DataContractIsReadonlyError::new(data_contract_id.into()),
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
