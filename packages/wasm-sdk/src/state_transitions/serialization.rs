//! State Transition Serialization Interface
//!
//! This module provides WASM bindings for serializing and deserializing state transitions.
//! It acts as a bridge between JavaScript and the native DPP state transition types.

use dpp::serialization::Signable;
use dpp::state_transition::StateTransition;
use platform_version::version::PlatformVersion;
use wasm_bindgen::prelude::*;
use web_sys::js_sys::{Object, Reflect, Uint8Array};

// Import accessor traits
use dpp::identity::state_transition::AssetLockProved;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;

/// State transition type enum for JavaScript
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub enum StateTransitionTypeWasm {
    DataContractCreate = 0,
    Batch = 1,
    IdentityCreate = 2,
    IdentityTopUp = 3,
    DataContractUpdate = 4,
    IdentityUpdate = 5,
    IdentityCreditWithdrawal = 6,
    IdentityCreditTransfer = 7,
    MasternodeVote = 8,
}

/// Serialize any state transition to bytes
#[wasm_bindgen(js_name = serializeStateTransition)]
pub fn serialize_state_transition(state_transition_bytes: &Uint8Array) -> Result<Vec<u8>, JsError> {
    // The input is already a serialized state transition from one of our creation methods
    // We just need to return it as-is for now
    Ok(state_transition_bytes.to_vec())
}

/// Deserialize state transition from bytes
#[wasm_bindgen(js_name = deserializeStateTransition)]
pub fn deserialize_state_transition(bytes: &Uint8Array) -> Result<JsValue, JsError> {
    let bytes = bytes.to_vec();
    let platform_version = PlatformVersion::latest();

    let state_transition =
        StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
            .map_err(|e| JsError::new(&format!("Failed to deserialize state transition: {}", e)))?;

    // Convert to JavaScript object
    state_transition_to_js_object(&state_transition)
}

/// Get the type of a serialized state transition
#[wasm_bindgen(js_name = getStateTransitionType)]
pub fn get_state_transition_type(bytes: &Uint8Array) -> Result<StateTransitionTypeWasm, JsError> {
    let bytes = bytes.to_vec();
    let platform_version = PlatformVersion::latest();

    let state_transition =
        StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
            .map_err(|e| JsError::new(&format!("Failed to deserialize state transition: {}", e)))?;

    Ok(match state_transition {
        StateTransition::DataContractCreate(_) => StateTransitionTypeWasm::DataContractCreate,
        StateTransition::DataContractUpdate(_) => StateTransitionTypeWasm::DataContractUpdate,
        StateTransition::Batch(_) => StateTransitionTypeWasm::Batch,
        StateTransition::IdentityCreate(_) => StateTransitionTypeWasm::IdentityCreate,
        StateTransition::IdentityTopUp(_) => StateTransitionTypeWasm::IdentityTopUp,
        StateTransition::IdentityCreditWithdrawal(_) => {
            StateTransitionTypeWasm::IdentityCreditWithdrawal
        }
        StateTransition::IdentityUpdate(_) => StateTransitionTypeWasm::IdentityUpdate,
        StateTransition::IdentityCreditTransfer(_) => {
            StateTransitionTypeWasm::IdentityCreditTransfer
        }
        StateTransition::MasternodeVote(_) => StateTransitionTypeWasm::MasternodeVote,
    })
}

/// Calculate the hash of a state transition
#[wasm_bindgen(js_name = calculateStateTransitionId)]
pub fn calculate_state_transition_id(bytes: &Uint8Array) -> Result<String, JsError> {
    use sha2::{Digest, Sha256};

    let bytes = bytes.to_vec();
    let platform_version = PlatformVersion::latest();

    // Validate that it's a proper state transition
    let _state_transition =
        StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
            .map_err(|e| JsError::new(&format!("Failed to deserialize state transition: {}", e)))?;

    // Calculate SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();

    Ok(hex::encode(result))
}

