use drive::drive::Drive;
use drive::verify::RootHash;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;
use js_sys::Uint8Array;

#[wasm_bindgen]
pub struct VerifyIdentityBalanceForIdentityIdResult {
    root_hash: Vec<u8>,
    balance: Option<u64>,
}

#[wasm_bindgen]
impl VerifyIdentityBalanceForIdentityIdResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn balance(&self) -> Option<u64> {
        self.balance
    }
}

#[wasm_bindgen(js_name = "verifyIdentityBalanceForIdentityId")]
pub fn verify_identity_balance_for_identity_id(
    proof: &Uint8Array,
    identity_id: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyIdentityBalanceForIdentityIdResult, JsValue> {
    let proof_vec = proof.to_vec();
    
    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, balance_option) = Drive::verify_identity_balance_for_identity_id(
        &proof_vec,
        identity_id_bytes,
        verify_subset_of_proof,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    Ok(VerifyIdentityBalanceForIdentityIdResult {
        root_hash: root_hash.to_vec(),
        balance: balance_option,
    })
}