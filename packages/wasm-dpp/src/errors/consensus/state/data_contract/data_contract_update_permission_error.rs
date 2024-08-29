use crate::identifier::IdentifierWrapper;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use dpp::consensus::state::data_contract::data_contract_update_permission_error::DataContractUpdatePermissionError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractUpdatePermissionError)]
pub struct DataContractUpdatePermissionErrorWasm {
    inner: DataContractUpdatePermissionError,
}

impl From<&DataContractUpdatePermissionError> for DataContractUpdatePermissionErrorWasm {
    fn from(e: &DataContractUpdatePermissionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractUpdatePermissionError)]
impl DataContractUpdatePermissionErrorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(data_contract_id: IdentifierWrapper, identity_id: IdentifierWrapper) -> Self {
        Self {
            inner: DataContractUpdatePermissionError::new(
                data_contract_id.into(),
                identity_id.into(),
            ),
        }
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> IdentifierWrapper {
        self.inner.data_contract_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn identity_id(&self) -> IdentifierWrapper {
        self.inner.identity_id().to_owned().into()
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
