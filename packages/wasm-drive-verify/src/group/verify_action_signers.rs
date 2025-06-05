use dpp::data_contract::group::GroupMemberPower;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::verify::RootHash;
use js_sys::{Array, Object, Uint8Array};
use serde_wasm_bindgen::to_value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyActionSignersResult {
    root_hash: Vec<u8>,
    signers: JsValue,
}

#[wasm_bindgen]
impl VerifyActionSignersResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn signers(&self) -> JsValue {
        self.signers.clone()
    }
}

/// Verify action signers and return as an array of [signer_id, power] pairs
#[wasm_bindgen(js_name = "verifyActionSignersVec")]
pub fn verify_action_signers_vec(
    proof: &Uint8Array,
    contract_id: &Uint8Array,
    group_contract_position: u16,
    action_status: u8,
    action_id: &Uint8Array,
    is_proof_subset: bool,
    platform_version_number: u32,
) -> Result<VerifyActionSignersResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    let action_id_bytes: [u8; 32] = action_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid action_id length. Expected 32 bytes."))?;

    // Convert action_status from u8 to GroupActionStatus
    let action_status_enum = match action_status {
        0 => GroupActionStatus::Active,
        1 => GroupActionStatus::Closed,
        _ => return Err(JsValue::from_str("Invalid action status value")),
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, signers_vec): (RootHash, Vec<(Identifier, GroupMemberPower)>) =
        drive::verify::group::verify_action_signers(
            &proof_vec,
            Identifier::from(contract_id_bytes),
            group_contract_position,
            action_status_enum,
            Identifier::from(action_id_bytes),
            is_proof_subset,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert Vec<(Identifier, GroupMemberPower)> to JavaScript array
    let js_array = Array::new();
    for (signer_id, power) in signers_vec {
        let pair_array = Array::new();
        pair_array.push(&Uint8Array::from(signer_id.as_bytes()).into());
        pair_array.push(&JsValue::from(power));
        js_array.push(&pair_array);
    }

    Ok(VerifyActionSignersResult {
        root_hash: root_hash.to_vec(),
        signers: js_array.into(),
    })
}

/// Verify action signers and return as a map with signer_id as key
#[wasm_bindgen(js_name = "verifyActionSignersMap")]
pub fn verify_action_signers_map(
    proof: &Uint8Array,
    contract_id: &Uint8Array,
    group_contract_position: u16,
    action_status: u8,
    action_id: &Uint8Array,
    is_proof_subset: bool,
    platform_version_number: u32,
) -> Result<VerifyActionSignersResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    let action_id_bytes: [u8; 32] = action_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid action_id length. Expected 32 bytes."))?;

    // Convert action_status from u8 to GroupActionStatus
    let action_status_enum = match action_status {
        0 => GroupActionStatus::Active,
        1 => GroupActionStatus::Closed,
        _ => return Err(JsValue::from_str("Invalid action status value")),
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, signers_map): (RootHash, BTreeMap<Identifier, GroupMemberPower>) =
        drive::verify::group::verify_action_signers(
            &proof_vec,
            Identifier::from(contract_id_bytes),
            group_contract_position,
            action_status_enum,
            Identifier::from(action_id_bytes),
            is_proof_subset,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert BTreeMap<Identifier, GroupMemberPower> to JavaScript object
    let js_object = Object::new();
    for (signer_id, power) in signers_map {
        // Use base64 encoded identifier as key
        let id_base64 = base64::encode(signer_id.as_bytes());
        js_sys::Reflect::set(
            &js_object,
            &JsValue::from_str(&id_base64),
            &JsValue::from(power),
        )
        .map_err(|_| JsValue::from_str("Failed to set object property"))?;
    }

    Ok(VerifyActionSignersResult {
        root_hash: root_hash.to_vec(),
        signers: js_object.into(),
    })
}
