//! Identity state transitions
//!
//! This module provides WASM bindings for identity-related state transitions including:
//! - Identity creation with asset lock proofs
//! - Identity top-up operations
//! - Identity updates (adding/removing keys, etc.)

use crate::error::to_js_error;
use dpp::serialization::PlatformDeserializable;
use dpp::identity::{Identity, IdentityV0, KeyID};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::{IdentityPublicKey, v0::IdentityPublicKeyV0};
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::{KeyType, Purpose, SecurityLevel};
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::prelude::{AssetLockProof, Identifier};
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::StateTransition;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;
use web_sys::js_sys::{Number, Uint8Array, Object, Reflect, Array};

/// Create a new identity with an asset lock proof
#[wasm_bindgen(js_name = createIdentity)]
pub fn create_identity(
    asset_lock_proof_bytes: &[u8],
    public_keys: JsValue,
) -> Result<Uint8Array, JsError> {
    // Parse public keys
    let public_keys = if public_keys.is_array() {
        parse_public_keys_from_js(&public_keys)?
    } else {
        return Err(JsError::new("public_keys must be an array"));
    };
    
    if public_keys.is_empty() {
        return Err(JsError::new("At least one public key is required"));
    }
    
    // Convert to public keys in creation
    let public_keys_in_creation: Vec<IdentityPublicKeyInCreation> = public_keys
        .into_iter()
        .map(|key| key.into())
        .collect();
    
    // Deserialize asset lock proof using our asset_lock module
    use crate::asset_lock::AssetLockProof as WasmAssetLockProof;
    let wasm_proof = WasmAssetLockProof::from_bytes(asset_lock_proof_bytes)?;
    let asset_lock_proof = wasm_proof.inner().clone();
    
    // Create the identity ID from asset lock proof
    let identity_id = asset_lock_proof.create_identifier()
        .map_err(|e| JsError::new(&format!("Failed to create identity ID: {}", e)))?;
    
    // Create the identity create transition
    let transition = IdentityCreateTransition::V0(IdentityCreateTransitionV0 {
        public_keys: public_keys_in_creation,
        asset_lock_proof,
        user_fee_increase: 0,
        signature: Default::default(),
        identity_id,
    });
    
    // Serialize the transition
    StateTransition::IdentityCreate(transition)
        .serialize_to_bytes()
        .map_err(to_js_error)
        .map(|bytes| Uint8Array::from(bytes.as_slice()))
}

/// Top up an existing identity with additional credits
#[wasm_bindgen(js_name = topUpIdentity)]
pub fn topup_identity(
    identity_id: &str,
    asset_lock_proof_bytes: &[u8],
) -> Result<Uint8Array, JsError> {
    // Parse identity ID
    let identity_id = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    // Deserialize asset lock proof using our asset_lock module
    use crate::asset_lock::AssetLockProof as WasmAssetLockProof;
    let wasm_proof = WasmAssetLockProof::from_bytes(asset_lock_proof_bytes)?;
    let asset_lock_proof = wasm_proof.inner().clone();
    
    // Create the identity top up transition
    let transition = IdentityTopUpTransition::V0(IdentityTopUpTransitionV0 {
        identity_id,
        asset_lock_proof,
        user_fee_increase: 0,
        signature: Default::default(),
    });
    
    // Serialize the transition
    StateTransition::IdentityTopUp(transition)
        .serialize_to_bytes()
        .map_err(to_js_error)
        .map(|bytes| Uint8Array::from(bytes.as_slice()))
}

