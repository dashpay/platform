use crate::utils::getters::VecU8ToUint8Array;
use dpp::group::action_event::GroupActionEvent;
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
    // Serialize via DPP's canonical serde, matching wasm-dpp2's `to_object`:
    // internally tagged (`$kind`/`$type`), identifier and byte fields become
    // `Uint8Array`, and u64 amounts become `BigInt` (JS-safe above 2^53). This
    // replaces a hand-rolled construction that emitted a divergent bare-`type`,
    // PascalCase shape.
    use serde::Serialize;
    let value = dpp::platform_value::to_value(event)
        .map_err(|e| JsValue::from_str(&format!("Failed to convert group action event: {e}")))?;
    // rs-dpp preserves typed map keys (e.g. `TokenPricingSchedule::SetPrices`
    // emits `Value::U64` keys); JS plain objects require string keys, so
    // stringify them before handing the tree to serde_wasm_bindgen.
    let normalized = stringify_map_keys_for_object(&value);
    let serializer = serde_wasm_bindgen::Serializer::new()
        .serialize_maps_as_objects(true)
        .serialize_bytes_as_arrays(false)
        .serialize_large_number_types_as_bigints(true);
    normalized
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize group action event: {e}")))
}

/// Recursively normalize a `Value` tree so that all `Map` keys are `Value::Text`.
/// JS plain objects require string keys, but rs-dpp preserves typed map keys
/// (e.g. `BTreeMap<TokenAmount, Credits>` emits `Value::U64` keys). Mirrors
/// wasm-dpp2's `platform_value_to_object` normalization.
fn stringify_map_keys_for_object(value: &dpp::platform_value::Value) -> dpp::platform_value::Value {
    use dpp::platform_value::Value;
    match value {
        Value::Map(entries) => Value::Map(
            entries
                .iter()
                .map(|(k, v)| (stringify_key(k), stringify_map_keys_for_object(v)))
                .collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.iter().map(stringify_map_keys_for_object).collect())
        }
        other => other.clone(),
    }
}

fn stringify_key(key: &dpp::platform_value::Value) -> dpp::platform_value::Value {
    use dpp::platform_value::string_encoding::{encode, Encoding};
    use dpp::platform_value::Value;
    match key {
        Value::Text(_) => key.clone(),
        Value::U8(n) => Value::Text(n.to_string()),
        Value::U16(n) => Value::Text(n.to_string()),
        Value::U32(n) => Value::Text(n.to_string()),
        Value::U64(n) => Value::Text(n.to_string()),
        Value::I8(n) => Value::Text(n.to_string()),
        Value::I16(n) => Value::Text(n.to_string()),
        Value::I32(n) => Value::Text(n.to_string()),
        Value::I64(n) => Value::Text(n.to_string()),
        Value::Bool(b) => Value::Text(b.to_string()),
        Value::Identifier(bytes) => Value::Text(encode(bytes, Encoding::Base58)),
        Value::Bytes(bytes) => Value::Text(encode(bytes, Encoding::Base64)),
        Value::Bytes20(bytes) => Value::Text(encode(bytes, Encoding::Base64)),
        Value::Bytes32(bytes) => Value::Text(encode(bytes, Encoding::Base64)),
        Value::Bytes36(bytes) => Value::Text(encode(bytes, Encoding::Base64)),
        // Float / Null / Array / Map / Tag / EnumU8 fall through; the serializer
        // surfaces a clear error if any ever appears as a map key (no rs-dpp
        // domain type in this tree uses them that way).
        other => other.clone(),
    }
}

// Pure-Rust coverage of the `Value`-tree the group-action serializer feeds to
// `serde_wasm_bindgen`. These assert the canonical `$kind`/`$type` shape and the
// typed-map-key normalization; the final JsValue mapping (typed `U64` -> BigInt,
// `Identifier` -> Uint8Array) is the wasm-dpp2-proven serializer config and is not
// re-tested here (it needs a wasm runtime).
#[cfg(test)]
mod tests {
    use super::*;
    use dpp::platform_value::Value;
    use dpp::tokens::token_event::TokenEvent;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use std::collections::BTreeMap;

    fn event_value(event: &GroupActionEvent) -> Value {
        let value = dpp::platform_value::to_value(event).expect("to_value");
        stringify_map_keys_for_object(&value)
    }

    fn field<'a>(map: &'a Value, key: &str) -> Option<&'a Value> {
        map.as_map()?
            .iter()
            .find(|(k, _)| k.as_text() == Some(key))
            .map(|(_, v)| v)
    }

    #[test]
    fn mint_event_has_canonical_kind_type_shape() {
        let event = GroupActionEvent::TokenEvent(TokenEvent::Mint(
            1234,
            Identifier::new([0x11; 32]),
            Some("note".to_string()),
        ));
        let v = event_value(&event);
        // Internally tagged: `$kind` (GroupActionEvent) + `$type` (TokenEvent),
        // flattened into one object. Amount stays a typed `U64` (-> BigInt),
        // recipient a typed `Identifier` (-> Uint8Array) at serialize time.
        assert_eq!(
            field(&v, "$kind").and_then(Value::as_text),
            Some("tokenEvent")
        );
        assert_eq!(field(&v, "$type").and_then(Value::as_text), Some("mint"));
        assert_eq!(field(&v, "amount"), Some(&Value::U64(1234)));
        assert!(matches!(field(&v, "recipient"), Some(Value::Identifier(_))));
    }

    #[test]
    fn set_prices_map_keys_are_stringified() {
        // `TokenPricingSchedule::SetPrices` emits `BTreeMap<u64, u64>` -> `U64`
        // map keys; JS plain objects require string keys, so normalization must
        // convert them (else serde_wasm_bindgen errors "Map key is not a string").
        let mut prices = BTreeMap::new();
        prices.insert(5u64, 50u64);
        prices.insert(9_007_199_254_740_993u64, 1u64); // > 2^53
        let event = GroupActionEvent::TokenEvent(TokenEvent::ChangePriceForDirectPurchase(
            Some(TokenPricingSchedule::SetPrices(prices)),
            None,
        ));
        let v = event_value(&event);
        assert_eq!(
            field(&v, "$type").and_then(Value::as_text),
            Some("changePriceForDirectPurchase")
        );
        // Walk to the pricing-schedule map and assert every key is now `Text`.
        let mut found_map = false;
        fn assert_all_keys_text(v: &Value, found: &mut bool) {
            match v {
                Value::Map(entries) => {
                    for (k, val) in entries {
                        if matches!(val, Value::U64(_)) && matches!(k, Value::U64(_)) {
                            panic!("u64 map key survived normalization: {k:?}");
                        }
                        assert!(matches!(k, Value::Text(_)), "non-string map key: {k:?}");
                        *found = true;
                        assert_all_keys_text(val, found);
                    }
                }
                Value::Array(items) => items.iter().for_each(|i| assert_all_keys_text(i, found)),
                _ => {}
            }
        }
        assert_all_keys_text(&v, &mut found_map);
        assert!(found_map, "expected at least one map in the tree");
    }
}
