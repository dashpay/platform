use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::verify::RootHash;
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
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
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
                &JsValue::from_str("tokenId"),
                &Uint8Array::from(&info.token_id[..]),
            )
            .map_err(|_| JsValue::from_str("Failed to set tokenId"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("owner"),
                &Uint8Array::from(&info.owner[..]),
            )
            .map_err(|_| JsValue::from_str("Failed to set owner"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("maxSupply"),
                &JsValue::from_f64(info.max_supply as f64),
            )
            .map_err(|_| JsValue::from_str("Failed to set maxSupply"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("burnAmount"),
                &JsValue::from_f64(info.burn_amount as f64),
            )
            .map_err(|_| JsValue::from_str("Failed to set burnAmount"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("totalSupply"),
                &JsValue::from_f64(info.total_supply as f64),
            )
            .map_err(|_| JsValue::from_str("Failed to set totalSupply"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("aggregatedIdentityBalance"),
                &JsValue::from_f64(info.aggregated_identity_balance as f64),
            )
            .map_err(|_| JsValue::from_str("Failed to set aggregatedIdentityBalance"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("contractId"),
                &Uint8Array::from(&info.contract_id[..]),
            )
            .map_err(|_| JsValue::from_str("Failed to set contractId"))?;

            obj.into()
        }
        None => JsValue::NULL,
    };

    Ok(VerifyTokenContractInfoResult {
        root_hash: root_hash.to_vec(),
        contract_info: contract_info_js,
    })
}
