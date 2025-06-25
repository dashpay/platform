use crate::utils::getters::VecU8ToUint8Array;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionTypeWithResolvedRecipient;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionResolvedRecipient;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::tokens::emergency_action::TokenEmergencyAction;
use dpp::tokens::token_event::TokenEvent;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
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
            Reflect::set(
                &v0_obj,
                &JsValue::from_str("contract_id"),
                &contract_id_array,
            )
            .map_err(|_| JsValue::from_str("Failed to set contract_id"))?;

            // Set proposer_id
            let proposer_id_array = Uint8Array::from(v0.proposer_id.as_slice());
            Reflect::set(
                &v0_obj,
                &JsValue::from_str("proposer_id"),
                &proposer_id_array,
            )
            .map_err(|_| JsValue::from_str("Failed to set proposer_id"))?;

            // Set token_contract_position
            Reflect::set(
                &v0_obj,
                &JsValue::from_str("token_contract_position"),
                &JsValue::from_str(&v0.token_contract_position.to_string()),
            )
            .map_err(|_| JsValue::from_str("Failed to set token_contract_position"))?;

            // Serialize the event
            let event_js = group_action_event_to_js(&v0.event)?;
            Reflect::set(&v0_obj, &JsValue::from_str("event"), &event_js)
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
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
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

// Helper function to convert GroupActionEvent to JS object
fn group_action_event_to_js(event: &GroupActionEvent) -> Result<JsValue, JsValue> {
    match event {
        GroupActionEvent::TokenEvent(token_event) => {
            let event_obj = Object::new();

            let token_event_js = token_event_to_js(token_event)?;
            Reflect::set(
                &event_obj,
                &JsValue::from_str("TokenEvent"),
                &token_event_js,
            )
            .map_err(|_| JsValue::from_str("Failed to set TokenEvent"))?;

            Ok(event_obj.into())
        }
    }
}