/// Update an existing identity (add/remove keys, etc.)
#[wasm_bindgen]
pub fn update_identity(
    identity_id: &str,
    revision: u64,
    nonce: u64,
    _add_public_keys: JsValue,
    _disable_public_keys: JsValue,
    _public_keys_disabled_at: Option<u64>,
    signature_public_key_id: Number,
) -> Result<Uint8Array, JsError> {
    // Parse identity ID
    let identity_id = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    // Parse signature public key ID
    let signature_public_key_id = signature_public_key_id
        .as_f64()
        .ok_or_else(|| JsError::new("signature_public_key_id must be a number"))?;

    let signature_public_key_id: KeyID = if signature_public_key_id.is_finite()
        && signature_public_key_id >= KeyID::MIN as f64
        && signature_public_key_id <= (KeyID::MAX as f64)
    {
        signature_public_key_id as KeyID
    } else {
        return Err(JsError::new(&format!(
            "signature_public_key_id {} out of valid range",
            signature_public_key_id
        )));
    };

    // Parse public keys to add from JsValue
    let add_public_keys = if _add_public_keys.is_array() {
        parse_public_keys_in_creation_from_js(&_add_public_keys)?
    } else {
        vec![]
    };
    
    // Parse public key IDs to disable from JsValue
    let disable_public_keys = if _disable_public_keys.is_array() {
        parse_key_ids_from_js(&_disable_public_keys)?
    } else {
        vec![]
    };

    // Create the identity update transition
    let transition = IdentityUpdateTransition::V0(IdentityUpdateTransitionV0 {
        identity_id,
        revision,
        nonce,
        add_public_keys,
        disable_public_keys,
        user_fee_increase: 0,
        signature_public_key_id,
        signature: Default::default(),
    });

    // Serialize the transition
    StateTransition::IdentityUpdate(transition)
        .serialize_to_bytes()
        .map_err(to_js_error)
        .map(|bytes| Uint8Array::from(bytes.as_slice()))
}

/// Builder for creating identity state transitions
#[wasm_bindgen]
pub struct IdentityTransitionBuilder {
    identity_id: Option<Identifier>,
    revision: u64,
    add_public_keys: Vec<dpp::identity::IdentityPublicKey>,
    disable_public_keys: Vec<KeyID>,
}

