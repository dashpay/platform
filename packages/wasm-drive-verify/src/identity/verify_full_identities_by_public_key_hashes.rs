use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::serialization::{bytes_to_base58, identity_to_js_value};
use dpp::prelude::Identity;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::verify::RootHash;
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyFullIdentitiesByPublicKeyHashesResult {
    root_hash: Vec<u8>,
    identities: JsValue,
}

#[wasm_bindgen]
impl VerifyFullIdentitiesByPublicKeyHashesResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn identities(&self) -> JsValue {
        self.identities.clone()
    }
}

// Vec variant - returns array of tuples [publicKeyHash, identity]
#[wasm_bindgen(js_name = "verifyFullIdentitiesByPublicKeyHashesVec")]
pub fn verify_full_identities_by_public_key_hashes_vec(
    proof: &Uint8Array,
    public_key_hashes: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyFullIdentitiesByPublicKeyHashesResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse public key hashes from JS array
    let hashes_array: Array = public_key_hashes
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("public_key_hashes must be an array"))?;

    let mut public_key_hashes_vec = Vec::new();
    for i in 0..hashes_array.length() {
        let hash_array = hashes_array.get(i);
        let hash_uint8: Uint8Array = hash_array
            .dyn_into()
            .map_err(|_| JsValue::from_str("Each public key hash must be a Uint8Array"))?;

        let hash_vec = hash_uint8.to_vec();
        let hash_bytes: [u8; 20] = hash_vec
            .try_into()
            .map_err(|_| JsValue::from_str("Invalid public key hash length. Expected 20 bytes."))?;

        public_key_hashes_vec.push(hash_bytes);
    }

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, identities_vec): (RootHash, Vec<([u8; 20], Option<Identity>)>) =
        Drive::verify_full_identities_by_public_key_hashes(
            &proof_vec,
            &public_key_hashes_vec,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (hash, identity_option) in identities_vec {
        let tuple_array = Array::new();

        // Add public key hash as Uint8Array
        let hash_uint8 = Uint8Array::from(&hash[..]);
        tuple_array.push(&hash_uint8);

        // Add identity
        match identity_option {
            Some(identity) => {
                let identity_js = identity_to_js_value(identity)?;
                tuple_array.push(&identity_js);
            }
            None => {
                tuple_array.push(&JsValue::NULL);
            }
        }

        js_array.push(&tuple_array);
    }

    Ok(VerifyFullIdentitiesByPublicKeyHashesResult {
        root_hash: root_hash.to_vec(),
        identities: js_array.into(),
    })
}

// BTreeMap variant - returns object with public key hash (base58) as key
#[wasm_bindgen(js_name = "verifyFullIdentitiesByPublicKeyHashesMap")]
pub fn verify_full_identities_by_public_key_hashes_map(
    proof: &Uint8Array,
    public_key_hashes: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyFullIdentitiesByPublicKeyHashesResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse public key hashes from JS array
    let hashes_array: Array = public_key_hashes
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("public_key_hashes must be an array"))?;

    let mut public_key_hashes_vec = Vec::new();
    for i in 0..hashes_array.length() {
        let hash_array = hashes_array.get(i);
        let hash_uint8: Uint8Array = hash_array
            .dyn_into()
            .map_err(|_| JsValue::from_str("Each public key hash must be a Uint8Array"))?;

        let hash_vec = hash_uint8.to_vec();
        let hash_bytes: [u8; 20] = hash_vec
            .try_into()
            .map_err(|_| JsValue::from_str("Invalid public key hash length. Expected 20 bytes."))?;

        public_key_hashes_vec.push(hash_bytes);
    }

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, identities_map): (RootHash, BTreeMap<[u8; 20], Option<Identity>>) =
        Drive::verify_full_identities_by_public_key_hashes(
            &proof_vec,
            &public_key_hashes_vec,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with base58 keys
    let js_obj = Object::new();
    for (hash, identity_option) in identities_map {
        let base58_key = bytes_to_base58(&hash);

        let identity_js = match identity_option {
            Some(identity) => identity_to_js_value(identity)?,
            None => JsValue::NULL,
        };

        Reflect::set(&js_obj, &JsValue::from_str(&base58_key), &identity_js)
            .map_err(|_| JsValue::from_str("Failed to set identity in result object"))?;
    }

    Ok(VerifyFullIdentitiesByPublicKeyHashesResult {
        root_hash: root_hash.to_vec(),
        identities: js_obj.into(),
    })
}
