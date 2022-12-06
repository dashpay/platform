mod apply;
mod validation;

pub use apply::*;
pub use validation::*;

use dpp::{
    data_contract::state_transition::DataContractCreateTransition,
    state_transition::{
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
    },
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{
    buffer::Buffer,
    errors::{from_dpp_err, RustConversionError},
    identifier::IdentifierWrapper,
    with_js_error, DataContractParameters, DataContractWasm, StateTransitionExecutionContextWasm,
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

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.get_owner_id().clone().into()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u32 {
        self.0.get_type() as u32
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self, skip_signature: Option<bool>) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        Ok(self
            .0
            .to_json(skip_signature.unwrap_or(false))
            .map_err(from_dpp_err)?
            .serialize(&serializer)
            .expect("JSON is a valid object"))
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self, skip_signature: Option<bool>) -> Result<Buffer, JsValue> {
        let bytes = self
            .0
            .to_buffer(skip_signature.unwrap_or(false))
            .map_err(from_dpp_err)?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=getModifiedDataIds)]
    pub fn get_modified_data_ids(&self) -> Vec<JsValue> {
        self.0
            .get_modified_data_ids()
            .into_iter()
            .map(|identifier| Into::<IdentifierWrapper>::into(identifier.clone()).into())
            .collect()
    }

    #[wasm_bindgen(js_name=isDataContractStateTransition)]
    pub fn is_data_contract_state_transition(&self) -> bool {
        self.0.is_data_contract_state_transition()
    }

    #[wasm_bindgen(js_name=isDocumentStateTransition)]
    pub fn is_document_state_transition(&self) -> bool {
        self.0.is_document_state_transition()
    }

    #[wasm_bindgen(js_name=isIdentityStateTransition)]
    pub fn is_identity_state_transition(&self) -> bool {
        self.0.is_identity_state_transition()
    }

    #[wasm_bindgen(js_name=setExecutionContext)]
    pub fn set_execution_context(&mut self, context: StateTransitionExecutionContextWasm) {
        self.0.set_execution_context(context.into())
    }
}
