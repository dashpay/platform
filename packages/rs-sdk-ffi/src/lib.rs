//! Dash Platform SDK FFI bindings
//!
//! This crate provides C-compatible FFI bindings for the Dash Platform SDK,
//! enabling cross-platform applications to interact with Dash Platform through C interfaces.

mod contested_resource;
mod context_callbacks;
mod context_provider;
#[cfg(test)]
mod context_provider_stubs;
// core_stubs module removed - no longer needed with callback approach
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
pub use context_callbacks::*;
pub use context_provider::*;
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
pub use voting::*;

use std::panic;

/// Initialize the FFI library.
/// This should be called once at app startup before using any other functions.
#[no_mangle]
pub extern "C" fn dash_sdk_init() {
    // NOTE: Panic handler setup removed to avoid conflicts with dash-unified-ffi
    // The unified library sets its own panic handler in dash_unified_init()
    
    // Initialize context callbacks storage
    context_callbacks::init_global_callbacks();
    
    // Initialize any other subsystems if needed
}

/// Get the version of the Dash SDK FFI library
#[no_mangle]
pub extern "C" fn dash_sdk_version() -> *const std::os::raw::c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const std::os::raw::c_char
}