/// Validate a state transition (basic validation without state)
#[wasm_bindgen(js_name = validateStateTransitionStructure)]
pub fn validate_state_transition_structure(bytes: &Uint8Array) -> Result<JsValue, JsError> {
    let bytes = bytes.to_vec();
    let platform_version = PlatformVersion::latest();

    // Try to deserialize - this performs basic structure validation
    let state_transition =
        StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
            .map_err(|e| JsError::new(&format!("Invalid state transition structure: {}", e)))?;

    let result = Object::new();
    Reflect::set(&result, &"valid".into(), &true.into())
        .map_err(|_| JsError::new("Failed to set valid"))?;
    Reflect::set(&result, &"type".into(), &state_transition.name().into())
        .map_err(|_| JsError::new("Failed to set type"))?;

    Ok(result.into())
}

/// Check if a state transition requires an identity signature
#[wasm_bindgen(js_name = isIdentitySignedStateTransition)]
pub fn is_identity_signed_state_transition(bytes: &Uint8Array) -> Result<bool, JsError> {
    let bytes = bytes.to_vec();
    let platform_version = PlatformVersion::latest();

    let state_transition =
        StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
            .map_err(|e| JsError::new(&format!("Failed to deserialize state transition: {}", e)))?;

    Ok(state_transition.is_identity_signed())
}

/// Get the identity ID associated with a state transition (if applicable)
#[wasm_bindgen(js_name = getStateTransitionIdentityId)]
pub fn get_state_transition_identity_id(bytes: &Uint8Array) -> Result<Option<String>, JsError> {
    let bytes = bytes.to_vec();
    let platform_version = PlatformVersion::latest();

    let state_transition =
        StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
            .map_err(|e| JsError::new(&format!("Failed to deserialize state transition: {}", e)))?;

    // Get identity ID based on transition type
    use dpp::prelude::Identifier;
    let identity_id: Option<Identifier> = match &state_transition {
        StateTransition::IdentityCreate(st) => Some(st.identity_id()),
        StateTransition::IdentityTopUp(st) => Some(*st.identity_id()),
        StateTransition::IdentityUpdate(st) => Some(st.identity_id()),
        StateTransition::IdentityCreditWithdrawal(st) => Some(st.identity_id()),
        StateTransition::IdentityCreditTransfer(st) => Some(st.identity_id()),
        _ => None,
    };

    Ok(identity_id.map(|id| id.to_string(platform_value::string_encoding::Encoding::Base58)))
}

/// Get modified data IDs from a state transition
#[wasm_bindgen(js_name = getModifiedDataIds)]
pub fn get_modified_data_ids(bytes: &Uint8Array) -> Result<JsValue, JsError> {
    let bytes = bytes.to_vec();
    let platform_version = PlatformVersion::latest();

    let state_transition =
        StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
            .map_err(|e| JsError::new(&format!("Failed to deserialize state transition: {}", e)))?;

    let result = Object::new();

    match &state_transition {
        StateTransition::DataContractCreate(st) => {
            let contract_id = st
                .data_contract()
                .id()
                .to_string(platform_value::string_encoding::Encoding::Base58);
            Reflect::set(&result, &"dataContractId".into(), &contract_id.into())
                .map_err(|_| JsError::new("Failed to set data contract ID"))?;
        }
        StateTransition::DataContractUpdate(st) => {
            let contract_id = st
                .data_contract()
                .id()
                .to_string(platform_value::string_encoding::Encoding::Base58);
            Reflect::set(&result, &"dataContractId".into(), &contract_id.into())
                .map_err(|_| JsError::new("Failed to set data contract ID"))?;
        }
        StateTransition::IdentityCreate(st) => {
            let identity_id = st
                .identity_id()
                .to_string(platform_value::string_encoding::Encoding::Base58);
            Reflect::set(&result, &"identityId".into(), &identity_id.into())
                .map_err(|_| JsError::new("Failed to set identity ID"))?;
        }
        StateTransition::IdentityTopUp(st) => {
            let identity_id = st
                .identity_id()
                .to_string(platform_value::string_encoding::Encoding::Base58);
            Reflect::set(&result, &"identityId".into(), &identity_id.into())
                .map_err(|_| JsError::new("Failed to set identity ID"))?;
        }
        StateTransition::IdentityUpdate(st) => {
            let identity_id = st
                .identity_id()
                .to_string(platform_value::string_encoding::Encoding::Base58);
            Reflect::set(&result, &"identityId".into(), &identity_id.into())
                .map_err(|_| JsError::new("Failed to set identity ID"))?;
        }
        _ => {
            // Other types have more complex data IDs
        }
    }

    Ok(result.into())
}

