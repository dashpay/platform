use crate::utils::getters::VecU8ToUint8Array;
use dpp::identity::Purpose;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::{Array, Uint8Array};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyIdentitiesContractKeysResult {
    root_hash: Vec<u8>,
    keys: JsValue,
}

#[wasm_bindgen]
impl VerifyIdentitiesContractKeysResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn keys(&self) -> JsValue {
        self.keys.clone()
    }
}

#[wasm_bindgen(js_name = "verifyIdentitiesContractKeys")]
pub fn verify_identities_contract_keys(
    proof: &Uint8Array,
    identity_ids: &Array,
    contract_id: &Uint8Array,
    document_type_name: Option<String>,
    purposes: &Array,
    is_proof_subset: bool,
    platform_version_number: u32,
) -> Result<VerifyIdentitiesContractKeysResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    // Convert identity_ids array
    let mut identity_ids_vec = Vec::new();
    for i in 0..identity_ids.length() {
        let id_array = identity_ids
            .get(i)
            .dyn_into::<Uint8Array>()
            .map_err(|_| JsValue::from_str("Invalid identity_id in array"))?;
        let id_bytes: [u8; 32] = id_array
            .to_vec()
            .try_into()
            .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;
        identity_ids_vec.push(id_bytes);
    }

    // Convert purposes array
    let mut purposes_vec = Vec::new();
    for i in 0..purposes.length() {
        let purpose_num = purposes
            .get(i)
            .as_f64()
            .ok_or_else(|| JsValue::from_str("Invalid purpose value"))?;
        let purpose = match purpose_num as u8 {
            0 => Purpose::AUTHENTICATION,
            1 => Purpose::ENCRYPTION,
            2 => Purpose::DECRYPTION,
            3 => Purpose::TRANSFER,
            4 => Purpose::SYSTEM,
            5 => Purpose::VOTING,
            6 => Purpose::OWNER,
            _ => return Err(JsValue::from_str("Invalid purpose value")),
        };
        purposes_vec.push(purpose);
    }

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, keys) = Drive::verify_identities_contract_keys(
        &proof_vec,
        &identity_ids_vec,
        &contract_id_bytes,
        document_type_name,
        purposes_vec,
        is_proof_subset,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert IdentitiesContractKeys to JavaScript object
    let keys_js = to_value(&keys)
        .map_err(|e| JsValue::from_str(&format!("Failed to convert keys to JsValue: {:?}", e)))?;

    Ok(VerifyIdentitiesContractKeysResult {
        root_hash: root_hash.to_vec(),
        keys: keys_js,
    })
}
