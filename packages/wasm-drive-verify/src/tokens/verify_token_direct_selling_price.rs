use drive::drive::Drive;
use drive::verify::RootHash;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;
use js_sys::Uint8Array;

#[wasm_bindgen]
pub struct VerifyTokenDirectSellingPriceResult {
    root_hash: Vec<u8>,
    price: Option<u64>,
}

#[wasm_bindgen]
impl VerifyTokenDirectSellingPriceResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn price(&self) -> Option<u64> {
        self.price
    }
}

#[wasm_bindgen(js_name = "verifyTokenDirectSellingPrice")]
pub fn verify_token_direct_selling_price(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenDirectSellingPriceResult, JsValue> {
    let proof_vec = proof.to_vec();
    
    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, price_option) = Drive::verify_token_direct_selling_price(
        &proof_vec,
        token_id_bytes,
        verify_subset_of_proof,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    Ok(VerifyTokenDirectSellingPriceResult {
        root_hash: root_hash.to_vec(),
        price: price_option,
    })
}