/// Convert a state transition to a JavaScript object representation
fn state_transition_to_js_object(state_transition: &StateTransition) -> Result<JsValue, JsError> {
    let obj = Object::new();

    // Add common fields
    Reflect::set(&obj, &"type".into(), &state_transition.name().into())
        .map_err(|_| JsError::new("Failed to set type"))?;

    // Add type-specific fields
    match state_transition {
        StateTransition::IdentityCreate(st) => {
            let identity_id = st
                .identity_id()
                .to_string(platform_value::string_encoding::Encoding::Base58);
            Reflect::set(&obj, &"identityId".into(), &identity_id.into())
                .map_err(|_| JsError::new("Failed to set identity ID"))?;

            // Add public keys count
            Reflect::set(
                &obj,
                &"publicKeysCount".into(),
                &(st.public_keys().len() as u32).into(),
            )
            .map_err(|_| JsError::new("Failed to set public keys count"))?;

            // Add asset lock proof type
            let proof = st.asset_lock_proof();
            let proof_type = match proof {
                dpp::prelude::AssetLockProof::Instant(_) => "instant",
                dpp::prelude::AssetLockProof::Chain(_) => "chain",
            };
            Reflect::set(&obj, &"assetLockProofType".into(), &proof_type.into())
                .map_err(|_| JsError::new("Failed to set asset lock proof type"))?;
        }
        StateTransition::IdentityTopUp(st) => {
            let identity_id = st
                .identity_id()
                .to_string(platform_value::string_encoding::Encoding::Base58);
            Reflect::set(&obj, &"identityId".into(), &identity_id.into())
                .map_err(|_| JsError::new("Failed to set identity ID"))?;

            // IdentityTopUp also has an asset lock proof
            let proof = st.asset_lock_proof();
            let proof_type = match proof {
                dpp::prelude::AssetLockProof::Instant(_) => "instant",
                dpp::prelude::AssetLockProof::Chain(_) => "chain",
            };
            Reflect::set(&obj, &"assetLockProofType".into(), &proof_type.into())
                .map_err(|_| JsError::new("Failed to set asset lock proof type"))?;
        }
        StateTransition::DataContractCreate(st) => {
            let contract_id = st
                .data_contract()
                .id()
                .to_string(platform_value::string_encoding::Encoding::Base58);
            Reflect::set(&obj, &"dataContractId".into(), &contract_id.into())
                .map_err(|_| JsError::new("Failed to set data contract ID"))?;
        }
        StateTransition::DataContractUpdate(st) => {
            let contract_id = st
                .data_contract()
                .id()
                .to_string(platform_value::string_encoding::Encoding::Base58);
            Reflect::set(&obj, &"dataContractId".into(), &contract_id.into())
                .map_err(|_| JsError::new("Failed to set data contract ID"))?;
        }
        _ => {
            // Add more fields as needed for other types
        }
    }

    Ok(obj.into())
}

/// Extract signable bytes from a state transition (for signing)
#[wasm_bindgen(js_name = getStateTransitionSignableBytes)]
pub fn get_state_transition_signable_bytes(bytes: &Uint8Array) -> Result<Uint8Array, JsError> {
    let bytes = bytes.to_vec();
    let platform_version = PlatformVersion::latest();

    let state_transition =
        StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
            .map_err(|e| JsError::new(&format!("Failed to deserialize state transition: {}", e)))?;

    let signable_bytes = state_transition
        .signable_bytes()
        .map_err(|e| JsError::new(&format!("Failed to get signable bytes: {}", e)))?;

    Ok(Uint8Array::from(&signable_bytes[..]))
}
