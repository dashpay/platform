//! Persist the protocol version auto-detect learns, per network, so the next
//! SDK instance seeds at it instead of at the per-network floor.
//!
//! rs-sdk keeps the detected version in the instance only: every page load
//! starts at the floor and ratchets up once a proof verifies. A first request
//! that only parses at the network's real version (a contract using newer
//! index grammar, say) fails at the floor instead of ratcheting. Writing the
//! learned version to `localStorage` under `dash-sdk.protocol-version.<network>`
//! and seeding it back through `SdkBuilder::with_initial_version` removes that
//! first-request gap for mainnet and testnet. Devnets are re-cut from the
//! development line and already seed at the current version; regtest is local.
//! Both are left alone.
//!
//! The seed keeps auto-detect on, so a network that has moved further still
//! ratchets the SDK (and this store) upward. A version this build does not
//! know is ignored, as is a value at or below the floor.

use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::sdk::min_protocol_version;
use js_sys::{Array, Function, Reflect};
use wasm_bindgen::{JsCast, JsValue};

const KEY_PREFIX: &str = "dash-sdk.protocol-version.";

/// The storage key for a network, or `None` for networks that are not persisted.
pub(crate) fn storage_key(network: Network) -> Option<String> {
    match network {
        Network::Mainnet => Some(format!("{KEY_PREFIX}mainnet")),
        Network::Testnet => Some(format!("{KEY_PREFIX}testnet")),
        Network::Devnet | Network::Regtest => None,
    }
}

/// The persisted version for `network`, when one is stored, this build knows it,
/// and it lies above the network's floor (a seed at or below the floor is a no-op).
pub(crate) fn load(network: Network) -> Option<&'static PlatformVersion> {
    let key = storage_key(network)?;
    let storage = local_storage()?;
    let raw = call_method(&storage, "getItem", &[JsValue::from_str(&key)])?.as_string()?;
    let version = raw.trim().parse::<u32>().ok()?;
    if version <= min_protocol_version(network) {
        return None;
    }
    PlatformVersion::get(version).ok()
}

/// Persist `version` for `network`. Silent when the network is not persisted or
/// no `localStorage` is reachable (workers, Node, blocked site data).
pub(crate) fn store(network: Network, version: u32) {
    let Some(key) = storage_key(network) else {
        return;
    };
    let Some(storage) = local_storage() else {
        return;
    };
    call_method(
        &storage,
        "setItem",
        &[
            JsValue::from_str(&key),
            JsValue::from_str(&version.to_string()),
        ],
    );
}

/// `globalThis.localStorage`, when the host has one. Reached through `Reflect`
/// so a missing binding (a worker, Node) is a `None`, not a link error, and a
/// throwing accessor (blocked site data) is swallowed.
fn local_storage() -> Option<JsValue> {
    let storage = Reflect::get(&js_sys::global(), &JsValue::from_str("localStorage")).ok()?;
    if storage.is_undefined() || storage.is_null() {
        return None;
    }
    Some(storage)
}

fn call_method(target: &JsValue, name: &str, args: &[JsValue]) -> Option<JsValue> {
    let method = Reflect::get(target, &JsValue::from_str(name))
        .ok()?
        .dyn_into::<Function>()
        .ok()?;
    let arguments = Array::new();
    for arg in args {
        arguments.push(arg);
    }
    Reflect::apply(&method, target, &arguments).ok()
}
