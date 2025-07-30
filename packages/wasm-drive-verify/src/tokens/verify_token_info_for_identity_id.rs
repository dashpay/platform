use crate::utils::getters::VecU8ToUint8Array;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTokenInfoForIdentityIdResult {
    root_hash: Vec<u8>,
    token_info: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenInfoForIdentityIdResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn token_info(&self) -> JsValue {
        self.token_info.clone()
    }
}

#[wasm_bindgen(js_name = "verifyTokenInfoForIdentityId")]
pub fn verify_token_info_for_identity_id(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    identity_id: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenInfoForIdentityIdResult, JsValue> {
    let proof_vec = proof.to_vec();

    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, token_info_option) = Drive::verify_token_info_for_identity_id(
        &proof_vec,
        token_id_bytes,
        identity_id_bytes,
        verify_subset_of_proof,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let token_info_js = match token_info_option {
        Some(info) => {
            let obj = Object::new();

            // IdentityTokenInfo only has a frozen field
            match info {
                IdentityTokenInfo::V0(v0) => {
                    Reflect::set(
                        &obj,
                        &JsValue::from_str("frozen"),
                        &JsValue::from_bool(v0.frozen),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set frozen"))?;
                }
            }

            obj.into()
        }
        None => JsValue::NULL,
    };

    Ok(VerifyTokenInfoForIdentityIdResult {
        root_hash: root_hash.to_vec(),
        token_info: token_info_js,
    })
}
