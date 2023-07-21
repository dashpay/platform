#[cfg(feature = "bindgen")]
/// Generation of bindings
pub mod bindgen;

/// Data formats used for uniffi bindings
pub mod codec;

/// Proof bindings
pub mod proof;

/// Return version of rs-drive-li
#[no_mangle]
#[uniffi::export]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
