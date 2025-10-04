pub mod methods;

use crate::identifier::IdentifierWasm;
use dpp::document::{Document, DocumentV0, DocumentV0Getters};
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::platform_value::Value;
use dpp::prelude::{BlockHeight, CoreBlockHeight, Revision};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = Document)]
pub struct DocumentWasm {
    id: IdentifierWasm,
    owner_id: IdentifierWasm,
    revision: Option<Revision>,
    data_contract_id: IdentifierWasm,
    document_type_name: String,
    properties: BTreeMap<String, Value>,
    created_at: Option<TimestampMillis>,
    updated_at: Option<TimestampMillis>,
    transferred_at: Option<TimestampMillis>,
    created_at_block_height: Option<BlockHeight>,
    updated_at_block_height: Option<BlockHeight>,
    transferred_at_block_height: Option<BlockHeight>,
    created_at_core_block_height: Option<CoreBlockHeight>,
    updated_at_core_block_height: Option<CoreBlockHeight>,
    transferred_at_core_block_height: Option<CoreBlockHeight>,
    entropy: Option<[u8; 32]>,
    creator_id: Option<IdentifierWasm>,
}

impl From<DocumentWasm> for Document {
    fn from(wasm_doc: DocumentWasm) -> Self {
        Document::V0(DocumentV0 {
            id: wasm_doc.id.into(),
            owner_id: wasm_doc.owner_id.into(),
            properties: wasm_doc.properties,
            revision: wasm_doc.revision,
            created_at: wasm_doc.created_at,
            updated_at: wasm_doc.updated_at,
            transferred_at: wasm_doc.transferred_at,
            created_at_block_height: wasm_doc.created_at_block_height,
            updated_at_block_height: wasm_doc.updated_at_block_height,
            transferred_at_block_height: wasm_doc.transferred_at_block_height,
            created_at_core_block_height: wasm_doc.created_at_core_block_height,
            updated_at_core_block_height: wasm_doc.updated_at_core_block_height,
            transferred_at_core_block_height: wasm_doc.transferred_at_core_block_height,
            creator_id: wasm_doc.creator_id.map(Into::into),
        })
    }
}

impl From<Document> for DocumentWasm {
    fn from(doc: Document) -> Self {
        DocumentWasm {
            id: doc.id().into(),
            owner_id: doc.owner_id().into(),
            revision: doc.revision(),
            data_contract_id: Identifier::default().into(),
            document_type_name: "".to_string(),
            properties: doc.properties().clone(),
            created_at: doc.created_at(),
            updated_at: doc.updated_at(),
            transferred_at: doc.transferred_at(),
            created_at_block_height: doc.created_at_block_height(),
            updated_at_block_height: doc.updated_at_block_height(),
            transferred_at_block_height: doc.transferred_at_block_height(),
            created_at_core_block_height: doc.created_at_core_block_height(),
            updated_at_core_block_height: doc.updated_at_core_block_height(),
            transferred_at_core_block_height: doc.transferred_at_core_block_height(),
            entropy: None,
            creator_id: doc.creator_id().map(Into::into),
        }
    }
}

impl DocumentWasm {
    pub fn from_batch(
        document: Document,
        data_contract_id: Identifier,
        document_type_name: String,
        entropy: Option<[u8; 32]>,
    ) -> Self {
        DocumentWasm {
            id: document.id().into(),
            owner_id: document.owner_id().into(),
            revision: document.revision(),
            data_contract_id: data_contract_id.into(),
            document_type_name,
            properties: document.properties().clone(),
            created_at: document.created_at(),
            updated_at: document.updated_at(),
            transferred_at: document.transferred_at(),
            created_at_block_height: document.created_at_block_height(),
            updated_at_block_height: document.updated_at_block_height(),
            transferred_at_block_height: document.transferred_at_block_height(),
            created_at_core_block_height: document.created_at_core_block_height(),
            updated_at_core_block_height: document.updated_at_core_block_height(),
            transferred_at_core_block_height: document.transferred_at_core_block_height(),
            entropy,
            creator_id: document.creator_id().map(Into::into),
        }
    }
}
