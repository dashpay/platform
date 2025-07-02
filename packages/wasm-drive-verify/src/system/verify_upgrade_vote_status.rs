use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::serialization::identifier_to_base58;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyUpgradeVoteStatusResult {
    root_hash: Vec<u8>,
    vote_status: JsValue,
}

#[wasm_bindgen]
impl VerifyUpgradeVoteStatusResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn vote_status(&self) -> JsValue {
        self.vote_status.clone()
    }
}

#[wasm_bindgen(js_name = "verifyUpgradeVoteStatus")]
pub fn verify_upgrade_vote_status(
    proof: &Uint8Array,
    start_protx_hash: Option<Uint8Array>,
    count: u16,
    platform_version_number: u32,
) -> Result<VerifyUpgradeVoteStatusResult, JsValue> {
    let proof_vec = proof.to_vec();

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    // Convert Uint8Array to Option<[u8; 32]>
    let start_protx_hash_array = match start_protx_hash {
        Some(hash) => {
            let hash_vec = hash.to_vec();
            if hash_vec.len() != 32 {
                return Err(JsValue::from_str(
                    "start_protx_hash must be exactly 32 bytes",
                ));
            }
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&hash_vec);
            Some(hash_array)
        }
        None => None,
    };

    let (root_hash, vote_status_map) = Drive::verify_upgrade_vote_status(
        &proof_vec,
        start_protx_hash_array,
        count,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert BTreeMap<[u8; 32], ProtocolVersion> to JS object
    let js_obj = Object::new();
    for (protx_hash, protocol_version) in vote_status_map {
        let base58_key = identifier_to_base58(&protx_hash);
        let value = JsValue::from_f64(protocol_version as f64);

        Reflect::set(&js_obj, &JsValue::from_str(&base58_key), &value)
            .map_err(|_| JsValue::from_str("Failed to set vote status entry in result object"))?;
    }

    Ok(VerifyUpgradeVoteStatusResult {
        root_hash: root_hash.to_vec(),
        vote_status: js_obj.into(),
    })
}
