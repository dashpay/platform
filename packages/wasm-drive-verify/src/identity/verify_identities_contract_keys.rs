use crate::utils::getters::VecU8ToUint8Array;
use dpp::identity::Purpose;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::{Array, Uint8Array};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

fn purpose_from_js_number(value: f64) -> Result<Purpose, ()> {
    if !value.is_finite() || value.fract() != 0.0 || !(0.0..=u8::MAX as f64).contains(&value) {
        return Err(());
    }

    Purpose::try_from(value as u8).map_err(|_| ())
}

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
        let purpose = purpose_from_js_number(purpose_num)
            .map_err(|_| JsValue::from_str("Invalid purpose value"))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purpose_parser_accepts_only_exact_discriminants() {
        for (value, expected) in [
            (0.0, Purpose::AUTHENTICATION),
            (1.0, Purpose::ENCRYPTION),
            (2.0, Purpose::DECRYPTION),
            (3.0, Purpose::TRANSFER),
            (4.0, Purpose::SYSTEM),
            (5.0, Purpose::VOTING),
            (6.0, Purpose::OWNER),
        ] {
            assert_eq!(purpose_from_js_number(value), Ok(expected));
        }
    }

    #[test]
    fn purpose_parser_rejects_lossy_or_unknown_values() {
        for value in [
            -1.0,
            -0.5,
            0.9,
            1.9,
            3.9,
            5.9,
            6.9,
            7.0,
            255.0,
            256.0,
            f64::NAN,
            f64::NEG_INFINITY,
            f64::INFINITY,
        ] {
            assert_eq!(
                purpose_from_js_number(value),
                Err(()),
                "{value:?} should be rejected"
            );
        }
    }
}
