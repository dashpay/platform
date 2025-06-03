//! Swift-friendly SDK wrapper for Dash Platform
//!
//! This crate provides an idiomatic Swift-compatible C FFI interface
//! over the ios-sdk-ffi crate, making it easier to use from Swift.

mod data_contract;
mod document;
mod error;
mod identity;
mod sdk;
mod signer;
mod token;

#[cfg(test)]
mod tests;

// The ios_sdk_ffi crate is available through Cargo.toml

pub use data_contract::*;
pub use document::*;
pub use error::*;
pub use identity::*;
pub use sdk::*;
pub use signer::*;
pub use token::*;

use std::panic;

/// Initialize the Swift SDK library.
/// This should be called once at app startup before using any other functions.
#[no_mangle]
pub extern "C" fn swift_dash_sdk_init() {
    // Set up panic hook to prevent unwinding across FFI boundary
    panic::set_hook(Box::new(|panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic"
        };

        let location = if let Some(location) = panic_info.location() {
            format!(" at {}:{}", location.file(), location.line())
        } else {
            String::new()
        };

        eprintln!("Swift Dash SDK panic: {}{}", msg, location);
    }));

    // Initialize the underlying FFI
    unsafe {
        ios_sdk_ffi::ios_sdk_init();
    }
}

/// Get the version of the Swift Dash SDK library
#[no_mangle]
pub extern "C" fn swift_dash_sdk_version() -> *const std::os::raw::c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const std::os::raw::c_char
}
