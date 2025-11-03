use crate::identifier::IdentifierWasm;
use dpp::voting::contender_structs::{
    ContenderWithSerializedDocument, ContenderWithSerializedDocumentV0,
};
use js_sys::Uint8Array;
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
        identity_id: &IdentifierWasm,
        serialized_document: Option<Vec<u8>>,
        vote_tally: Option<u32>,
    ) -> Self {
        let inner = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
            identity_id: (*identity_id).into(),
            serialized_document,
            vote_tally,
        });

        Self(inner)
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
}

impl ContenderWithSerializedDocumentWasm {
    pub fn into_inner(self) -> ContenderWithSerializedDocument {
        self.0
    }

    pub fn as_inner(&self) -> &ContenderWithSerializedDocument {
        &self.0
    }
}