#[wasm_bindgen]
impl IdentityTransitionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> IdentityTransitionBuilder {
        IdentityTransitionBuilder {
            identity_id: None,
            revision: 0,
            add_public_keys: vec![],
            disable_public_keys: vec![],
        }
    }

    #[wasm_bindgen(js_name = setIdentityId)]
    pub fn set_identity_id(&mut self, identity_id: &str) -> Result<(), JsError> {
        let id = Identifier::from_string(
            identity_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

        self.identity_id = Some(id);
        Ok(())
    }

    #[wasm_bindgen(js_name = setRevision)]
    pub fn set_revision(&mut self, revision: u64) {
        self.revision = revision;
    }

    #[wasm_bindgen(js_name = addPublicKey)]
    pub fn add_public_key(&mut self, public_key: JsValue) -> Result<(), JsError> {
        let key = parse_public_key_from_js(&public_key)?;
        self.add_public_keys.push(key);
        Ok(())
    }
    
    #[wasm_bindgen(js_name = addPublicKeys)]
    pub fn add_public_keys(&mut self, public_keys: JsValue) -> Result<(), JsError> {
        let keys = parse_public_keys_from_js(&public_keys)?;
        self.add_public_keys.extend(keys);
        Ok(())
    }
    
    #[wasm_bindgen(js_name = disablePublicKey)]
    pub fn disable_public_key(&mut self, key_id: u32) -> Result<(), JsError> {
        self.disable_public_keys.push(key_id as KeyID);
        Ok(())
    }
    
    #[wasm_bindgen(js_name = disablePublicKeys)]
    pub fn disable_public_keys(&mut self, key_ids: JsValue) -> Result<(), JsError> {
        let ids = parse_key_ids_from_js(&key_ids)?;
        self.disable_public_keys.extend(ids);
        Ok(())
    }

    #[wasm_bindgen(js_name = buildCreateTransition)]
    pub fn build_create_transition(
        self,
        asset_lock_proof_bytes: &[u8],
    ) -> Result<Uint8Array, JsError> {
        // Deserialize asset lock proof
        use crate::asset_lock::AssetLockProof as WasmAssetLockProof;
        let wasm_proof = WasmAssetLockProof::from_bytes(asset_lock_proof_bytes)?;
        let asset_lock_proof = wasm_proof.inner().clone();
        
        // Create the identity ID from asset lock proof
        let identity_id = asset_lock_proof.create_identifier()
            .map_err(|e| JsError::new(&format!("Failed to create identity ID: {}", e)))?;
        
        // Convert public keys to keys in creation
        let public_keys_in_creation: Vec<IdentityPublicKeyInCreation> = self.add_public_keys
            .into_iter()
            .map(|key| key.into())
            .collect();
        
        // Create the identity create transition
        let transition = IdentityCreateTransition::V0(IdentityCreateTransitionV0 {
            public_keys: public_keys_in_creation,
            asset_lock_proof,
            user_fee_increase: 0,
            signature: Default::default(),
            identity_id,
        });
        
        // Serialize the transition
        StateTransition::IdentityCreate(transition)
            .serialize_to_bytes()
            .map_err(to_js_error)
            .map(|bytes| Uint8Array::from(bytes.as_slice()))
    }

    #[wasm_bindgen(js_name = buildTopUpTransition)]
    pub fn build_topup_transition(
        self,
        asset_lock_proof_bytes: &[u8],
    ) -> Result<Uint8Array, JsError> {
        let identity_id = self
            .identity_id
            .ok_or_else(|| JsError::new("Identity ID must be set for top-up transition"))?;

        // Deserialize asset lock proof
        use crate::asset_lock::AssetLockProof as WasmAssetLockProof;
        let wasm_proof = WasmAssetLockProof::from_bytes(asset_lock_proof_bytes)?;
        let asset_lock_proof = wasm_proof.inner().clone();
        
        // Create the identity top up transition
        let transition = IdentityTopUpTransition::V0(IdentityTopUpTransitionV0 {
            identity_id,
            asset_lock_proof,
            user_fee_increase: 0,
            signature: Default::default(),
        });
        
        // Serialize the transition
        StateTransition::IdentityTopUp(transition)
            .serialize_to_bytes()
            .map_err(to_js_error)
            .map(|bytes| Uint8Array::from(bytes.as_slice()))
    }

    #[wasm_bindgen(js_name = buildUpdateTransition)]
    pub fn build_update_transition(
        self,
        nonce: u64,
        signature_public_key_id: Number,
        _public_keys_disabled_at: Option<u64>,
    ) -> Result<Uint8Array, JsError> {
        let identity_id = self
            .identity_id
            .ok_or_else(|| JsError::new("Identity ID must be set for update transition"))?;

        // Parse signature public key ID
        let signature_public_key_id = signature_public_key_id
            .as_f64()
            .ok_or_else(|| JsError::new("signature_public_key_id must be a number"))?;

        let signature_public_key_id: KeyID = if signature_public_key_id.is_finite()
            && signature_public_key_id >= KeyID::MIN as f64
            && signature_public_key_id <= (KeyID::MAX as f64)
        {
            signature_public_key_id as KeyID
        } else {
            return Err(JsError::new(&format!(
                "signature_public_key_id {} out of valid range",
                signature_public_key_id
            )));
        };

        // Create the identity update transition
        let transition = IdentityUpdateTransition::V0(IdentityUpdateTransitionV0 {
            identity_id,
            revision: self.revision,
            nonce,
            add_public_keys: self.add_public_keys
                .into_iter()
                .map(|key| key.into())
                .collect(),
            disable_public_keys: self.disable_public_keys,
            user_fee_increase: 0,
            signature_public_key_id,
            signature: Default::default(),
        });

        // Serialize the transition
        StateTransition::IdentityUpdate(transition)
            .serialize_to_bytes()
            .map_err(to_js_error)
            .map(|bytes| Uint8Array::from(bytes.as_slice()))
    }
}

/// Parse public keys from JavaScript array
fn parse_public_keys_from_js(js_array: &JsValue) -> Result<Vec<IdentityPublicKey>, JsError> {
    let array = js_array
        .dyn_ref::<Array>()
        .ok_or_else(|| JsError::new("Expected an array of public keys"))?;
    
    let mut keys = Vec::new();
    
    for i in 0..array.length() {
        let key_obj = array.get(i);
        let key = parse_public_key_from_js(&key_obj)?;
        keys.push(key);
    }
    
    Ok(keys)
}

