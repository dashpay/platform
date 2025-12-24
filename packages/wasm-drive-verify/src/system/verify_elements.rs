// Element type is not exposed through drive's verify feature
// This is a placeholder implementation that demonstrates the limitation
use crate::utils::getters::VecU8ToUint8Array;
use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyElementsResult {
    root_hash: Vec<u8>,
    elements: JsValue,
}

#[wasm_bindgen]
impl VerifyElementsResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn elements(&self) -> JsValue {
        self.elements.clone()
    }
}

/// Verifies elements at a specific path with given keys
///
/// **Note**: This function is currently not fully implemented due to limitations in the
/// WASM environment. The Element type from grovedb is not exposed through the verify
/// feature, making it impossible to properly serialize and return element data.
///
/// For document verification, please use the document-specific verification functions
/// such as `verify_proof_keep_serialized` which are designed to work within these
/// limitations.
///
/// # Alternative Approaches:
///
/// 1. For document queries: Use `DriveDocumentQuery.verify_proof_keep_serialized()`
/// 2. For identity queries: Use the identity-specific verification functions
/// 3. For contract queries: Use `verify_contract()`
///
/// This limitation exists because:
/// - The Element enum from grovedb contains references to internal tree structures
/// - These structures cannot be safely exposed across the WASM boundary
/// - The verify feature intentionally excludes server-side types for security
#[wasm_bindgen(js_name = "verifyElements")]
pub fn verify_elements(
    _proof: &Uint8Array,
    _path: &Array,
    _keys: &Array,
    _platform_version_number: u32,
) -> Result<VerifyElementsResult, JsValue> {
    Err(JsValue::from_str(
        "verify_elements is not available in WASM due to grovedb Element type limitations. \
         Please use document, identity, or contract-specific verification functions instead. \
         See the function documentation for alternatives.",
    ))
}
