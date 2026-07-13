//! Internal helpers shared across the `proof_result` submodules.
//!
//! These items are `pub(super)` only — they are not part of the public WASM
//! surface and exist purely to keep the per-domain modules DRY.

use crate::DocumentWasm;
use crate::PlatformAddressWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::utils::JsMapExt;
use dpp::document::Document;
use dpp::platform_value::Identifier;
use js_sys::{BigInt, Map};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// Build a plain JS object from key-value pairs.
pub(super) fn js_obj(entries: &[(&str, JsValue)]) -> JsValue {
    let obj = js_sys::Object::new();
    for (key, val) in entries {
        js_sys::Reflect::set(&obj, &(*key).into(), val).unwrap();
    }
    obj.into()
}

/// Read a `Map`-shaped property from an ingested JS value.
///
/// `toJSON` normalizes a `Map` to a plain object so it survives
/// `JSON.stringify`. A value that round-tripped through
/// `JSON.parse(JSON.stringify(...))` therefore arrives here as a plain
/// object, not a `Map`. Accept both: use a real `Map` directly, otherwise
/// rebuild one from the plain object's entries so `.size`/`.get()`/iteration
/// behave as a `Map`.
pub(super) fn read_map_property(value: &JsValue, name: &str) -> WasmDppResult<Map> {
    let raw = js_sys::Reflect::get(value, &name.into())
        .map_err(|_| WasmDppError::generic(format!("Missing property: {}", name)))?;
    if raw.is_instance_of::<Map>() {
        Ok(raw.unchecked_into())
    } else if raw.is_object() {
        let entries = js_sys::Object::entries(raw.unchecked_ref());
        let map = Map::new();
        for entry in entries.iter() {
            let pair: js_sys::Array = entry.unchecked_into();
            map.set(&pair.get(0), &pair.get(1));
        }
        Ok(map)
    } else {
        Err(WasmDppError::generic(format!(
            "Property {} must be a Map or plain object",
            name
        )))
    }
}

/// Wrap a raw `Document` into `DocumentWasm`.
///
/// `DocumentWasm` requires metadata (contract ID, type name) that a bare
/// `Document` does not carry.  When converting proof-result documents we
/// use empty defaults — the actual document data (id, owner_id, properties,
/// revision, timestamps) is fully preserved.
pub(super) fn doc_to_wasm(doc: Document) -> DocumentWasm {
    DocumentWasm::new(doc, Identifier::default(), String::new(), None)
}

/// Helper to build `Map<string, { address: PlatformAddress, nonce: number, credits: BigInt } | undefined>`
/// from the Rust address-info BTreeMap.  Shared by three variants.
///
/// Keys are hex-encoded PlatformAddress bytes so that JS consumers can
/// look up entries by string (JS Map uses reference equality for object keys).
pub(super) fn build_address_infos_map(
    map: std::collections::BTreeMap<dpp::address_funds::PlatformAddress, Option<(u32, u64)>>,
) -> Map {
    Map::from_entries(map.into_iter().map(|(address, info)| {
        let address_wasm = PlatformAddressWasm::from(address);
        let key: JsValue = address_wasm.to_hex().into();
        let val: JsValue = match info {
            Some((nonce, credits)) => {
                let obj = js_sys::Object::new();
                js_sys::Reflect::set(&obj, &"address".into(), &address_wasm.into()).unwrap();
                js_sys::Reflect::set(&obj, &"nonce".into(), &nonce.into()).unwrap();
                js_sys::Reflect::set(&obj, &"credits".into(), &BigInt::from(credits).into())
                    .unwrap();
                obj.into()
            }
            None => JsValue::undefined(),
        };
        (key, val)
    }))
}

/// Helper to build `Map<string(hex), boolean>` from shielded nullifier results.
pub(super) fn build_nullifier_map(nullifiers: Vec<(Vec<u8>, bool)>) -> Map {
    Map::from_entries(nullifiers.into_iter().map(|(nullifier, is_spent)| {
        let key: JsValue = hex::encode(&nullifier).into();
        let val: JsValue = is_spent.into();
        (key, val)
    }))
}

pub(super) fn action_status_to_string(
    status: dpp::group::group_action_status::GroupActionStatus,
) -> String {
    match status {
        dpp::group::group_action_status::GroupActionStatus::ActionActive => {
            "ActionActive".to_string()
        }
        dpp::group::group_action_status::GroupActionStatus::ActionClosed => {
            "ActionClosed".to_string()
        }
    }
}
