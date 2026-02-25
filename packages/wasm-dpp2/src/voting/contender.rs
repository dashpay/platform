use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use dpp::prelude::Identifier;
use dpp::voting::contender_structs::{
    ContenderWithSerializedDocument, ContenderWithSerializedDocumentV0,
};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * ContenderWithSerializedDocument serialized as a plain object.
 */
export interface ContenderWithSerializedDocumentObject {
    v0: {
        identityId: Uint8Array;
        serializedDocument: Uint8Array | null;
        voteTally: number | null;
    };
}

/**
 * ContenderWithSerializedDocument serialized as JSON.
 */
export interface ContenderWithSerializedDocumentJSON {
    v0: {
        identityId: string;
        serializedDocument: string | null;
        voteTally: number | null;
    };
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ContenderWithSerializedDocumentObject")]
    pub type ContenderWithSerializedDocumentObjectJs;

    #[wasm_bindgen(typescript_type = "ContenderWithSerializedDocumentJSON")]
    pub type ContenderWithSerializedDocumentJSONJs;
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
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
    pub fn constructor(
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "serializedDocument")] serialized_document: Option<Vec<u8>>,
        #[wasm_bindgen(js_name = "voteTally")] vote_tally: Option<u32>,
    ) -> WasmDppResult<Self> {
        let identity: Identifier = identity_id.try_into()?;

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
    pub fn serialized_document(&self) -> Option<Uint8Array> {
        self.0
            .serialized_document()
            .as_ref()
            .map(|bytes| Uint8Array::from(bytes.as_slice()))
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

impl_wasm_conversions!(
    ContenderWithSerializedDocumentWasm,
    ContenderWithSerializedDocument,
    ContenderWithSerializedDocumentObjectJs,
    ContenderWithSerializedDocumentJSONJs
);

impl_wasm_type_info!(
    ContenderWithSerializedDocumentWasm,
    ContenderWithSerializedDocument
);
