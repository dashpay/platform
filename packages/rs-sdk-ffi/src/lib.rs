//! Dash Platform SDK FFI bindings
//!
//! This crate provides C-compatible FFI bindings for the Dash Platform SDK,
//! enabling cross-platform applications to interact with Dash Platform through C interfaces.

mod contested_resource;
mod data_contract;
mod document;
mod error;
mod evonode;
mod group;
mod identity;
mod protocol_version;
mod sdk;
mod signer;
mod system;
mod token;
mod types;
mod utils;
mod voting;

#[cfg(test)]
mod test_utils;

pub use contested_resource::*;
pub use data_contract::*;
pub use document::*;
pub use error::*;
pub use evonode::*;
pub use group::*;
pub use identity::*;
pub use protocol_version::*;
pub use sdk::*;
pub use signer::*;
pub use system::*;
pub use token::*;
pub use types::*;
pub use utils::*;
pub use voting::*;

use std::panic;

/// Initialize the FFI library.
/// This should be called once at app startup before using any other functions.
#[no_mangle]
pub extern "C" fn dash_sdk_init() {
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

        eprintln!("Dash SDK FFI panic: {}{}", msg, location);
    }));

    // Initialize any other subsystems if needed
}

/// Get the version of the Dash SDK FFI library
#[no_mangle]
pub extern "C" fn dash_sdk_version() -> *const std::os::raw::c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const std::os::raw::c_char
}
