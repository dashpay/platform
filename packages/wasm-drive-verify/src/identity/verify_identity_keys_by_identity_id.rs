use dpp::identity::PartialIdentity;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::Drive;
use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

// Helper function to convert PartialIdentity to JS object
fn partial_identity_to_js(identity: &PartialIdentity) -> Result<JsValue, JsValue> {
    let obj = Object::new();
    
    // Set id
    let id_array = Uint8Array::from(identity.id.as_slice());
    Reflect::set(&obj, &JsValue::from_str("id"), &id_array)
        .map_err(|_| JsValue::from_str("Failed to set id"))?;
    
    // Set loadedPublicKeys
    let keys_obj = Object::new();
    for (key_id, _public_key) in &identity.loaded_public_keys {
        let key_obj = Object::new();
        
        // Set key properties
        Reflect::set(&key_obj, &JsValue::from_str("id"), &JsValue::from_f64(*key_id as f64))
            .map_err(|_| JsValue::from_str("Failed to set key id"))?;
        
        // For now, we'll add a placeholder for the full key data
        // TODO: Implement full IdentityPublicKey serialization
        Reflect::set(&key_obj, &JsValue::from_str("data"), &JsValue::from_str("[Key data not yet implemented]"))
            .map_err(|_| JsValue::from_str("Failed to set key data"))?;
        
        Reflect::set(&keys_obj, &JsValue::from_str(&key_id.to_string()), &key_obj)
            .map_err(|_| JsValue::from_str("Failed to set key in map"))?;
    }
    Reflect::set(&obj, &JsValue::from_str("loadedPublicKeys"), &keys_obj)
        .map_err(|_| JsValue::from_str("Failed to set loadedPublicKeys"))?;
    
    // Set balance
    match identity.balance {
        Some(balance) => {
            Reflect::set(&obj, &JsValue::from_str("balance"), &JsValue::from_f64(balance as f64))
                .map_err(|_| JsValue::from_str("Failed to set balance"))?;
        }
        None => {
            Reflect::set(&obj, &JsValue::from_str("balance"), &JsValue::NULL)
                .map_err(|_| JsValue::from_str("Failed to set balance to null"))?;
        }
    }
    
    // Set revision
    match identity.revision {
        Some(revision) => {
            Reflect::set(&obj, &JsValue::from_str("revision"), &JsValue::from_f64(revision as f64))
                .map_err(|_| JsValue::from_str("Failed to set revision"))?;
        }
        None => {
            Reflect::set(&obj, &JsValue::from_str("revision"), &JsValue::NULL)
                .map_err(|_| JsValue::from_str("Failed to set revision to null"))?;
        }
    }
    
    // Set notFoundPublicKeys
    let not_found_array = Array::new();
    for key_id in &identity.not_found_public_keys {
        not_found_array.push(&JsValue::from_f64(*key_id as f64));
    }
    Reflect::set(&obj, &JsValue::from_str("notFoundPublicKeys"), &not_found_array)
        .map_err(|_| JsValue::from_str("Failed to set notFoundPublicKeys"))?;
    
    Ok(obj.into())
}

#[wasm_bindgen]
pub struct VerifyIdentityKeysByIdentityIdResult {
    root_hash: Vec<u8>,
    identity: JsValue,
}

#[wasm_bindgen]
impl VerifyIdentityKeysByIdentityIdResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn identity(&self) -> JsValue {
        self.identity.clone()
    }
}

#[wasm_bindgen(js_name = "verifyIdentityKeysByIdentityId")]
pub fn verify_identity_keys_by_identity_id(
    proof: &Uint8Array,
    identity_id: &Uint8Array,
    specific_key_ids: Option<Array>,
    with_revision: bool,
    with_balance: bool,
    is_proof_subset: bool,
    limit: Option<u16>,
    offset: Option<u16>,
    platform_version_number: u32,
) -> Result<VerifyIdentityKeysByIdentityIdResult, JsValue> {
    let proof_vec = proof.to_vec();

    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

    // Create the key request type based on whether specific keys are requested
    let request_type = if let Some(keys_array) = specific_key_ids {
        let mut keys_vec = Vec::new();
        for i in 0..keys_array.length() {
            let key_id = keys_array
                .get(i)
                .as_f64()
                .ok_or_else(|| JsValue::from_str("Invalid key ID"))?
                as u32;
            keys_vec.push(key_id);
        }
        KeyRequestType::SpecificKeys(keys_vec)
    } else {
        KeyRequestType::AllKeys
    };

    let key_request = IdentityKeysRequest {
        identity_id: identity_id_bytes,
        request_type,
        limit,
        offset,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, identity_option) = Drive::verify_identity_keys_by_identity_id(
        &proof_vec,
        key_request,
        with_revision,
        with_balance,
        is_proof_subset,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    let identity_js = match identity_option {
        Some(identity) => {
            partial_identity_to_js(&identity)?
        }
        None => JsValue::NULL,
    };

    Ok(VerifyIdentityKeysByIdentityIdResult {
        root_hash: root_hash.to_vec(),
        identity: identity_js,
    })
}
