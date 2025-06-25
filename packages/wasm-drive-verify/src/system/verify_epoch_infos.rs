use crate::utils::getters::VecU8ToUint8Array;
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::{Array, Uint8Array};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyEpochInfosResult {
    root_hash: Vec<u8>,
    epoch_infos: JsValue,
}

#[wasm_bindgen]
impl VerifyEpochInfosResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn epoch_infos(&self) -> JsValue {
        self.epoch_infos.clone()
    }
}

#[wasm_bindgen(js_name = "verifyEpochInfos")]
pub fn verify_epoch_infos(
    proof: &Uint8Array,
    current_epoch: u16,
    start_epoch: Option<u16>,
    count: u16,
    ascending: bool,
    platform_version_number: u32,
) -> Result<VerifyEpochInfosResult, JsValue> {
    let proof_vec = proof.to_vec();

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, epoch_infos_vec) = Drive::verify_epoch_infos(
        &proof_vec,
        current_epoch,
        start_epoch,
        count,
        ascending,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert Vec<ExtendedEpochInfo> to JS array
    let js_array = Array::new();
    for epoch_info in epoch_infos_vec {
        let epoch_info_json = serde_json::json!({
            "index": epoch_info.index(),
            "firstBlockHeight": epoch_info.first_block_height(),
            "firstCoreBlockHeight": epoch_info.first_core_block_height(),
            "firstBlockTime": epoch_info.first_block_time(),
            "feeMultiplierPermille": epoch_info.fee_multiplier_permille(),
            "protocolVersion": epoch_info.protocol_version(),
        });

        let js_value = to_value(&epoch_info_json).map_err(|e| {
            JsValue::from_str(&format!("Failed to convert epoch info to JsValue: {:?}", e))
        })?;
        js_array.push(&js_value);
    }

    Ok(VerifyEpochInfosResult {
        root_hash: root_hash.to_vec(),
        epoch_infos: js_array.into(),
    })
}
