use crate::utils::getters::VecU8ToUint8Array;
use dpp::prelude::CoreBlockHeight;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTotalCreditsInSystemResult {
    root_hash: Vec<u8>,
    total_credits: u64,
}

#[wasm_bindgen]
impl VerifyTotalCreditsInSystemResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn total_credits(&self) -> u64 {
        self.total_credits
    }
}

#[wasm_bindgen(js_name = "verifyTotalCreditsInSystem")]
pub fn verify_total_credits_in_system(
    proof: &Uint8Array,
    core_subsidy_halving_interval: u32,
    activation_core_height: u32,
    current_core_height: u32,
    platform_version_number: u32,
) -> Result<VerifyTotalCreditsInSystemResult, JsValue> {
    let proof_vec = proof.to_vec();

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    // Create a closure that returns the activation core height
    let request_activation_core_height =
        || -> Result<CoreBlockHeight, drive::error::Error> { Ok(activation_core_height) };

    let (root_hash, total_credits) = Drive::verify_total_credits_in_system(
        &proof_vec,
        core_subsidy_halving_interval,
        request_activation_core_height,
        current_core_height,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    Ok(VerifyTotalCreditsInSystemResult {
        root_hash: root_hash.to_vec(),
        total_credits,
    })
}
