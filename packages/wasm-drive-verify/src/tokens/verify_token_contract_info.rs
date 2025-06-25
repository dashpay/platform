use crate::utils::getters::VecU8ToUint8Array;
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTokenContractInfoResult {
    root_hash: Vec<u8>,
    contract_info: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenContractInfoResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn contract_info(&self) -> JsValue {
        self.contract_info.clone()
    }
}

#[wasm_bindgen(js_name = "verifyTokenContractInfo")]
pub fn verify_token_contract_info(
    proof: &Uint8Array,
    token_id: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenContractInfoResult, JsValue> {
    let proof_vec = proof.to_vec();

    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, contract_info_option) = Drive::verify_token_contract_info(
        &proof_vec,
        token_id_bytes,
        verify_subset_of_proof,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let contract_info_js = match contract_info_option {
        Some(info) => {
            let obj = Object::new();

            // Convert TokenContractInfo fields to JS object
            Reflect::set(
                &obj,
                &JsValue::from_str("contractId"),
                &Uint8Array::from(info.contract_id().as_slice()),
            )
            .map_err(|_| JsValue::from_str("Failed to set contractId"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("tokenContractPosition"),
                &JsValue::from_str(&info.token_contract_position().to_string()),
            )
            .map_err(|_| JsValue::from_str("Failed to set tokenContractPosition"))?;

            obj.into()
        }
        None => JsValue::NULL,
    };

    Ok(VerifyTokenContractInfoResult {
        root_hash: root_hash.to_vec(),
        contract_info: contract_info_js,
    })
}