/// Parse public keys for state transitions (IdentityPublicKeyInCreation)
fn parse_public_keys_in_creation_from_js(js_array: &JsValue) -> Result<Vec<IdentityPublicKeyInCreation>, JsError> {
    let array = js_array
        .dyn_ref::<Array>()
        .ok_or_else(|| JsError::new("Expected an array of public keys"))?;
    
    let mut keys = Vec::new();
    
    for i in 0..array.length() {
        let key_obj = array.get(i);
        let key = parse_public_key_in_creation_from_js(&key_obj)?;
        keys.push(key);
    }
    
    Ok(keys)
}

/// Parse a single public key from JavaScript object
fn parse_public_key_from_js(js_obj: &JsValue) -> Result<IdentityPublicKey, JsError> {
    let obj = js_obj
        .dyn_ref::<Object>()
        .ok_or_else(|| JsError::new("Expected a public key object"))?;
    
    // Get key ID
    let id = Reflect::get(obj, &"id".into())
        .map_err(|_| JsError::new("Missing 'id' field"))?
        .as_f64()
        .ok_or_else(|| JsError::new("'id' must be a number"))? as KeyID;
    
    // Get key type
    let key_type_str = Reflect::get(obj, &"type".into())
        .map_err(|_| JsError::new("Missing 'type' field"))?
        .as_string()
        .ok_or_else(|| JsError::new("'type' must be a string"))?;
    
    let key_type = match key_type_str.as_str() {
        "ECDSA_SECP256K1" => KeyType::ECDSA_SECP256K1,
        "BLS12_381" => KeyType::BLS12_381,
        "ECDSA_HASH160" => KeyType::ECDSA_HASH160,
        "BIP13_SCRIPT_HASH" => KeyType::BIP13_SCRIPT_HASH,
        "EDDSA_25519_HASH160" => KeyType::EDDSA_25519_HASH160,
        _ => return Err(JsError::new(&format!("Invalid key type: {}", key_type_str))),
    };
    
    // Get purpose
    let purpose_num = Reflect::get(obj, &"purpose".into())
        .map_err(|_| JsError::new("Missing 'purpose' field"))?
        .as_f64()
        .ok_or_else(|| JsError::new("'purpose' must be a number"))? as u8;
    
    let purpose = match purpose_num {
        0 => Purpose::AUTHENTICATION,
        1 => Purpose::ENCRYPTION,
        2 => Purpose::DECRYPTION,
        3 => Purpose::TRANSFER,
        5 => Purpose::SYSTEM,
        6 => Purpose::VOTING,
        _ => return Err(JsError::new(&format!("Invalid purpose: {}", purpose_num))),
    };
    
    // Get security level
    let security_level_num = Reflect::get(obj, &"securityLevel".into())
        .map_err(|_| JsError::new("Missing 'securityLevel' field"))?
        .as_f64()
        .ok_or_else(|| JsError::new("'securityLevel' must be a number"))? as u8;
    
    let security_level = match security_level_num {
        0 => SecurityLevel::MASTER,
        1 => SecurityLevel::CRITICAL,
        2 => SecurityLevel::HIGH,
        3 => SecurityLevel::MEDIUM,
        _ => return Err(JsError::new(&format!("Invalid security level: {}", security_level_num))),
    };
    
    // Get data
    let data_value = Reflect::get(obj, &"data".into())
        .map_err(|_| JsError::new("Missing 'data' field"))?;
    
    let data_array = data_value
        .dyn_ref::<Uint8Array>()
        .ok_or_else(|| JsError::new("'data' must be a Uint8Array"))?;
    
    let data = data_array.to_vec();
    
    // Get optional fields
    let read_only = Reflect::get(obj, &"readOnly".into())
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    let disabled_at = Reflect::get(obj, &"disabledAt".into())
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as u64);
    
    // Create the public key
    Ok(IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id,
        purpose,
        security_level,
        key_type,
        read_only,
        data: data.into(),
        disabled_at,
        contract_bounds: None,
    }))
}

