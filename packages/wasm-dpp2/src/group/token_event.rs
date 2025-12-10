use crate::enums::token::emergency_action::TokenEmergencyActionWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::state_transitions::batch::token_pricing_schedule::TokenPricingScheduleWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use crate::tokens::encrypted_note::private_encrypted_note::PrivateEncryptedNoteWasm;
use crate::tokens::encrypted_note::shared_encrypted_note::SharedEncryptedNoteWasm;
use crate::utils::JsValueExt;
use dpp::tokens::token_event::TokenEvent;
use js_sys::{BigInt, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "TokenEvent")]
pub struct TokenEventWasm(TokenEvent);

impl From<TokenEvent> for TokenEventWasm {
    fn from(event: TokenEvent) -> Self {
        TokenEventWasm(event)
    }
}

impl From<TokenEventWasm> for TokenEvent {
    fn from(event: TokenEventWasm) -> Self {
        event.0
    }
}

#[wasm_bindgen(js_class = TokenEvent)]
impl TokenEventWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenEvent".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name(&self) -> String {
        "TokenEvent".to_string()
    }

    #[wasm_bindgen(getter = "variant")]
    pub fn variant(&self) -> String {
        match &self.0 {
            TokenEvent::Mint(..) => "Mint",
            TokenEvent::Burn(..) => "Burn",
            TokenEvent::Freeze(..) => "Freeze",
            TokenEvent::Unfreeze(..) => "Unfreeze",
            TokenEvent::DestroyFrozenFunds(..) => "DestroyFrozenFunds",
            TokenEvent::Transfer(..) => "Transfer",
            TokenEvent::Claim(..) => "Claim",
            TokenEvent::EmergencyAction(..) => "EmergencyAction",
            TokenEvent::ConfigUpdate(..) => "ConfigUpdate",
            TokenEvent::ChangePriceForDirectPurchase(..) => "ChangePriceForDirectPurchase",
            TokenEvent::DirectPurchase(..) => "DirectPurchase",
        }
        .to_string()
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let object = Object::new();
        set_property(&object, "variant", &JsValue::from_str(&self.variant()))?;

        match &self.0 {
            TokenEvent::Mint(amount, recipient, note) => {
                set_amount_json(&object, *amount)?;
                set_property(
                    &object,
                    "recipient",
                    &IdentifierWasm::from(*recipient).to_base58().into(),
                )?;
                set_note(&object, note)?;
            }
            TokenEvent::Burn(amount, burn_from, note) => {
                set_amount_json(&object, *amount)?;
                set_property(
                    &object,
                    "burnFrom",
                    &IdentifierWasm::from(*burn_from).to_base58().into(),
                )?;
                set_note(&object, note)?;
            }
            TokenEvent::Freeze(identity, note) | TokenEvent::Unfreeze(identity, note) => {
                set_property(
                    &object,
                    "identity",
                    &IdentifierWasm::from(*identity).to_base58().into(),
                )?;
                set_note(&object, note)?;
            }
            TokenEvent::DestroyFrozenFunds(identity, amount, note) => {
                set_property(
                    &object,
                    "identity",
                    &IdentifierWasm::from(*identity).to_base58().into(),
                )?;
                set_amount_json(&object, *amount)?;
                set_note(&object, note)?;
            }
            TokenEvent::Transfer(recipient, note, shared, private, amount) => {
                set_property(
                    &object,
                    "recipient",
                    &IdentifierWasm::from(*recipient).to_base58().into(),
                )?;
                set_note(&object, note)?;

                match shared {
                    Some(value) => {
                        let shared_obj = Object::new();
                        set_property(&shared_obj, "senderKeyIndex", &JsValue::from(value.0))?;
                        set_property(&shared_obj, "recipientKeyIndex", &JsValue::from(value.1))?;
                        set_property(
                            &shared_obj,
                            "value",
                            &hex::encode(&value.2).into(),
                        )?;
                        set_property(&object, "sharedEncryptedNote", &shared_obj.into())?
                    }
                    None => set_property(&object, "sharedEncryptedNote", &JsValue::NULL)?,
                }

                match private {
                    Some(value) => {
                        let private_obj = Object::new();
                        set_property(&private_obj, "rootEncryptionKeyIndex", &JsValue::from(value.0))?;
                        set_property(&private_obj, "derivationEncryptionKeyIndex", &JsValue::from(value.1))?;
                        set_property(
                            &private_obj,
                            "value",
                            &hex::encode(&value.2).into(),
                        )?;
                        set_property(&object, "privateEncryptedNote", &private_obj.into())?
                    }
                    None => set_property(&object, "privateEncryptedNote", &JsValue::NULL)?,
                }

                set_amount_json(&object, *amount)?;
            }
            TokenEvent::Claim(distribution, amount, note) => {
                let distribution_js =
                    serde_wasm_bindgen::to_value(distribution).map_err(|err| {
                        WasmDppError::serialization(format!(
                            "unable to serialize token distribution recipient: {}",
                            err
                        ))
                    })?;
                set_property(&object, "distribution", &distribution_js)?;
                set_amount_json(&object, *amount)?;
                set_note(&object, note)?;
            }
            TokenEvent::EmergencyAction(action, note) => {
                set_property(
                    &object,
                    "action",
                    &JsValue::from(String::from(TokenEmergencyActionWasm::from(*action))),
                )?;
                set_note(&object, note)?;
            }
            TokenEvent::ConfigUpdate(change, note) => {
                let change_wasm = TokenConfigurationChangeItemWasm::from(change.clone());
                let change_obj = Object::new();
                set_property(&change_obj, "itemName", &change_wasm.get_item_name().into())?;
                set_property(&object, "change", &change_obj.into())?;
                set_note(&object, note)?;
            }
            TokenEvent::ChangePriceForDirectPurchase(schedule, note) => {
                match schedule {
                    Some(s) => {
                        let schedule_wasm = TokenPricingScheduleWasm::from(s.clone());
                        let schedule_obj = Object::new();
                        set_property(
                            &schedule_obj,
                            "type",
                            &schedule_wasm.get_scheduled_type().into(),
                        )?;
                        if let Ok(val) = schedule_wasm.get_value() {
                            set_property(&schedule_obj, "value", &val)?;
                        }
                        set_property(&object, "pricingSchedule", &schedule_obj.into())?
                    }
                    None => set_property(&object, "pricingSchedule", &JsValue::NULL)?,
                }
                set_note(&object, note)?;
            }
            TokenEvent::DirectPurchase(amount, credits) => {
                set_amount_json(&object, *amount)?;
                set_property(&object, "credits", &JsValue::from(*credits as f64))?;
            }
        }

        Ok(object.into())
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let object = Object::new();
        set_property(&object, "variant", &JsValue::from_str(&self.variant()))?;

        match &self.0 {
            TokenEvent::Mint(amount, recipient, note) => {
                set_amount(&object, *amount)?;
                set_property(
                    &object,
                    "recipient",
                    &JsValue::from(IdentifierWasm::from(*recipient)),
                )?;
                set_note(&object, note)?;
            }
            TokenEvent::Burn(amount, burn_from, note) => {
                set_amount(&object, *amount)?;
                set_property(
                    &object,
                    "burnFrom",
                    &JsValue::from(IdentifierWasm::from(*burn_from)),
                )?;
                set_note(&object, note)?;
            }
            TokenEvent::Freeze(identity, note) | TokenEvent::Unfreeze(identity, note) => {
                set_property(
                    &object,
                    "identity",
                    &JsValue::from(IdentifierWasm::from(*identity)),
                )?;
                set_note(&object, note)?;
            }
            TokenEvent::DestroyFrozenFunds(identity, amount, note) => {
                set_property(
                    &object,
                    "identity",
                    &JsValue::from(IdentifierWasm::from(*identity)),
                )?;
                set_amount(&object, *amount)?;
                set_note(&object, note)?;
            }
            TokenEvent::Transfer(recipient, note, shared, private, amount) => {
                set_property(
                    &object,
                    "recipient",
                    &JsValue::from(IdentifierWasm::from(*recipient)),
                )?;
                set_note(&object, note)?;

                match shared {
                    Some(value) => set_property(
                        &object,
                        "sharedEncryptedNote",
                        &JsValue::from(SharedEncryptedNoteWasm::from(value.clone())),
                    )?,
                    None => set_property(&object, "sharedEncryptedNote", &JsValue::NULL)?,
                }

                match private {
                    Some(value) => set_property(
                        &object,
                        "privateEncryptedNote",
                        &JsValue::from(PrivateEncryptedNoteWasm::from(value.clone())),
                    )?,
                    None => set_property(&object, "privateEncryptedNote", &JsValue::NULL)?,
                }

                set_amount(&object, *amount)?;
            }
            TokenEvent::Claim(distribution, amount, note) => {
                let distribution_js =
                    serde_wasm_bindgen::to_value(distribution).map_err(|err| {
                        WasmDppError::serialization(format!(
                            "unable to serialize token distribution recipient: {}",
                            err
                        ))
                    })?;
                set_property(&object, "distribution", &distribution_js)?;
                set_amount(&object, *amount)?;
                set_note(&object, note)?;
            }
            TokenEvent::EmergencyAction(action, note) => {
                set_property(
                    &object,
                    "action",
                    &JsValue::from(TokenEmergencyActionWasm::from(*action)),
                )?;
                set_note(&object, note)?;
            }
            TokenEvent::ConfigUpdate(change, note) => {
                set_property(
                    &object,
                    "change",
                    &JsValue::from(TokenConfigurationChangeItemWasm::from(change.clone())),
                )?;
                set_note(&object, note)?;
            }
            TokenEvent::ChangePriceForDirectPurchase(schedule, note) => {
                match schedule {
                    Some(s) => set_property(
                        &object,
                        "pricingSchedule",
                        &JsValue::from(TokenPricingScheduleWasm::from(s.clone())),
                    )?,
                    None => set_property(&object, "pricingSchedule", &JsValue::NULL)?,
                }
                set_note(&object, note)?;
            }
            TokenEvent::DirectPurchase(amount, credits) => {
                set_amount(&object, *amount)?;
                set_property(&object, "credits", &BigInt::from(*credits))?;
            }
        }

        Ok(object.into())
    }
}

fn set_amount(object: &Object, amount: u64) -> WasmDppResult<()> {
    set_property(object, "amount", &BigInt::from(amount))
}

fn set_amount_json(object: &Object, amount: u64) -> WasmDppResult<()> {
    set_property(object, "amount", &JsValue::from(amount as f64))
}

fn set_note(object: &Object, note: &Option<String>) -> WasmDppResult<()> {
    match note {
        Some(value) => set_property(object, "note", &JsValue::from_str(value)),
        None => set_property(object, "note", &JsValue::NULL),
    }
}

fn set_property(object: &Object, key: &str, value: &JsValue) -> WasmDppResult<()> {
    Reflect::set(object, &JsValue::from_str(key), value).map_err(|err| {
        let message = err.error_message();
        WasmDppError::generic(format!("unable to set property '{key}': {message}"))
    })?;

    Ok(())
}
