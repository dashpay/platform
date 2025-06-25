use crate::utils::getters::VecU8ToUint8Array;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyActionSignersTotalPowerResult {
    root_hash: Vec<u8>,
    action_status: u8,
    total_power: u64,
}

#[wasm_bindgen]
impl VerifyActionSignersTotalPowerResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn action_status(&self) -> u8 {
        self.action_status
    }

    #[wasm_bindgen(getter)]
    pub fn total_power(&self) -> u64 {
        self.total_power
    }
}

#[wasm_bindgen(js_name = "verifyActionSignersTotalPower")]
pub fn verify_action_signers_total_power(
    proof: &Uint8Array,
    contract_id: &Uint8Array,
    group_contract_position: u16,
    action_status: Option<u8>,
    action_id: &Uint8Array,
    action_signer_id: &Uint8Array,
    is_proof_subset: bool,
    platform_version_number: u32,
) -> Result<VerifyActionSignersTotalPowerResult, JsValue> {
    let proof_vec = proof.to_vec();

    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    let action_id_bytes: [u8; 32] = action_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid action_id length. Expected 32 bytes."))?;

    let action_signer_id_bytes: [u8; 32] = action_signer_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid action_signer_id length. Expected 32 bytes."))?;

    // Convert action_status from u8 to GroupActionStatus
    let action_status_enum = action_status
        .map(|status| match status {
            0 => Ok(GroupActionStatus::ActionActive),
            1 => Ok(GroupActionStatus::ActionClosed),
            _ => Err(JsValue::from_str("Invalid action status value")),
        })
        .transpose()?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, status, total_power) = Drive::verify_action_signer_and_total_power(
        &proof_vec,
        Identifier::from(contract_id_bytes),
        group_contract_position,
        action_status_enum,
        Identifier::from(action_id_bytes),
        Identifier::from(action_signer_id_bytes),
        is_proof_subset,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert GroupActionStatus back to u8
    let status_u8 = match status {
        GroupActionStatus::ActionActive => 0,
        GroupActionStatus::ActionClosed => 1,
    };

    Ok(VerifyActionSignersTotalPowerResult {
        root_hash: root_hash.to_vec(),
        action_status: status_u8,
        total_power: total_power as u64,
    })
}
