//! iOS SDK FFI bindings for Dash Platform SDK
//! 
//! This crate provides C-compatible FFI bindings for the Dash Platform SDK,
//! enabling iOS applications to interact with Dash Platform through Swift.

mod error;
mod types;
mod sdk;
mod identity;
mod document;
mod data_contract;
mod utils;

pub use error::*;
pub use types::*;
pub use sdk::*;
pub use identity::*;
pub use document::*;
pub use data_contract::*;
pub use utils::*;

use std::panic;

/// Initialize the FFI library.
/// This should be called once at app startup before using any other functions.
#[no_mangle]
pub extern "C" fn ios_sdk_init() {
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
        
        eprintln!("iOS SDK FFI panic: {}{}", msg, location);
    }));
    
    // Initialize any other subsystems if needed
}

/// Get the version of the iOS SDK FFI library
#[no_mangle]
pub extern "C" fn ios_sdk_version() -> *const std::os::raw::c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const std::os::raw::c_char
}