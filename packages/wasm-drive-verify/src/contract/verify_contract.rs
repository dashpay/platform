use crate::utils::getters::VecU8ToUint8Array;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::Uint8Array;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyContractResult {
    root_hash: Vec<u8>,
    contract: JsValue,
}

#[wasm_bindgen]
impl VerifyContractResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn contract(&self) -> JsValue {
        self.contract.clone()
    }
}

#[wasm_bindgen(js_name = "verifyContract")]
pub fn verify_contract(
    proof: &Uint8Array,
    contract_known_keeps_history: Option<bool>,
    is_proof_subset: bool,
    in_multiple_contract_proof_form: bool,
    contract_id: &Uint8Array,
    platform_version_number: u32,
) -> Result<VerifyContractResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, contract_option) = Drive::verify_contract(
        &proof_vec,
        contract_known_keeps_history,
        is_proof_subset,
        in_multiple_contract_proof_form,
        contract_id_bytes,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let contract_js = match contract_option {
        Some(contract) => to_value(&contract)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize contract: {:?}", e)))?,
        None => JsValue::NULL,
    };

    Ok(VerifyContractResult {
        root_hash: root_hash.to_vec(),
        contract: contract_js,
    })
}