/// Parse a single public key for creation from JavaScript object
fn parse_public_key_in_creation_from_js(js_obj: &JsValue) -> Result<IdentityPublicKeyInCreation, JsError> {
    let obj = js_obj
        .dyn_ref::<Object>()
        .ok_or_else(|| JsError::new("Expected a public key object"))?;
    
    // Get key ID
    let id = Reflect::get(obj, &"id".into())
        .map_err(|_| JsError::new("Missing 'id' field"))?
        .as_f64()
        .ok_or_else(|| JsError::new("'id' must be a number"))? as KeyID;
    
    // Get key type
    let key_type_str = Reflect::get(obj, &"type".into())
        .map_err(|_| JsError::new("Missing 'type' field"))?
        .as_string()
        .ok_or_else(|| JsError::new("'type' must be a string"))?;
    
    let key_type = match key_type_str.as_str() {
        "ECDSA_SECP256K1" => KeyType::ECDSA_SECP256K1,
        "BLS12_381" => KeyType::BLS12_381,
        "ECDSA_HASH160" => KeyType::ECDSA_HASH160,
        "BIP13_SCRIPT_HASH" => KeyType::BIP13_SCRIPT_HASH,
        "EDDSA_25519_HASH160" => KeyType::EDDSA_25519_HASH160,
        _ => return Err(JsError::new(&format!("Invalid key type: {}", key_type_str))),
    };
    
    // Get purpose
    let purpose_num = Reflect::get(obj, &"purpose".into())
        .map_err(|_| JsError::new("Missing 'purpose' field"))?
        .as_f64()
        .ok_or_else(|| JsError::new("'purpose' must be a number"))? as u8;
    
    let purpose = match purpose_num {
        0 => Purpose::AUTHENTICATION,
        1 => Purpose::ENCRYPTION,
        2 => Purpose::DECRYPTION,
        3 => Purpose::TRANSFER,
        5 => Purpose::SYSTEM,
        6 => Purpose::VOTING,
        _ => return Err(JsError::new(&format!("Invalid purpose: {}", purpose_num))),
    };
    
    // Get security level
    let security_level_num = Reflect::get(obj, &"securityLevel".into())
        .map_err(|_| JsError::new("Missing 'securityLevel' field"))?
        .as_f64()
        .ok_or_else(|| JsError::new("'securityLevel' must be a number"))? as u8;
    
    let security_level = match security_level_num {
        0 => SecurityLevel::MASTER,
        1 => SecurityLevel::CRITICAL,
        2 => SecurityLevel::HIGH,
        3 => SecurityLevel::MEDIUM,
        _ => return Err(JsError::new(&format!("Invalid security level: {}", security_level_num))),
    };
    
    // Get data
    let data_value = Reflect::get(obj, &"data".into())
        .map_err(|_| JsError::new("Missing 'data' field"))?;
    
    let data_array = data_value
        .dyn_ref::<Uint8Array>()
        .ok_or_else(|| JsError::new("'data' must be a Uint8Array"))?;
    
    let data = data_array.to_vec();
    
    // Get optional fields
    let read_only = Reflect::get(obj, &"readOnly".into())
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    // Create the public key for creation
    let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id,
        purpose,
        security_level,
        key_type,
        read_only,
        data: data.into(),
        disabled_at: None,
        contract_bounds: None,
    });
    
    Ok(public_key.into())
}

/// Parse key IDs from JavaScript array
fn parse_key_ids_from_js(js_array: &JsValue) -> Result<Vec<KeyID>, JsError> {
    let array = js_array
        .dyn_ref::<Array>()
        .ok_or_else(|| JsError::new("Expected an array of key IDs"))?;
    
    let mut key_ids = Vec::new();
    
    for i in 0..array.length() {
        let value = array.get(i);
        let key_id = value
            .as_f64()
            .ok_or_else(|| JsError::new("Key ID must be a number"))? as KeyID;
        key_ids.push(key_id);
    }
    
    Ok(key_ids)
}

