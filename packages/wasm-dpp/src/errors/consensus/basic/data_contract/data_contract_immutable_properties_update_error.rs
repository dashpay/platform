use dpp::consensus::basic::data_contract::DataContractImmutablePropertiesUpdateError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractImmutablePropertiesUpdateError)]
pub struct DataContractImmutablePropertiesUpdateErrorWasm {
    inner: DataContractImmutablePropertiesUpdateError,
}

impl From<&DataContractImmutablePropertiesUpdateError>
    for DataContractImmutablePropertiesUpdateErrorWasm
{
    fn from(e: &DataContractImmutablePropertiesUpdateError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractImmutablePropertiesUpdateError)]
impl DataContractImmutablePropertiesUpdateErrorWasm {
    #[wasm_bindgen(js_name=getOperation)]
    pub fn get_operation(&self) -> String {
        self.inner.operation()
    }

    #[wasm_bindgen(js_name=getFieldPath)]
    pub fn get_field_path(&self) -> String {
        self.inner.field_path()
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
