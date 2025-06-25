use crate::utils::getters::VecU8ToUint8Array;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::verify::RootHash;
use js_sys::{Array, Object, Uint8Array};
use serde_wasm_bindgen::to_value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyGroupInfosInContractResult {
    root_hash: Vec<u8>,
    groups: JsValue,
}

#[wasm_bindgen]
impl VerifyGroupInfosInContractResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn groups(&self) -> JsValue {
        self.groups.clone()
    }
}

/// Verify group infos in contract and return as an array of [position, group] pairs
#[wasm_bindgen(js_name = "verifyGroupInfosInContractVec")]
pub fn verify_group_infos_in_contract_vec(
    proof: &Uint8Array,
    contract_id: &Uint8Array,
    start_group_contract_position: Option<u16>,
    start_at_included: Option<bool>,
    limit: Option<u16>,
    is_proof_subset: bool,
    platform_version_number: u32,
) -> Result<VerifyGroupInfosInContractResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    let start_position = match (start_group_contract_position, start_at_included) {
        (Some(pos), Some(included)) => Some((pos, included)),
        (Some(_), None) => {
            return Err(JsValue::from_str(
                "start_at_included must be provided when start_group_contract_position is set",
            ))
        }
        (None, Some(_)) => {
            return Err(JsValue::from_str(
                "start_group_contract_position must be provided when start_at_included is set",
            ))
        }
        (None, None) => None,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, groups_vec): (RootHash, Vec<(GroupContractPosition, Group)>) =
        Drive::verify_group_infos_in_contract(
            &proof_vec,
            Identifier::from(contract_id_bytes),
            start_position,
            limit,
            is_proof_subset,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert Vec<(GroupContractPosition, Group)> to JavaScript array
    let js_array = Array::new();
    for (position, group) in groups_vec {
        let pair_array = Array::new();
        pair_array.push(&JsValue::from(position));

        let group_json = serde_json::to_value(&group)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize group: {:?}", e)))?;
        let group_js = to_value(&group_json).map_err(|e| {
            JsValue::from_str(&format!("Failed to convert group to JsValue: {:?}", e))
        })?;
        pair_array.push(&group_js);

        js_array.push(&pair_array);
    }

    Ok(VerifyGroupInfosInContractResult {
        root_hash: root_hash.to_vec(),
        groups: js_array.into(),
    })
}

/// Verify group infos in contract and return as a map with position as key
#[wasm_bindgen(js_name = "verifyGroupInfosInContractMap")]
pub fn verify_group_infos_in_contract_map(
    proof: &Uint8Array,
    contract_id: &Uint8Array,
    start_group_contract_position: Option<u16>,
    start_at_included: Option<bool>,
    limit: Option<u16>,
    is_proof_subset: bool,
    platform_version_number: u32,
) -> Result<VerifyGroupInfosInContractResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    let start_position = match (start_group_contract_position, start_at_included) {
        (Some(pos), Some(included)) => Some((pos, included)),
        (Some(_), None) => {
            return Err(JsValue::from_str(
                "start_at_included must be provided when start_group_contract_position is set",
            ))
        }
        (None, Some(_)) => {
            return Err(JsValue::from_str(
                "start_group_contract_position must be provided when start_at_included is set",
            ))
        }
        (None, None) => None,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, groups_map): (RootHash, BTreeMap<GroupContractPosition, Group>) =
        Drive::verify_group_infos_in_contract(
            &proof_vec,
            Identifier::from(contract_id_bytes),
            start_position,
            limit,
            is_proof_subset,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert BTreeMap<GroupContractPosition, Group> to JavaScript object
    let js_object = Object::new();
    for (position, group) in groups_map {
        let group_json = serde_json::to_value(&group)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize group: {:?}", e)))?;
        let group_js = to_value(&group_json).map_err(|e| {
            JsValue::from_str(&format!("Failed to convert group to JsValue: {:?}", e))
        })?;

        js_sys::Reflect::set(&js_object, &JsValue::from(position.to_string()), &group_js)
            .map_err(|_| JsValue::from_str("Failed to set object property"))?;
    }

    Ok(VerifyGroupInfosInContractResult {
        root_hash: root_hash.to_vec(),
        groups: js_object.into(),
    })
}
