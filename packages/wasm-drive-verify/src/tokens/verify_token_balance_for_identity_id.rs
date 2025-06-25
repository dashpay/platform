use crate::utils::getters::VecU8ToUint8Array;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTokenBalanceForIdentityIdResult {
    root_hash: Vec<u8>,
    balance: Option<u64>,
}

#[wasm_bindgen]
impl VerifyTokenBalanceForIdentityIdResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn balance(&self) -> Option<u64> {
        self.balance
    }
}

#[wasm_bindgen(js_name = "verifyTokenBalanceForIdentityId")]
pub fn verify_token_balance_for_identity_id(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    identity_id: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenBalanceForIdentityIdResult, JsValue> {
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

    let (root_hash, balance_option) = Drive::verify_token_balance_for_identity_id(
        &proof_vec,
        token_id_bytes,
        identity_id_bytes,
        verify_subset_of_proof,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    Ok(VerifyTokenBalanceForIdentityIdResult {
        root_hash: root_hash.to_vec(),
        balance: balance_option,
    })
}
