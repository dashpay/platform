use crate::utils::getters::VecU8ToUint8Array;
use dpp::identity::identity_public_key::IdentityPublicKey;
use dpp::identity::PartialIdentity;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::Drive;
use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

// Helper function to convert PartialIdentity to JS object
pub fn partial_identity_to_js(identity: &PartialIdentity) -> Result<JsValue, JsValue> {
    let obj = Object::new();

    // Set id
    let id_array = Uint8Array::from(identity.id.as_slice());
    Reflect::set(&obj, &JsValue::from_str("id"), &id_array)
        .map_err(|_| JsValue::from_str("Failed to set id"))?;

    // Set loadedPublicKeys
    let keys_obj = Object::new();
    for (key_id, _public_key) in &identity.loaded_public_keys {
        let key_obj = Object::new();

        // Serialize the full IdentityPublicKey
        let serialized_key = serialize_identity_public_key(_public_key)?;

        // Merge the serialized key properties into the key object
        let key_keys = Object::keys(&serialized_key);
        for i in 0..key_keys.length() {
            let prop_name = key_keys.get(i);
            let prop_value = Reflect::get(&serialized_key, &prop_name)
                .map_err(|_| JsValue::from_str("Failed to get key property"))?;
            Reflect::set(&key_obj, &prop_name, &prop_value)
                .map_err(|_| JsValue::from_str("Failed to set key property"))?;
        }

        Reflect::set(&keys_obj, &JsValue::from_str(&key_id.to_string()), &key_obj)
            .map_err(|_| JsValue::from_str("Failed to set key in map"))?;
    }
    Reflect::set(&obj, &JsValue::from_str("loadedPublicKeys"), &keys_obj)
        .map_err(|_| JsValue::from_str("Failed to set loadedPublicKeys"))?;

    // Set balance
    match identity.balance {
        Some(balance) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("balance"),
                &JsValue::from_str(&balance.to_string()),
            )
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
            Reflect::set(
                &obj,
                &JsValue::from_str("revision"),
                &JsValue::from_str(&revision.to_string()),
            )
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
        not_found_array.push(&JsValue::from_str(&key_id.to_string()));
    }
    Reflect::set(
        &obj,
        &JsValue::from_str("notFoundPublicKeys"),
        &not_found_array,
    )
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
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
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
                .as_string()
                .ok_or_else(|| JsValue::from_str("Key ID must be a string"))?
                .parse::<u32>()
                .map_err(|_| JsValue::from_str("Invalid key ID number"))?;
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
        Some(identity) => partial_identity_to_js(&identity)?,
        None => JsValue::NULL,
    };

    Ok(VerifyIdentityKeysByIdentityIdResult {
        root_hash: root_hash.to_vec(),
        identity: identity_js,
    })
}

// Helper function to serialize IdentityPublicKey to JS object
fn serialize_identity_public_key(key: &IdentityPublicKey) -> Result<Object, JsValue> {
    let obj = Object::new();

    match key {
        IdentityPublicKey::V0(key_v0) => {
            // Set id
            Reflect::set(&obj, &JsValue::from_str("id"), &JsValue::from(key_v0.id))
                .map_err(|_| JsValue::from_str("Failed to set key id"))?;

            // Set purpose (as number)
            Reflect::set(
                &obj,
                &JsValue::from_str("purpose"),
                &JsValue::from(key_v0.purpose as u8),
            )
            .map_err(|_| JsValue::from_str("Failed to set purpose"))?;

            // Set security level (as number)
            Reflect::set(
                &obj,
                &JsValue::from_str("securityLevel"),
                &JsValue::from(key_v0.security_level as u8),
            )
            .map_err(|_| JsValue::from_str("Failed to set security level"))?;

            // Set contract bounds (optional)
            match &key_v0.contract_bounds {
                Some(bounds) => {
                    let bounds_obj = Object::new();
                    match bounds {
                        dpp::identity::identity_public_key::contract_bounds::ContractBounds::SingleContract { id } => {
                            Reflect::set(&bounds_obj, &JsValue::from_str("type"), &JsValue::from_str("SingleContract"))
                                .map_err(|_| JsValue::from_str("Failed to set bounds type"))?;
                            let id_array = Uint8Array::from(id.as_slice());
                            Reflect::set(&bounds_obj, &JsValue::from_str("id"), &id_array)
                                .map_err(|_| JsValue::from_str("Failed to set bounds id"))?;
                        }
                        dpp::identity::identity_public_key::contract_bounds::ContractBounds::SingleContractDocumentType { id, document_type_name } => {
                            Reflect::set(&bounds_obj, &JsValue::from_str("type"), &JsValue::from_str("SingleContractDocumentType"))
                                .map_err(|_| JsValue::from_str("Failed to set bounds type"))?;
                            let id_array = Uint8Array::from(id.as_slice());
                            Reflect::set(&bounds_obj, &JsValue::from_str("id"), &id_array)
                                .map_err(|_| JsValue::from_str("Failed to set bounds id"))?;
                            Reflect::set(&bounds_obj, &JsValue::from_str("documentTypeName"), &JsValue::from_str(document_type_name))
                                .map_err(|_| JsValue::from_str("Failed to set document type name"))?;
                        }
                    }
                    Reflect::set(&obj, &JsValue::from_str("contractBounds"), &bounds_obj)
                        .map_err(|_| JsValue::from_str("Failed to set contract bounds"))?;
                }
                None => {
                    Reflect::set(&obj, &JsValue::from_str("contractBounds"), &JsValue::NULL)
                        .map_err(|_| JsValue::from_str("Failed to set contract bounds to null"))?;
                }
            }

            // Set key type (as number)
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from(key_v0.key_type as u8),
            )
            .map_err(|_| JsValue::from_str("Failed to set key type"))?;

            // Set read only flag
            Reflect::set(
                &obj,
                &JsValue::from_str("readOnly"),
                &JsValue::from_bool(key_v0.read_only),
            )
            .map_err(|_| JsValue::from_str("Failed to set read only"))?;

            // Set key data (as Uint8Array)
            let data_array = Uint8Array::from(key_v0.data.as_slice());
            Reflect::set(&obj, &JsValue::from_str("data"), &data_array)
                .map_err(|_| JsValue::from_str("Failed to set key data"))?;

            // Set disabled_at (optional timestamp)
            match key_v0.disabled_at {
                Some(timestamp) => {
                    Reflect::set(
                        &obj,
                        &JsValue::from_str("disabledAt"),
                        &JsValue::from_str(&timestamp.to_string()),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set disabled at"))?;
                }
                None => {
                    Reflect::set(&obj, &JsValue::from_str("disabledAt"), &JsValue::NULL)
                        .map_err(|_| JsValue::from_str("Failed to set disabled at to null"))?;
                }
            }
        }
    }

    Ok(obj)
}
