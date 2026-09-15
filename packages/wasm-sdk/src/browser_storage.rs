//! The page's `localStorage`, reached without a `web-sys` dependency.
//!
//! Everything the SDK remembers across page loads (the protocol version
//! auto-detect learned, cached data contracts) goes through here. On the
//! wasm32 target the functions talk to `globalThis.localStorage` through
//! `js_sys::Reflect`, so a host without one (a worker, Node) or a throwing
//! accessor (blocked site data) is a `None` or a no-op, never a link error
//! or a panic. Off wasm32, the crate is compiled natively for its unit
//! tests, where every wasm-bindgen import panics when called; there the
//! store is a thread-local map, so the persistence paths run for real and
//! each test thread sees its own empty storage.

#[cfg(target_arch = "wasm32")]
mod imp {
    use js_sys::{Array, Function, Reflect};
    use wasm_bindgen::{JsCast, JsValue};

    /// `globalThis.localStorage`, when the host has one.
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

    pub(super) fn get_item(key: &str) -> Option<String> {
        let storage = local_storage()?;
        call_method(&storage, "getItem", &[JsValue::from_str(key)])?.as_string()
    }

    pub(super) fn set_item(key: &str, value: &str) {
        if let Some(storage) = local_storage() {
            call_method(
                &storage,
                "setItem",
                &[JsValue::from_str(key), JsValue::from_str(value)],
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static STORE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    }

    pub(super) fn get_item(key: &str) -> Option<String> {
        STORE.with(|store| store.borrow().get(key).cloned())
    }

    pub(super) fn set_item(key: &str, value: &str) {
        STORE.with(|store| {
            store
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
        });
    }
}

/// The stored value for `key`, if any.
pub(crate) fn get_item(key: &str) -> Option<String> {
    imp::get_item(key)
}

/// Store `value` under `key`. Silent when no storage is reachable.
pub(crate) fn set_item(key: &str, value: &str) {
    imp::set_item(key, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        assert_eq!(get_item("dash-sdk.test.key"), None);
        set_item("dash-sdk.test.key", "value");
        assert_eq!(get_item("dash-sdk.test.key").as_deref(), Some("value"));
        set_item("dash-sdk.test.key", "other");
        assert_eq!(get_item("dash-sdk.test.key").as_deref(), Some("other"));
    }
}
