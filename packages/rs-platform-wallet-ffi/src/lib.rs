// Platform Wallet FFI Library
// Provides C-compatible FFI bindings for rs-platform-wallet
//
// This library also re-exports all of rs-sdk-ffi, providing a unified FFI
// library for both Platform SDK and Platform Wallet functionality.

#![allow(non_camel_case_types)]

pub mod contact;
pub mod contact_request;
pub mod error;
pub mod established_contact;
pub mod handle;
pub mod identity_manager;
pub mod managed_identity;
pub mod platform_sync;
pub mod platform_wallet_info;
pub mod types;
pub mod utils;

// Re-exports from platform-wallet-ffi modules
pub use contact::*;
pub use contact_request::*;
pub use error::*;
pub use established_contact::*;
pub use handle::*;
pub use identity_manager::*;
pub use managed_identity::*;
pub use platform_sync::*;
pub use platform_wallet_info::*;
pub use types::*;
pub use utils::*;

// Re-export all of rs-sdk-ffi for unified library access
// Consumers can link just platform-wallet-ffi to get both SDK and wallet functionality
pub use rs_sdk_ffi::*;

/// Initialize the FFI library
/// Must be called before using any other functions
#[no_mangle]
pub extern "C" fn platform_wallet_ffi_init() {
    // Initialize any global state if needed
    // Currently a no-op but kept for future compatibility
}

/// Get the version of the platform wallet FFI library
#[no_mangle]
pub extern "C" fn platform_wallet_ffi_version() -> *const std::os::raw::c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const std::os::raw::c_char
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        platform_wallet_ffi_init();
        // Should not panic
    }

    #[test]
    fn test_version() {
        let version = platform_wallet_ffi_version();
        assert!(!version.is_null());

        let version_str = unsafe { std::ffi::CStr::from_ptr(version).to_str().unwrap() };
        assert!(!version_str.is_empty());
    }
}
