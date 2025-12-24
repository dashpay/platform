use crate::utils::getters::VecU8ToUint8Array;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::Uint8Array;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTokenStatusResult {
    root_hash: Vec<u8>,
    status: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenStatusResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn status(&self) -> JsValue {
        self.status.clone()
    }
}

#[wasm_bindgen(js_name = "verifyTokenStatus")]
pub fn verify_token_status(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenStatusResult, JsValue> {
    let proof_vec = proof.to_vec();

    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, status_option) = Drive::verify_token_status(
        &proof_vec,
        token_id_bytes,
        verify_subset_of_proof,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let status_js = match status_option {
        Some(status) => {
            // Convert TokenStatus to a JS object
            let status_value = match status {
                TokenStatus::V0(v0) => {
                    serde_json::json!({"paused": v0.paused})
                }
            };
            to_value(&status_value).unwrap_or(JsValue::NULL)
        }
        None => JsValue::NULL,
    };

    Ok(VerifyTokenStatusResult {
        root_hash: root_hash.to_vec(),
        status: status_js,
    })
}
