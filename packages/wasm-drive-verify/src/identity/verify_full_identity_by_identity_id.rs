use dpp::prelude::Identity;
use dpp::version::PlatformVersion;
use drive::verify::RootHash;
use js_sys::Uint8Array;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyFullIdentityByIdentityIdResult {
    root_hash: Vec<u8>,
    identity: JsValue,
}

#[wasm_bindgen]
impl VerifyFullIdentityByIdentityIdResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn identity(&self) -> JsValue {
        self.identity.clone()
    }
}

#[wasm_bindgen(js_name = "verifyFullIdentityByIdentityId")]
pub fn verify_full_identity_by_identity_id(
    proof: &Uint8Array,
    is_proof_subset: bool,
    identity_id: &Uint8Array,
    platform_version_number: u32,
) -> Result<VerifyFullIdentityByIdentityIdResult, JsValue> {
    let proof_vec = proof.to_vec();

    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, identity_option) =
        drive::verify::identity::verify_full_identity_by_identity_id(
            &proof_vec,
            is_proof_subset,
            identity_id_bytes,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let identity_js = match identity_option {
        Some(identity) => {
            let identity_json = serde_json::to_value(&identity).map_err(|e| {
                JsValue::from_str(&format!("Failed to serialize identity: {:?}", e))
            })?;
            to_value(&identity_json).map_err(|e| {
                JsValue::from_str(&format!("Failed to convert identity to JsValue: {:?}", e))
            })?
        }
        None => JsValue::NULL,
    };

    Ok(VerifyFullIdentityByIdentityIdResult {
        root_hash: root_hash.to_vec(),
        identity: identity_js,
    })
}
