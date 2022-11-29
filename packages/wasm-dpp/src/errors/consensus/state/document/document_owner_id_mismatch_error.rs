use crate::buffer::Buffer;
use dpp::identifier::Identifier;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DocumentOwnerIdMismatchError)]
pub struct DocumentOwnerIdMismatchErrorWasm {
    document_id: Identifier,
    document_owner_id: Identifier,
    existing_document_owner_id: Identifier,
    code: u32,
}

#[wasm_bindgen(js_class=DocumentOwnerIdMismatchError)]
impl DocumentOwnerIdMismatchErrorWasm {
    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.document_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getDocumentOwnerId)]
    pub fn document_owner_id(&self) -> Buffer {
        Buffer::from_bytes(self.document_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getExistingDocumentOwnerId)]
    pub fn existing_document_owner_id(&self) -> Buffer {
        Buffer::from_bytes(self.document_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl DocumentOwnerIdMismatchErrorWasm {
    pub fn new(document_id: Identifier, document_owner_id: Identifier, existing_document_owner_id: Identifier, code: u32) -> Self {
        Self { document_id, document_owner_id, existing_document_owner_id, code }
    }
}
