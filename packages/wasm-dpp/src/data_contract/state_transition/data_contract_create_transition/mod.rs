use dpp::{
    data_contract::state_transition::DataContractCreateTransition,
    state_transition::StateTransitionType,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{
    buffer::Buffer,
    errors::{from_dpp_err, RustConversionError},
    with_js_error, DataContractParameters, DataContractWasm,
};

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataContractCreateTransitionParameters {
    protocol_version: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    data_contract: Option<DataContractParameters>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    entropy: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    signature_public_key_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    signature: Option<Vec<u8>>,
}

#[wasm_bindgen(js_class=DataContractCreateTransition)]
impl DataContractCreateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<DataContractCreateTransitionWasm, JsValue> {
        let parameters: DataContractCreateTransitionParameters =
            with_js_error!(serde_wasm_bindgen::from_value(raw_parameters))?;
        DataContractCreateTransition::from_raw_object(
            serde_json::to_value(parameters).expect("the struct will be a valid json"),
        )
        .map(Into::into)
        .map_err(from_dpp_err)
    }

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
