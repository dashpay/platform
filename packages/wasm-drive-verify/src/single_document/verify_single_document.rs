use dpp::version::PlatformVersion;
use drive::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use wasm_bindgen::prelude::*;

/// WASM wrapper for SingleDocumentDriveQuery
#[wasm_bindgen]
pub struct SingleDocumentDriveQueryWasm {
    /// The inner Rust query
    inner: SingleDocumentDriveQuery,
}

#[wasm_bindgen]
impl SingleDocumentDriveQueryWasm {
    /// Create a new SingleDocumentDriveQuery
    #[wasm_bindgen(constructor)]
    pub fn new(
        contract_id: Vec<u8>,
        document_type_name: String,
        document_type_keeps_history: bool,
        document_id: Vec<u8>,
        block_time_ms: Option<f64>,
        contested_status: u8,
    ) -> Result<SingleDocumentDriveQueryWasm, JsValue> {
        let contract_id_array: [u8; 32] = contract_id
            .try_into()
            .map_err(|_| JsValue::from_str("contract_id must be exactly 32 bytes"))?;

        let document_id_array: [u8; 32] = document_id
            .try_into()
            .map_err(|_| JsValue::from_str("document_id must be exactly 32 bytes"))?;

        let contested_status = match contested_status {
            0 => SingleDocumentDriveQueryContestedStatus::NotContested,
            1 => SingleDocumentDriveQueryContestedStatus::MaybeContested,
            2 => SingleDocumentDriveQueryContestedStatus::Contested,
            _ => return Err(JsValue::from_str("Invalid contested status value")),
        };

        let block_time_ms = block_time_ms.map(|v| v as u64);

        Ok(SingleDocumentDriveQueryWasm {
            inner: SingleDocumentDriveQuery {
                contract_id: contract_id_array,
                document_type_name,
                document_type_keeps_history,
                document_id: document_id_array,
                block_time_ms,
                contested_status,
            },
        })
    }

    /// Get the contract ID
    #[wasm_bindgen(getter, js_name = "contractId")]
    pub fn contract_id(&self) -> Vec<u8> {
        self.inner.contract_id.to_vec()
    }

    /// Get the document type name
    #[wasm_bindgen(getter, js_name = "documentTypeName")]
    pub fn document_type_name(&self) -> String {
        self.inner.document_type_name.clone()
    }

    /// Get whether the document type keeps history
    #[wasm_bindgen(getter, js_name = "documentTypeKeepsHistory")]
    pub fn document_type_keeps_history(&self) -> bool {
        self.inner.document_type_keeps_history
    }

    /// Get the document ID
    #[wasm_bindgen(getter, js_name = "documentId")]
    pub fn document_id(&self) -> Vec<u8> {
        self.inner.document_id.to_vec()
    }

    /// Get the block time in milliseconds
    #[wasm_bindgen(getter, js_name = "blockTimeMs")]
    pub fn block_time_ms(&self) -> Option<f64> {
        self.inner.block_time_ms.map(|v| v as f64)
    }

    /// Get the contested status
    #[wasm_bindgen(getter, js_name = "contestedStatus")]
    pub fn contested_status(&self) -> u8 {
        match self.inner.contested_status {
            SingleDocumentDriveQueryContestedStatus::NotContested => 0,
            SingleDocumentDriveQueryContestedStatus::MaybeContested => 1,
            SingleDocumentDriveQueryContestedStatus::Contested => 2,
        }
    }
}

/// Result of a single document proof verification
#[wasm_bindgen]
pub struct SingleDocumentProofResult {
    root_hash: Vec<u8>,
    document_serialized: Option<Vec<u8>>,
}

#[wasm_bindgen]
impl SingleDocumentProofResult {
    /// Get the root hash
    #[wasm_bindgen(getter, js_name = "rootHash")]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    /// Get the serialized document (if found)
    #[wasm_bindgen(getter, js_name = "documentSerialized")]
    pub fn document_serialized(&self) -> Option<Vec<u8>> {
        self.document_serialized.clone()
    }

    /// Check if a document was found
    #[wasm_bindgen(js_name = "hasDocument")]
    pub fn has_document(&self) -> bool {
        self.document_serialized.is_some()
    }
}

/// Verify a single document proof and keep it serialized
#[wasm_bindgen(js_name = "verifySingleDocumentProofKeepSerialized")]
pub fn verify_single_document_proof_keep_serialized(
    query: &SingleDocumentDriveQueryWasm,
    is_subset: bool,
    proof: Vec<u8>,
    platform_version_number: u32,
) -> Result<SingleDocumentProofResult, JsValue> {
    // Get platform version
    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    // Verify the proof keeping it serialized
    match query
        .inner
        .verify_proof_keep_serialized(is_subset, &proof, &platform_version)
    {
        Ok((root_hash, maybe_serialized_document)) => Ok(SingleDocumentProofResult {
            root_hash: root_hash.to_vec(),
            document_serialized: maybe_serialized_document,
        }),
        Err(e) => Err(JsValue::from_str(&format!("Verification failed: {}", e))),
    }
}

/// Create a SingleDocumentDriveQuery for a non-contested document
#[wasm_bindgen(js_name = "createSingleDocumentQuery")]
pub fn create_single_document_query(
    contract_id: Vec<u8>,
    document_type_name: String,
    document_type_keeps_history: bool,
    document_id: Vec<u8>,
    block_time_ms: Option<f64>,
) -> Result<SingleDocumentDriveQueryWasm, JsValue> {
    SingleDocumentDriveQueryWasm::new(
        contract_id,
        document_type_name,
        document_type_keeps_history,
        document_id,
        block_time_ms,
        0, // NotContested
    )
}

/// Create a SingleDocumentDriveQuery for a maybe contested document
#[wasm_bindgen(js_name = "createSingleDocumentQueryMaybeContested")]
pub fn create_single_document_query_maybe_contested(
    contract_id: Vec<u8>,
    document_type_name: String,
    document_type_keeps_history: bool,
    document_id: Vec<u8>,
    block_time_ms: Option<f64>,
) -> Result<SingleDocumentDriveQueryWasm, JsValue> {
    SingleDocumentDriveQueryWasm::new(
        contract_id,
        document_type_name,
        document_type_keeps_history,
        document_id,
        block_time_ms,
        1, // MaybeContested
    )
}

/// Create a SingleDocumentDriveQuery for a contested document
#[wasm_bindgen(js_name = "createSingleDocumentQueryContested")]
pub fn create_single_document_query_contested(
    contract_id: Vec<u8>,
    document_type_name: String,
    document_type_keeps_history: bool,
    document_id: Vec<u8>,
    block_time_ms: Option<f64>,
) -> Result<SingleDocumentDriveQueryWasm, JsValue> {
    SingleDocumentDriveQueryWasm::new(
        contract_id,
        document_type_name,
        document_type_keeps_history,
        document_id,
        block_time_ms,
        2, // Contested
    )
}
