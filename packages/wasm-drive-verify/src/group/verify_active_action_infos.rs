use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::verify::RootHash;
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

// Helper function to convert GroupAction to JS object
fn group_action_to_js(action: &GroupAction) -> Result<JsValue, JsValue> {
    match action {
        GroupAction::V0(v0) => {
            let v0_obj = Object::new();
            
            // Set contract_id
            let contract_id_array = Uint8Array::from(v0.contract_id.as_slice());
            Reflect::set(&v0_obj, &JsValue::from_str("contract_id"), &contract_id_array)
                .map_err(|_| JsValue::from_str("Failed to set contract_id"))?;
            
            // Set proposer_id
            let proposer_id_array = Uint8Array::from(v0.proposer_id.as_slice());
            Reflect::set(&v0_obj, &JsValue::from_str("proposer_id"), &proposer_id_array)
                .map_err(|_| JsValue::from_str("Failed to set proposer_id"))?;
            
            // Set token_contract_position
            Reflect::set(&v0_obj, &JsValue::from_str("token_contract_position"), &JsValue::from_f64(v0.token_contract_position as f64))
                .map_err(|_| JsValue::from_str("Failed to set token_contract_position"))?;
            
            // For now, we'll set a placeholder for the event
            // TODO: Implement full event serialization
            Reflect::set(&v0_obj, &JsValue::from_str("event"), &JsValue::from_str("[Event serialization not yet implemented]"))
                .map_err(|_| JsValue::from_str("Failed to set event"))?;
            
            let action_obj = Object::new();
            Reflect::set(&action_obj, &JsValue::from_str("V0"), &v0_obj)
                .map_err(|_| JsValue::from_str("Failed to set V0"))?;
            
            Ok(action_obj.into())
        }
    }
}

#[wasm_bindgen]
pub struct VerifyActionInfosInContractResult {
    root_hash: Vec<u8>,
    actions: JsValue,
}

#[wasm_bindgen]
impl VerifyActionInfosInContractResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn actions(&self) -> JsValue {
        self.actions.clone()
    }
}

/// Verify action infos in contract and return as an array of [action_id, action] pairs
#[wasm_bindgen(js_name = "verifyActionInfosInContractVec")]
pub fn verify_action_infos_in_contract_vec(
    proof: &Uint8Array,
    contract_id: &Uint8Array,
    group_contract_position: u16,
    action_status: u8,
    start_action_id: Option<Uint8Array>,
    start_at_included: Option<bool>,
    limit: Option<u16>,
    is_proof_subset: bool,
    platform_version_number: u32,
) -> Result<VerifyActionInfosInContractResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    // Convert action_status from u8 to GroupActionStatus
    let action_status_enum = match action_status {
        0 => GroupActionStatus::ActionActive,
        1 => GroupActionStatus::ActionClosed,
        _ => return Err(JsValue::from_str("Invalid action status value")),
    };

    let start_position = match (start_action_id, start_at_included) {
        (Some(id), Some(included)) => {
            let id_bytes: [u8; 32] = id.to_vec().try_into().map_err(|_| {
                JsValue::from_str("Invalid start_action_id length. Expected 32 bytes.")
            })?;
            Some((Identifier::from(id_bytes), included))
        }
        (Some(_), None) => {
            return Err(JsValue::from_str(
                "start_at_included must be provided when start_action_id is set",
            ))
        }
        (None, Some(_)) => {
            return Err(JsValue::from_str(
                "start_action_id must be provided when start_at_included is set",
            ))
        }
        (None, None) => None,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, actions_vec): (RootHash, Vec<(Identifier, GroupAction)>) =
        Drive::verify_action_infos_in_contract(
            &proof_vec,
            Identifier::from(contract_id_bytes),
            group_contract_position,
            action_status_enum,
            start_position,
            limit,
            is_proof_subset,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert Vec<(Identifier, GroupAction)> to JavaScript array
    let js_array = Array::new();
    for (id, action) in actions_vec {
        let pair_array = Array::new();
        let id_bytes = id.as_bytes();
        pair_array.push(&Uint8Array::from(&id_bytes[..]).into());

        let action_js = group_action_to_js(&action)?;
        pair_array.push(&action_js);

        js_array.push(&pair_array);
    }

    Ok(VerifyActionInfosInContractResult {
        root_hash: root_hash.to_vec(),
        actions: js_array.into(),
    })
}

/// Verify action infos in contract and return as a map with action_id as key
#[wasm_bindgen(js_name = "verifyActionInfosInContractMap")]
pub fn verify_action_infos_in_contract_map(
    proof: &Uint8Array,
    contract_id: &Uint8Array,
    group_contract_position: u16,
    action_status: u8,
    start_action_id: Option<Uint8Array>,
    start_at_included: Option<bool>,
    limit: Option<u16>,
    is_proof_subset: bool,
    platform_version_number: u32,
) -> Result<VerifyActionInfosInContractResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    // Convert action_status from u8 to GroupActionStatus
    let action_status_enum = match action_status {
        0 => GroupActionStatus::ActionActive,
        1 => GroupActionStatus::ActionClosed,
        _ => return Err(JsValue::from_str("Invalid action status value")),
    };

    let start_position = match (start_action_id, start_at_included) {
        (Some(id), Some(included)) => {
            let id_bytes: [u8; 32] = id.to_vec().try_into().map_err(|_| {
                JsValue::from_str("Invalid start_action_id length. Expected 32 bytes.")
            })?;
            Some((Identifier::from(id_bytes), included))
        }
        (Some(_), None) => {
            return Err(JsValue::from_str(
                "start_at_included must be provided when start_action_id is set",
            ))
        }
        (None, Some(_)) => {
            return Err(JsValue::from_str(
                "start_action_id must be provided when start_at_included is set",
            ))
        }
        (None, None) => None,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, actions_map): (RootHash, BTreeMap<Identifier, GroupAction>) =
        Drive::verify_action_infos_in_contract(
            &proof_vec,
            Identifier::from(contract_id_bytes),
            group_contract_position,
            action_status_enum,
            start_position,
            limit,
            is_proof_subset,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert BTreeMap<Identifier, GroupAction> to JavaScript object
    let js_object = Object::new();
    for (id, action) in actions_map {
        let action_js = group_action_to_js(&action)?;

        // Use base64 encoded identifier as key
        use base64::{engine::general_purpose, Engine as _};
        let id_base64 = general_purpose::STANDARD.encode(id.as_bytes());
        js_sys::Reflect::set(&js_object, &JsValue::from_str(&id_base64), &action_js)
            .map_err(|_| JsValue::from_str("Failed to set object property"))?;
    }

    Ok(VerifyActionInfosInContractResult {
        root_hash: root_hash.to_vec(),
        actions: js_object.into(),
    })
}
