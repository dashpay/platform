use dpp::data_contract::state_transition::DataContractCreateTransition;
use wasm_bindgen::prelude::*;

use crate::{DataContractWasm, buffer::Buffer};

#[wasm_bindgen(js_name=DataContractCreateTransition)]
pub struct DataContractCreateTransitionWasm(DataContractCreateTransition);

impl From<DataContractCreateTransition> for DataContractCreateTransitionWasm {
    fn from(v: DataContractCreateTransition) -> Self {
        DataContractCreateTransitionWasm(v)
    }
}

impl Into<DataContractCreateTransition> for DataContractCreateTransitionWasm {
    fn into(self) -> DataContractCreateTransition {
        self.0
    }
}

#[wasm_bindgen(js_class=DataContractCreateTransition)]
impl DataContractCreateTransitionWasm {
    #[wasm_bindgen(js_name=getDataContract)]
    pub fn get_data_contract(&self) -> DataContractWasm {
        self.0.data_contract.clone().into()
    }

    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.0.protocol_version.into()
    }

    #[wasm_bindgen(js_name=getEntropy)]
    pub fn get_entropy(&self) -> Buffer {
        Buffer::from_bytes(&self.0.entropy)
    }
}