// Helper function to convert TokenEvent to JS object
fn token_event_to_js(event: &TokenEvent) -> Result<JsValue, JsValue> {
    let obj = Object::new();

    match event {
        TokenEvent::Mint(amount, recipient, note) => {
            Reflect::set(&obj, &JsValue::from_str("type"), &JsValue::from_str("Mint"))
                .map_err(|_| JsValue::from_str("Failed to set type"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("amount"),
                &JsValue::from_str(&amount.to_string()),
            )
            .map_err(|_| JsValue::from_str("Failed to set amount"))?;

            let recipient_array = Uint8Array::from(recipient.as_slice());
            Reflect::set(&obj, &JsValue::from_str("recipient"), &recipient_array)
                .map_err(|_| JsValue::from_str("Failed to set recipient"))?;

            match note {
                Some(n) => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::from_str(n)),
                None => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::NULL),
            }
            .map_err(|_| JsValue::from_str("Failed to set note"))?;
        }
        TokenEvent::Burn(amount, burn_from, note) => {
            Reflect::set(&obj, &JsValue::from_str("type"), &JsValue::from_str("Burn"))
                .map_err(|_| JsValue::from_str("Failed to set type"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("amount"),
                &JsValue::from_str(&amount.to_string()),
            )
            .map_err(|_| JsValue::from_str("Failed to set amount"))?;

            let burn_from_array = Uint8Array::from(burn_from.as_slice());
            Reflect::set(&obj, &JsValue::from_str("burnFrom"), &burn_from_array)
                .map_err(|_| JsValue::from_str("Failed to set burnFrom"))?;

            match note {
                Some(n) => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::from_str(n)),
                None => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::NULL),
            }
            .map_err(|_| JsValue::from_str("Failed to set note"))?;
        }
        TokenEvent::Freeze(frozen_identity, note) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("Freeze"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            let frozen_array = Uint8Array::from(frozen_identity.as_slice());
            Reflect::set(&obj, &JsValue::from_str("frozenIdentity"), &frozen_array)
                .map_err(|_| JsValue::from_str("Failed to set frozenIdentity"))?;

            match note {
                Some(n) => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::from_str(n)),
                None => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::NULL),
            }
            .map_err(|_| JsValue::from_str("Failed to set note"))?;
        }
        TokenEvent::Unfreeze(frozen_identity, note) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("Unfreeze"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            let frozen_array = Uint8Array::from(frozen_identity.as_slice());
            Reflect::set(&obj, &JsValue::from_str("frozenIdentity"), &frozen_array)
                .map_err(|_| JsValue::from_str("Failed to set frozenIdentity"))?;

            match note {
                Some(n) => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::from_str(n)),
                None => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::NULL),
            }
            .map_err(|_| JsValue::from_str("Failed to set note"))?;
        }
        TokenEvent::DestroyFrozenFunds(frozen_identity, amount, note) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("DestroyFrozenFunds"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            let frozen_array = Uint8Array::from(frozen_identity.as_slice());
            Reflect::set(&obj, &JsValue::from_str("frozenIdentity"), &frozen_array)
                .map_err(|_| JsValue::from_str("Failed to set frozenIdentity"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("amount"),
                &JsValue::from_str(&amount.to_string()),
            )
            .map_err(|_| JsValue::from_str("Failed to set amount"))?;

            match note {
                Some(n) => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::from_str(n)),
                None => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::NULL),
            }
            .map_err(|_| JsValue::from_str("Failed to set note"))?;
        }
        TokenEvent::Transfer(
            recipient,
            public_note,
            shared_encrypted_note,
            personal_encrypted_note,
            amount,
        ) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("Transfer"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            let recipient_array = Uint8Array::from(recipient.as_slice());
            Reflect::set(&obj, &JsValue::from_str("recipient"), &recipient_array)
                .map_err(|_| JsValue::from_str("Failed to set recipient"))?;

            match public_note {
                Some(n) => Reflect::set(
                    &obj,
                    &JsValue::from_str("publicNote"),
                    &JsValue::from_str(n),
                ),
                None => Reflect::set(&obj, &JsValue::from_str("publicNote"), &JsValue::NULL),
            }
            .map_err(|_| JsValue::from_str("Failed to set publicNote"))?;

            // Serialize shared encrypted note (optional)
            match shared_encrypted_note {
                Some((sender_key_index, recipient_key_index, encrypted_data)) => {
                    let shared_note_obj = Object::new();
                    Reflect::set(
                        &shared_note_obj,
                        &JsValue::from_str("senderKeyIndex"),
                        &JsValue::from(*sender_key_index),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set senderKeyIndex"))?;
                    Reflect::set(
                        &shared_note_obj,
                        &JsValue::from_str("recipientKeyIndex"),
                        &JsValue::from(*recipient_key_index),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set recipientKeyIndex"))?;
                    let encrypted_array = Uint8Array::from(encrypted_data.as_slice());
                    Reflect::set(
                        &shared_note_obj,
                        &JsValue::from_str("encryptedData"),
                        &encrypted_array,
                    )
                    .map_err(|_| JsValue::from_str("Failed to set encryptedData"))?;
                    Reflect::set(
                        &obj,
                        &JsValue::from_str("sharedEncryptedNote"),
                        &shared_note_obj,
                    )
                    .map_err(|_| JsValue::from_str("Failed to set sharedEncryptedNote"))?;
                }
                None => {
                    Reflect::set(
                        &obj,
                        &JsValue::from_str("sharedEncryptedNote"),
                        &JsValue::NULL,
                    )
                    .map_err(|_| JsValue::from_str("Failed to set sharedEncryptedNote"))?;
                }
            }

            // Serialize personal encrypted note (optional)
            match personal_encrypted_note {
                Some((root_key_index, derivation_key_index, encrypted_data)) => {
                    let personal_note_obj = Object::new();
                    Reflect::set(
                        &personal_note_obj,
                        &JsValue::from_str("rootKeyIndex"),
                        &JsValue::from(*root_key_index),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set rootKeyIndex"))?;
                    Reflect::set(
                        &personal_note_obj,
                        &JsValue::from_str("derivationKeyIndex"),
                        &JsValue::from(*derivation_key_index),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set derivationKeyIndex"))?;
                    let encrypted_array = Uint8Array::from(encrypted_data.as_slice());
                    Reflect::set(
                        &personal_note_obj,
                        &JsValue::from_str("encryptedData"),
                        &encrypted_array,
                    )
                    .map_err(|_| JsValue::from_str("Failed to set encryptedData"))?;
                    Reflect::set(
                        &obj,
                        &JsValue::from_str("personalEncryptedNote"),
                        &personal_note_obj,
                    )
                    .map_err(|_| JsValue::from_str("Failed to set personalEncryptedNote"))?;
                }
                None => {
                    Reflect::set(
                        &obj,
                        &JsValue::from_str("personalEncryptedNote"),
                        &JsValue::NULL,
                    )
                    .map_err(|_| JsValue::from_str("Failed to set personalEncryptedNote"))?;
                }
            }

            Reflect::set(
                &obj,
                &JsValue::from_str("amount"),
                &JsValue::from_str(&amount.to_string()),
            )
            .map_err(|_| JsValue::from_str("Failed to set amount"))?;
        }
        TokenEvent::Claim(distribution_type, amount, note) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("Claim"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            // Serialize distribution type
            let dist_obj = Object::new();
            match distribution_type {
                TokenDistributionTypeWithResolvedRecipient::PreProgrammed(id) => {
                    Reflect::set(
                        &dist_obj,
                        &JsValue::from_str("type"),
                        &JsValue::from_str("PreProgrammed"),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set distribution type"))?;
                    let id_array = Uint8Array::from(id.as_slice());
                    Reflect::set(&dist_obj, &JsValue::from_str("id"), &id_array)
                        .map_err(|_| JsValue::from_str("Failed to set distribution id"))?;
                }
                TokenDistributionTypeWithResolvedRecipient::Perpetual(recipient) => {
                    Reflect::set(
                        &dist_obj,
                        &JsValue::from_str("type"),
                        &JsValue::from_str("Perpetual"),
                    )
                    .map_err(|_| JsValue::from_str("Failed to set distribution type"))?;

                    // Serialize resolved recipient
                    match recipient {
                        TokenDistributionResolvedRecipient::ContractOwnerIdentity(id) => {
                            Reflect::set(
                                &dist_obj,
                                &JsValue::from_str("recipientType"),
                                &JsValue::from_str("ContractOwnerIdentity"),
                            )
                            .map_err(|_| JsValue::from_str("Failed to set recipientType"))?;
                            let id_array = Uint8Array::from(id.as_slice());
                            Reflect::set(&dist_obj, &JsValue::from_str("recipientId"), &id_array)
                                .map_err(|_| JsValue::from_str("Failed to set recipientId"))?;
                        }
                        TokenDistributionResolvedRecipient::Identity(id) => {
                            Reflect::set(
                                &dist_obj,
                                &JsValue::from_str("recipientType"),
                                &JsValue::from_str("Identity"),
                            )
                            .map_err(|_| JsValue::from_str("Failed to set recipientType"))?;
                            let id_array = Uint8Array::from(id.as_slice());
                            Reflect::set(&dist_obj, &JsValue::from_str("recipientId"), &id_array)
                                .map_err(|_| JsValue::from_str("Failed to set recipientId"))?;
                        }
                        TokenDistributionResolvedRecipient::Evonode(id) => {
                            Reflect::set(
                                &dist_obj,
                                &JsValue::from_str("recipientType"),
                                &JsValue::from_str("Evonode"),
                            )
                            .map_err(|_| JsValue::from_str("Failed to set recipientType"))?;
                            let id_array = Uint8Array::from(id.as_slice());
                            Reflect::set(&dist_obj, &JsValue::from_str("recipientId"), &id_array)
                                .map_err(|_| JsValue::from_str("Failed to set recipientId"))?;
                        }
                    }
                }
            }
            Reflect::set(&obj, &JsValue::from_str("distributionType"), &dist_obj)
                .map_err(|_| JsValue::from_str("Failed to set distributionType"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("amount"),
                &JsValue::from_str(&amount.to_string()),
            )
            .map_err(|_| JsValue::from_str("Failed to set amount"))?;

            match note {
                Some(n) => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::from_str(n)),
                None => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::NULL),
            }
            .map_err(|_| JsValue::from_str("Failed to set note"))?;
        }
        TokenEvent::EmergencyAction(action, note) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("EmergencyAction"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            let action_str = match action {
                TokenEmergencyAction::Pause => "Pause",
                TokenEmergencyAction::Resume => "Resume",
            };
            Reflect::set(
                &obj,
                &JsValue::from_str("action"),
                &JsValue::from_str(action_str),
            )
            .map_err(|_| JsValue::from_str("Failed to set action"))?;

            match note {
                Some(n) => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::from_str(n)),
                None => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::NULL),
            }
            .map_err(|_| JsValue::from_str("Failed to set note"))?;
        }
        TokenEvent::ConfigUpdate(config_item, note) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("ConfigUpdate"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            // For now, we'll just serialize the config item as a string representation
            // In a real implementation, you might want to handle each variant separately
            Reflect::set(
                &obj,
                &JsValue::from_str("configItem"),
                &JsValue::from_str(&format!("{:?}", config_item)),
            )
            .map_err(|_| JsValue::from_str("Failed to set configItem"))?;

            match note {
                Some(n) => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::from_str(n)),
                None => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::NULL),
            }
            .map_err(|_| JsValue::from_str("Failed to set note"))?;
        }
        TokenEvent::ChangePriceForDirectPurchase(pricing_schedule, note) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("ChangePriceForDirectPurchase"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            match pricing_schedule {
                Some(schedule) => {
                    let schedule_obj = Object::new();
                    match schedule {
                        TokenPricingSchedule::SinglePrice(price) => {
                            Reflect::set(
                                &schedule_obj,
                                &JsValue::from_str("type"),
                                &JsValue::from_str("SinglePrice"),
                            )
                            .map_err(|_| JsValue::from_str("Failed to set schedule type"))?;
                            Reflect::set(
                                &schedule_obj,
                                &JsValue::from_str("price"),
                                &JsValue::from_str(&price.to_string()),
                            )
                            .map_err(|_| JsValue::from_str("Failed to set price"))?;
                        }
                        TokenPricingSchedule::SetPrices(prices) => {
                            Reflect::set(
                                &schedule_obj,
                                &JsValue::from_str("type"),
                                &JsValue::from_str("SetPrices"),
                            )
                            .map_err(|_| JsValue::from_str("Failed to set schedule type"))?;

                            let prices_obj = Object::new();
                            for (amount, price) in prices {
                                Reflect::set(
                                    &prices_obj,
                                    &JsValue::from_str(&amount.to_string()),
                                    &JsValue::from_str(&price.to_string()),
                                )
                                .map_err(|_| JsValue::from_str("Failed to set price entry"))?;
                            }
                            Reflect::set(&schedule_obj, &JsValue::from_str("prices"), &prices_obj)
                                .map_err(|_| JsValue::from_str("Failed to set prices"))?;
                        }
                    }
                    Reflect::set(&obj, &JsValue::from_str("pricingSchedule"), &schedule_obj)
                        .map_err(|_| JsValue::from_str("Failed to set pricingSchedule"))?;
                }
                None => {
                    Reflect::set(&obj, &JsValue::from_str("pricingSchedule"), &JsValue::NULL)
                        .map_err(|_| JsValue::from_str("Failed to set pricingSchedule"))?;
                }
            }

            match note {
                Some(n) => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::from_str(n)),
                None => Reflect::set(&obj, &JsValue::from_str("note"), &JsValue::NULL),
            }
            .map_err(|_| JsValue::from_str("Failed to set note"))?;
        }
        TokenEvent::DirectPurchase(amount, credits) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("DirectPurchase"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("amount"),
                &JsValue::from_str(&amount.to_string()),
            )
            .map_err(|_| JsValue::from_str("Failed to set amount"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("credits"),
                &JsValue::from_str(&credits.to_string()),
            )
            .map_err(|_| JsValue::from_str("Failed to set credits"))?;
        }
    }

    Ok(obj.into())
}
