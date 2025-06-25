use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::serialization::{bytes_to_base58, identifier_to_base58};
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::verify::RootHash;
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyIdentityIdsByUniquePublicKeyHashesResult {
    root_hash: Vec<u8>,
    identity_ids: JsValue,
}

#[wasm_bindgen]
impl VerifyIdentityIdsByUniquePublicKeyHashesResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn identity_ids(&self) -> JsValue {
        self.identity_ids.clone()
    }
}

// Vec variant - returns array of tuples [publicKeyHash, identityId]
#[wasm_bindgen(js_name = "verifyIdentityIdsByUniquePublicKeyHashesVec")]
pub fn verify_identity_ids_by_unique_public_key_hashes_vec(
    proof: &Uint8Array,
    is_proof_subset: bool,
    public_key_hashes: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyIdentityIdsByUniquePublicKeyHashesResult, JsValue> {
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

    let (root_hash, identity_ids_vec): (RootHash, Vec<([u8; 20], Option<[u8; 32]>)>) =
        Drive::verify_identity_ids_by_unique_public_key_hashes(
            &proof_vec,
            is_proof_subset,
            &public_key_hashes_vec,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (hash, id_option) in identity_ids_vec {
        let tuple_array = Array::new();

        // Add public key hash as Uint8Array
        let hash_uint8 = Uint8Array::from(&hash[..]);
        tuple_array.push(&hash_uint8);

        // Add identity ID
        match id_option {
            Some(id) => {
                let id_uint8 = Uint8Array::from(&id[..]);
                tuple_array.push(&id_uint8);
            }
            None => {
                tuple_array.push(&JsValue::NULL);
            }
        }

        js_array.push(&tuple_array);
    }

    Ok(VerifyIdentityIdsByUniquePublicKeyHashesResult {
        root_hash: root_hash.to_vec(),
        identity_ids: js_array.into(),
    })
}

// BTreeMap variant - returns object with public key hash (base58) as key
#[wasm_bindgen(js_name = "verifyIdentityIdsByUniquePublicKeyHashesMap")]
pub fn verify_identity_ids_by_unique_public_key_hashes_map(
    proof: &Uint8Array,
    is_proof_subset: bool,
    public_key_hashes: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyIdentityIdsByUniquePublicKeyHashesResult, JsValue> {
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

    let (root_hash, identity_ids_map): (RootHash, BTreeMap<[u8; 20], Option<[u8; 32]>>) =
        Drive::verify_identity_ids_by_unique_public_key_hashes(
            &proof_vec,
            is_proof_subset,
            &public_key_hashes_vec,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with base58 keys
    let js_obj = Object::new();
    for (hash, id_option) in identity_ids_map {
        let base58_key = bytes_to_base58(&hash);

        let id_js = match id_option {
            Some(id) => {
                let id_base58 = identifier_to_base58(&id);
                JsValue::from_str(&id_base58)
            }
            None => JsValue::NULL,
        };

        Reflect::set(&js_obj, &JsValue::from_str(&base58_key), &id_js)
            .map_err(|_| JsValue::from_str("Failed to set identity ID in result object"))?;
    }

    Ok(VerifyIdentityIdsByUniquePublicKeyHashesResult {
        root_hash: root_hash.to_vec(),
        identity_ids: js_obj.into(),
    })
}
