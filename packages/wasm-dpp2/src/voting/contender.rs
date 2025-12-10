use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use dpp::prelude::Identifier;
use dpp::voting::contender_structs::{
    ContenderWithSerializedDocument, ContenderWithSerializedDocumentV0,
};
use js_sys::Uint8Array;
use serde::Serialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = "ContenderWithSerializedDocument")]
pub struct ContenderWithSerializedDocumentWasm(ContenderWithSerializedDocument);

impl From<ContenderWithSerializedDocument> for ContenderWithSerializedDocumentWasm {
    fn from(contender: ContenderWithSerializedDocument) -> Self {
        Self(contender)
    }
}

impl From<ContenderWithSerializedDocumentWasm> for ContenderWithSerializedDocument {
    fn from(contender: ContenderWithSerializedDocumentWasm) -> Self {
        contender.0
    }
}

#[wasm_bindgen(js_class = ContenderWithSerializedDocument)]
impl ContenderWithSerializedDocumentWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        identity_id: &JsValue,
        serialized_document: Option<Vec<u8>>,
        vote_tally: Option<u32>,
    ) -> WasmDppResult<Self> {
        let identity: Identifier = IdentifierWasm::try_from(identity_id)?.into();

        let inner = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
            identity_id: identity,
            serialized_document,
            vote_tally,
        });

        Ok(Self(inner))
    }

    #[wasm_bindgen(getter = identityId)]
    pub fn identity_id(&self) -> IdentifierWasm {
        self.0.identity_id().into()
    }

    #[wasm_bindgen(getter = serializedDocument)]
    pub fn serialized_document(&self) -> JsValue {
        match self.0.serialized_document() {
            Some(bytes) => Uint8Array::from(bytes.as_slice()).into(),
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen(getter = voteTally)]
    pub fn vote_tally(&self) -> Option<u32> {
        self.0.vote_tally()
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let json_value = serde_json::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        json_value
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<ContenderWithSerializedDocumentWasm> {
        let json_value: JsonValue = serde_wasm_bindgen::from_value(js_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        let contender: ContenderWithSerializedDocument = serde_json::from_value(json_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(ContenderWithSerializedDocumentWasm(contender))
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        serde_wasm_bindgen::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<ContenderWithSerializedDocumentWasm> {
        let contender: ContenderWithSerializedDocument = serde_wasm_bindgen::from_value(js_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(ContenderWithSerializedDocumentWasm(contender))
    }
}

impl ContenderWithSerializedDocumentWasm {
    pub fn into_inner(self) -> ContenderWithSerializedDocument {
        self.0
    }

    pub fn as_inner(&self) -> &ContenderWithSerializedDocument {
        &self.0
    }
}
