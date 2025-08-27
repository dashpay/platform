//! Dash Unified SDK FFI bindings
//!
//! This crate provides C-compatible FFI bindings for both Dash Core (SPV) and Platform SDKs,
//! enabling cross-platform applications to interact with the complete Dash ecosystem through C interfaces.

mod callback_bridge;
mod contested_resource;
mod context_callbacks;
pub mod context_provider;
#[cfg(test)]
mod context_provider_stubs;
mod core_sdk;
mod crypto;
mod data_contract;
mod document;
mod dpns;
mod error;
mod evonode;
mod group;
mod identity;
mod key_wallet;
mod protocol_version;
mod sdk;
mod signer;
mod signer_simple;
mod system;
mod token;
mod transaction;
mod types;
mod unified;
mod utils;
mod voting;

#[cfg(test)]
mod test_utils;

pub use callback_bridge::*;
pub use contested_resource::*;
pub use context_callbacks::*;
pub use context_provider::*;
pub use core_sdk::*;
pub use crypto::*;
pub use data_contract::*;
pub use document::*;
pub use dpns::*;
pub use error::*;
pub use evonode::*;
pub use group::*;
pub use identity::*;
pub use key_wallet::*;
pub use protocol_version::*;
pub use sdk::*;
pub use signer::*;
pub use signer_simple::*;
pub use system::*;
pub use token::*;
pub use transaction::*;
pub use types::*;
pub use unified::*;
pub use utils::*;
pub use voting::*;

// Re-export all Core SDK functions and types for unified access
pub use dash_spv_ffi::*;

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

/// Enable logging with the specified level
/// Level values: 0 = Error, 1 = Warn, 2 = Info, 3 = Debug, 4 = Trace
#[no_mangle]
pub extern "C" fn dash_sdk_enable_logging(level: u8) {
    use std::env;

    let log_level = match level {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        4 => "trace",
        _ => "info",
    };

    // Set RUST_LOG environment variable for detailed logging
    env::set_var(
        "RUST_LOG",
        format!(
            "dash_sdk={},rs_sdk={},dapi_grpc={},h2={},tower={},hyper={},tonic={}",
            log_level, log_level, log_level, log_level, log_level, log_level, log_level
        ),
    );

    // Note: env_logger initialization is done in SDK creation
    // We just set the environment variable here

    eprintln!("🔵 Logging enabled at level: {}", log_level);
}

/// Get the version of the Dash SDK FFI library
#[no_mangle]
pub extern "C" fn dash_sdk_version() -> *const std::os::raw::c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const std::os::raw::c_char
}
