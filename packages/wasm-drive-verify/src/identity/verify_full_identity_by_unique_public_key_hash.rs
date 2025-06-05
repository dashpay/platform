use drive::verify::RootHash;
use dpp::prelude::Identity;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;
use js_sys::Uint8Array;
use serde_wasm_bindgen::to_value;

#[wasm_bindgen]
pub struct VerifyFullIdentityByUniquePublicKeyHashResult {
    root_hash: Vec<u8>,
    identity: JsValue,
}

#[wasm_bindgen]
impl VerifyFullIdentityByUniquePublicKeyHashResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn identity(&self) -> JsValue {
        self.identity.clone()
    }
}

#[wasm_bindgen(js_name = "verifyFullIdentityByUniquePublicKeyHash")]
pub fn verify_full_identity_by_unique_public_key_hash(
    proof: &Uint8Array,
    public_key_hash: &Uint8Array,
    platform_version_number: u32,
) -> Result<VerifyFullIdentityByUniquePublicKeyHashResult, JsValue> {
    let proof_vec = proof.to_vec();
    
    let public_key_hash_bytes: [u8; 20] = public_key_hash
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid public_key_hash length. Expected 20 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, identity_option) = drive::verify::identity::verify_full_identity_by_unique_public_key_hash(
        &proof_vec,
        public_key_hash_bytes,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let identity_js = match identity_option {
        Some(identity) => {
            let identity_json = serde_json::to_value(&identity)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize identity: {:?}", e)))?;
            to_value(&identity_json)
                .map_err(|e| JsValue::from_str(&format!("Failed to convert identity to JsValue: {:?}", e)))?
        }
        None => JsValue::NULL,
    };

    Ok(VerifyFullIdentityByUniquePublicKeyHashResult {
        root_hash: root_hash.to_vec(),
        identity: identity_js,
    })
}