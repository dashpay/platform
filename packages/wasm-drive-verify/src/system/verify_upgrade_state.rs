use crate::utils::getters::VecU8ToUint8Array;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyUpgradeStateResult {
    root_hash: Vec<u8>,
    upgrade_state: JsValue,
}

#[wasm_bindgen]
impl VerifyUpgradeStateResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn upgrade_state(&self) -> JsValue {
        self.upgrade_state.clone()
    }
}

#[wasm_bindgen(js_name = "verifyUpgradeState")]
pub fn verify_upgrade_state(
    proof: &Uint8Array,
    platform_version_number: u32,
) -> Result<VerifyUpgradeStateResult, JsValue> {
    let proof_vec = proof.to_vec();

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, upgrade_state_map) = Drive::verify_upgrade_state(&proof_vec, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert IntMap<ProtocolVersion, u64> to JS object
    let js_obj = Object::new();
    for (protocol_version, count) in upgrade_state_map {
        let key = protocol_version.to_string();
        let value = JsValue::from_f64(count as f64);

        Reflect::set(&js_obj, &JsValue::from_str(&key), &value)
            .map_err(|_| JsValue::from_str("Failed to set upgrade state entry in result object"))?;
    }

    Ok(VerifyUpgradeStateResult {
        root_hash: root_hash.to_vec(),
        upgrade_state: js_obj.into(),
    })
}
