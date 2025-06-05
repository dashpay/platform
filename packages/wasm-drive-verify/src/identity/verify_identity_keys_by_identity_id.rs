use dpp::identity::PartialIdentity;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::Drive;
use drive::verify::RootHash;
use js_sys::{Array, Uint8Array};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyIdentityKeysByIdentityIdResult {
    root_hash: Vec<u8>,
    identity: JsValue,
}

#[wasm_bindgen]
impl VerifyIdentityKeysByIdentityIdResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn identity(&self) -> JsValue {
        self.identity.clone()
    }
}

#[wasm_bindgen(js_name = "verifyIdentityKeysByIdentityId")]
pub fn verify_identity_keys_by_identity_id(
    proof: &Uint8Array,
    identity_id: &Uint8Array,
    specific_key_ids: Option<Array>,
    with_revision: bool,
    with_balance: bool,
    is_proof_subset: bool,
    limit: Option<u16>,
    offset: Option<u16>,
    platform_version_number: u32,
) -> Result<VerifyIdentityKeysByIdentityIdResult, JsValue> {
    let proof_vec = proof.to_vec();

    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

    // Create the key request type based on whether specific keys are requested
    let request_type = if let Some(keys_array) = specific_key_ids {
        let mut keys_vec = Vec::new();
        for i in 0..keys_array.length() {
            let key_id = keys_array
                .get(i)
                .as_f64()
                .ok_or_else(|| JsValue::from_str("Invalid key ID"))?
                as u32;
            keys_vec.push(key_id);
        }
        KeyRequestType::SpecificKeys(keys_vec)
    } else {
        KeyRequestType::AllKeys
    };

    let key_request = IdentityKeysRequest {
        identity_id: identity_id_bytes,
        request_type,
        limit,
        offset,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, identity_option) = Drive::verify_identity_keys_by_identity_id(
        &proof_vec,
        key_request,
        with_revision,
        with_balance,
        is_proof_subset,
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

    Ok(VerifyIdentityKeysByIdentityIdResult {
        root_hash: root_hash.to_vec(),
        identity: identity_js,
    })
}
