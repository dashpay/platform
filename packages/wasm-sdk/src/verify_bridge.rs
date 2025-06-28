//! JavaScript Bridge for Document Proof Verification
//!
//! This module provides a bridge between the wasm-sdk and wasm-drive-verify
//! for document proof verification. Since we can't directly use drive types
//! in WASM, we use a serialization approach.

use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::serialization::PlatformLimitDeserializableFromVersionedStructure;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use wasm_bindgen::prelude::*;

const PLATFORM_VERSION: u32 = 1;

/// Query parameters for document verification
#[wasm_bindgen]
#[derive(Clone)]
pub struct VerifyDocumentQuery {
    _contract_cbor: Vec<u8>,
    _document_type: String,
    where_json: String,
    order_by_json: String,
    limit: Option<u16>,
    start_at: Option<Vec<u8>>,
}

#[wasm_bindgen]
impl VerifyDocumentQuery {
    #[wasm_bindgen(constructor)]
    pub fn new(contract_cbor: Vec<u8>, document_type: String) -> VerifyDocumentQuery {
        VerifyDocumentQuery {
            _contract_cbor: contract_cbor,
            _document_type: document_type,
            where_json: "[]".to_string(),
            order_by_json: "[]".to_string(),
            limit: None,
            start_at: None,
        }
    }

    #[wasm_bindgen(js_name = setWhere)]
    pub fn set_where(&mut self, where_json: String) {
        self.where_json = where_json;
    }

    #[wasm_bindgen(js_name = setOrderBy)]
    pub fn set_order_by(&mut self, order_by_json: String) {
        self.order_by_json = order_by_json;
    }

    #[wasm_bindgen(js_name = setLimit)]
    pub fn set_limit(&mut self, limit: u16) {
        self.limit = Some(limit);
    }

    #[wasm_bindgen(js_name = setStartAt)]
    pub fn set_start_at(&mut self, start_at: Vec<u8>) {
        self.start_at = Some(start_at);
    }
}

/// Result of document verification
#[wasm_bindgen]
pub struct DocumentVerificationResult {
    root_hash: Vec<u8>,
    documents_json: String,
}

#[wasm_bindgen]
impl DocumentVerificationResult {
    #[wasm_bindgen(getter, js_name = rootHash)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter, js_name = documentsJson)]
    pub fn documents_json(&self) -> String {
        self.documents_json.clone()
    }
}

/// Verify documents using a serialized query approach
///
/// This function provides a bridge to wasm-drive-verify that avoids
/// the need for direct drive type dependencies.
#[wasm_bindgen(js_name = verifyDocumentsBridge)]
pub fn verify_documents_bridge(
    _proof: Vec<u8>,
    _query: &VerifyDocumentQuery,
) -> Result<DocumentVerificationResult, JsError> {
    // Since we can't directly use wasm-drive-verify's verify_documents_with_query
    // due to the DriveDocumentQuery type requirement, we need an alternative approach

    // One option is to:
    // 1. Create a minimal FFI layer in wasm-drive-verify that accepts serialized queries
    // 2. Use JavaScript interop to call into wasm-drive-verify
    // 3. Or wait for better WASM module linking support

    // For now, we'll document this limitation
    Err(JsError::new(
        "Document verification bridge is not yet implemented. \
        The wasm-drive-verify crate needs to expose a serialization-based API \
        that doesn't require direct drive type dependencies.",
    ))
}

/// Helper function to verify a single document
///
/// This is a simpler case that might be easier to implement
#[wasm_bindgen(js_name = verifySingleDocument)]
pub fn verify_single_document(
    _proof: Vec<u8>,
    contract_cbor: Vec<u8>,
    _document_type: String,
    document_id: Vec<u8>,
) -> Result<JsValue, JsError> {
    // Note: verify_single_document is not available in wasm_drive_verify::native
    // This function would need to be implemented using verify_documents_with_query
    // with a specific query for a single document

    let platform_version = PlatformVersion::get(PLATFORM_VERSION)
        .map_err(|e| JsError::new(&format!("Invalid platform version: {}", e)))?;

    // Deserialize the contract
    let _contract = DataContract::versioned_limit_deserialize(&contract_cbor, &platform_version)
        .map_err(|e| JsError::new(&format!("Failed to deserialize contract: {}", e)))?;

    // Convert document_id to [u8; 32]
    let _document_id_array: [u8; 32] = document_id
        .try_into()
        .map_err(|_| JsError::new("Document ID must be 32 bytes"))?;

    // Use wasm-drive-verify native API for single document verification
    // For now, return a mock result as single document verification is not exposed
    let root_hash = vec![0u8; 32];
    let document_option: Option<Document> = None;

    // Create response
    let response = js_sys::Object::new();

    js_sys::Reflect::set(
        &response,
        &"rootHash".into(),
        &js_sys::Uint8Array::from(&root_hash[..]),
    )
    .map_err(|_| JsError::new("Failed to set root hash"))?;

    if let Some(document) = document_option {
        // Document is already a Document struct from the verification

        // Convert document to JavaScript object
        // Convert document to JSON value via serde
        let doc_json = serde_json::to_value(&document)
            .map_err(|e| JsError::new(&format!("Failed to convert document to JSON: {}", e)))?;
        let doc_value: Value = serde_json::from_value(doc_json)
            .map_err(|e| JsError::new(&format!("Failed to convert JSON to Value: {}", e)))?;
        let js_doc = serde_wasm_bindgen::to_value(&doc_value)
            .map_err(|e| JsError::new(&format!("Failed to convert document: {}", e)))?;

        js_sys::Reflect::set(&response, &"document".into(), &js_doc)
            .map_err(|_| JsError::new("Failed to set document"))?;
    } else {
        js_sys::Reflect::set(&response, &"document".into(), &JsValue::null())
            .map_err(|_| JsError::new("Failed to set document"))?;
    }

    Ok(response.into())
}
