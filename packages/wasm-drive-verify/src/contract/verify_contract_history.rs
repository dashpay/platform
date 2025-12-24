use crate::utils::getters::VecU8ToUint8Array;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::{Object, Reflect, Uint8Array};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyContractHistoryResult {
    root_hash: Vec<u8>,
    contract_history: JsValue,
}

#[wasm_bindgen]
impl VerifyContractHistoryResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn contract_history(&self) -> JsValue {
        self.contract_history.clone()
    }
}

#[wasm_bindgen(js_name = "verifyContractHistory")]
pub fn verify_contract_history(
    proof: &Uint8Array,
    contract_id: &Uint8Array,
    start_at_date: u64,
    limit: Option<u16>,
    offset: Option<u16>,
    platform_version_number: u32,
) -> Result<VerifyContractHistoryResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, contract_history_option) = Drive::verify_contract_history(
        &proof_vec,
        contract_id_bytes,
        start_at_date,
        limit,
        offset,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let contract_history_js = match contract_history_option {
        Some(history_map) => {
            let js_obj = Object::new();
            for (date, contract) in history_map {
                let contract_js = to_value(&contract).map_err(|e| {
                    JsValue::from_str(&format!("Failed to serialize contract: {:?}", e))
                })?;

                Reflect::set(&js_obj, &JsValue::from_str(&date.to_string()), &contract_js)
                    .map_err(|_| JsValue::from_str("Failed to set contract in history object"))?;
            }
            js_obj.into()
        }
        None => JsValue::NULL,
    };

    Ok(VerifyContractHistoryResult {
        root_hash: root_hash.to_vec(),
        contract_history: contract_history_js,
    })
}
