use dpp::consensus::basic::data_contract::DataContractInvalidIndexDefinitionUpdateError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractInvalidIndexDefinitionUpdateError)]
pub struct DataContractInvalidIndexDefinitionUpdateErrorWasm {
    inner: DataContractInvalidIndexDefinitionUpdateError,
}

impl From<&DataContractInvalidIndexDefinitionUpdateError>
    for DataContractInvalidIndexDefinitionUpdateErrorWasm
{
    fn from(e: &DataContractInvalidIndexDefinitionUpdateError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractInvalidIndexDefinitionUpdateError)]
impl DataContractInvalidIndexDefinitionUpdateErrorWasm {
    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.inner.document_type().to_string()
    }

    #[wasm_bindgen(js_name=getIndexName)]
    pub fn get_index_name(&self) -> String {
        self.inner.index_path().to_string()
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