/// Create a simple identity with a single ECDSA authentication key
#[wasm_bindgen(js_name = createBasicIdentity)]
pub fn create_basic_identity(
    asset_lock_proof_bytes: &[u8],
    public_key_data: &[u8],
) -> Result<Uint8Array, JsError> {
    // Create a basic authentication key
    let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::MASTER,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: public_key_data.to_vec().into(),
        disabled_at: None,
        contract_bounds: None,
    });
    
    let public_keys_in_creation = vec![public_key.into()];
    
    // Deserialize asset lock proof
    use crate::asset_lock::AssetLockProof as WasmAssetLockProof;
    let wasm_proof = WasmAssetLockProof::from_bytes(asset_lock_proof_bytes)?;
    let asset_lock_proof = wasm_proof.inner().clone();
    
    // Create the identity ID from asset lock proof
    let identity_id = asset_lock_proof.create_identifier()
        .map_err(|e| JsError::new(&format!("Failed to create identity ID: {}", e)))?;
    
    // Create the identity create transition
    let transition = IdentityCreateTransition::V0(IdentityCreateTransitionV0 {
        public_keys: public_keys_in_creation,
        asset_lock_proof,
        user_fee_increase: 0,
        signature: Default::default(),
        identity_id,
    });
    
    // Serialize the transition
    StateTransition::IdentityCreate(transition)
        .serialize_to_bytes()
        .map_err(to_js_error)
        .map(|bytes| Uint8Array::from(bytes.as_slice()))
}

/// Helper to create a standard identity public key configuration
#[wasm_bindgen(js_name = createStandardIdentityKeys)]
pub fn create_standard_identity_keys() -> Result<JsValue, JsError> {
    let keys = vec![
        // Master authentication key (id: 0)
        serde_json::json!({
            "id": 0,
            "type": "ECDSA_SECP256K1",
            "purpose": 0, // AUTHENTICATION
            "securityLevel": 0, // MASTER
            "readOnly": false,
            "data": null, // To be filled by user
        }),
        // High security authentication key (id: 1)
        serde_json::json!({
            "id": 1,
            "type": "ECDSA_SECP256K1",
            "purpose": 0, // AUTHENTICATION
            "securityLevel": 2, // HIGH
            "readOnly": false,
            "data": null, // To be filled by user
        }),
        // Transfer key (id: 2)
        serde_json::json!({
            "id": 2,
            "type": "ECDSA_SECP256K1",
            "purpose": 3, // TRANSFER
            "securityLevel": 1, // CRITICAL
            "readOnly": false,
            "data": null, // To be filled by user
        }),
    ];
    
    serde_wasm_bindgen::to_value(&keys)
        .map_err(|e| JsError::new(&format!("Failed to serialize keys: {}", e)))
}

/// Validate public keys for identity creation
#[wasm_bindgen(js_name = validateIdentityPublicKeys)]
pub fn validate_identity_public_keys(public_keys: JsValue) -> Result<JsValue, JsError> {
    let keys = if public_keys.is_array() {
        parse_public_keys_from_js(&public_keys)?
    } else {
        return Err(JsError::new("public_keys must be an array"));
    };
    
    if keys.is_empty() {
        return Err(JsError::new("At least one public key is required"));
    }
    
    // Check for at least one authentication key
    let has_auth_key = keys.iter().any(|key| {
        match key {
            IdentityPublicKey::V0(v0) => v0.purpose == Purpose::AUTHENTICATION,
        }
    });
    
    if !has_auth_key {
        return Err(JsError::new("At least one authentication key is required"));
    }
    
    // Check for duplicate key IDs
    let mut seen_ids = std::collections::HashSet::new();
    for key in &keys {
        let id = match key {
            IdentityPublicKey::V0(v0) => v0.id,
        };
        if !seen_ids.insert(id) {
            return Err(JsError::new(&format!("Duplicate key ID: {}", id)));
        }
    }
    
    // Check for at least one master key
    let has_master_key = keys.iter().any(|key| {
        match key {
            IdentityPublicKey::V0(v0) => v0.security_level == SecurityLevel::MASTER,
        }
    });
    
    if !has_master_key {
        return Err(JsError::new("At least one master security level key is required"));
    }
    
    let result = serde_json::json!({
        "valid": true,
        "keyCount": keys.len(),
        "hasAuthenticationKey": has_auth_key,
        "hasMasterKey": has_master_key,
    });
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsError::new(&format!("Failed to serialize result: {}", e)))
}