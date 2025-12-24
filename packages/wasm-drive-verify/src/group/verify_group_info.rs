use crate::utils::getters::VecU8ToUint8Array;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::Uint8Array;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyGroupInfoResult {
    root_hash: Vec<u8>,
    group: JsValue,
}

#[wasm_bindgen]
impl VerifyGroupInfoResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn group(&self) -> JsValue {
        self.group.clone()
    }
}

#[wasm_bindgen(js_name = "verifyGroupInfo")]
pub fn verify_group_info(
    proof: &Uint8Array,
    contract_id: &Uint8Array,
    group_contract_position: u16,
    is_proof_subset: bool,
    platform_version_number: u32,
) -> Result<VerifyGroupInfoResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, group_option) = Drive::verify_group_info(
        &proof_vec,
        Identifier::from(contract_id_bytes),
        group_contract_position,
        is_proof_subset,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert Option<Group> to JavaScript value
    let group_js = match group_option {
        Some(group) => {
            let group_json = serde_json::to_value(&group)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize group: {:?}", e)))?;
            to_value(&group_json).map_err(|e| {
                JsValue::from_str(&format!("Failed to convert group to JsValue: {:?}", e))
            })?
        }
        None => JsValue::null(),
    };

    Ok(VerifyGroupInfoResult {
        root_hash: root_hash.to_vec(),
        group: group_js,
    })
}
