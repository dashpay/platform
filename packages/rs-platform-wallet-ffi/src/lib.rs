// Platform Wallet FFI Library
// Provides C-compatible FFI bindings for rs-platform-wallet

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]
// Doc list formatting nits from our Markdown-style bullet lists in rustdoc.
#![allow(clippy::doc_overindented_list_items)]
// The FFI layer wraps a large crate error enum.
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]

pub mod asset_lock;
pub mod contact;
pub mod contact_request;
pub mod core_address_types;
pub mod core_wallet;
pub mod core_wallet_types;
pub mod dashpay;
pub mod dashpay_profile;
pub mod derivation;
pub mod dpns;
pub mod error;
pub mod established_contact;
pub mod event_handler;
pub mod handle;
pub mod identity_discovery;
pub mod identity_key_preview;
pub mod identity_keys_from_mnemonic;
pub mod identity_manager;
pub mod identity_persistence;
pub mod identity_registration;
pub mod identity_registration_funded_with_signer;
pub mod identity_registration_with_signer;
pub mod identity_transfer;
pub mod identity_update;
pub mod identity_withdrawal;
pub mod managed_identity;
pub mod manager;
pub mod memory_explorer;
pub mod persistence;
pub mod platform_address_sync;
pub mod platform_address_types;
pub mod platform_addresses;
pub mod platform_wallet_info;
mod runtime;
pub mod spv;
pub mod types;
pub mod utils;
pub mod wallet;
pub mod wallet_restore_types;
pub mod xpub_render;

// Re-exports
pub use asset_lock::*;
pub use contact::*;
pub use contact_request::*;
pub use core_address_types::*;
pub use core_wallet::*;
pub use core_wallet_types::*;
pub use dashpay::*;
pub use dashpay_profile::*;
pub use derivation::*;
pub use dpns::*;
pub use error::*;
pub use established_contact::*;
pub use event_handler::*;
pub use handle::*;
pub use identity_discovery::*;
pub use identity_key_preview::*;
pub use identity_keys_from_mnemonic::*;
pub use identity_manager::*;
pub use identity_persistence::*;
pub use identity_registration_funded_with_signer::*;
pub use identity_registration_with_signer::*;
pub use identity_transfer::*;
pub use identity_update::*;
pub use identity_withdrawal::*;
pub use managed_identity::*;
pub use manager::*;
pub use memory_explorer::*;
pub use persistence::*;
pub use platform_address_sync::*;
pub use platform_address_types::*;
pub use platform_addresses::*;
pub use platform_wallet_info::*;
pub use spv::*;
pub use types::*;
pub use utils::*;
pub use wallet::*;
pub use wallet_restore_types::*;
pub use xpub_render::*;

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
