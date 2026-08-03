// Platform Wallet FFI Library
// Provides C-compatible FFI bindings for rs-platform-wallet

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]
// Doc list formatting nits from our Markdown-style bullet lists in rustdoc.
#![allow(clippy::doc_overindented_list_items)]
// The FFI layer wraps a large crate error enum.
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]

pub mod address_private_key;
pub mod asset_lock;
pub mod asset_lock_persistence;
pub mod contact;
pub mod contact_info;
pub mod contact_persistence;
pub mod contact_request;
pub mod core_address_types;
pub mod core_wallet;
pub mod core_wallet_types;
pub mod dashpay;
pub mod dashpay_payment;
pub mod dashpay_profile;
pub mod dashpay_sync;
pub mod data_contract;
pub mod derivation;
pub mod derive_and_persist_callbacks;
pub mod derive_identity_key_at_slot;
pub mod document;
pub mod dpns;
pub mod error;
pub mod established_contact;
pub mod event_handler;
pub mod handle;
pub mod identity_derive_and_persist;
pub mod identity_discovery;
pub mod identity_key_preview;
pub mod identity_keys_from_mnemonic;
pub mod identity_loading;
pub mod identity_manager;
pub mod identity_persistence;
pub mod identity_registration;
pub mod identity_registration_funded_with_signer;
pub mod identity_registration_with_signer;
pub mod identity_sync;
pub mod identity_top_up;
pub mod identity_transfer;
pub mod identity_update;
pub mod identity_withdrawal;
pub mod invitation;
pub mod invitation_persistence;
pub mod logging;
pub mod managed_identity;
pub mod manager;
pub mod manager_diagnostics;
pub mod memory_explorer;
pub mod mnemonic_words;
pub mod persistence;
pub mod platform_address_sync;
pub mod platform_address_types;
pub mod platform_addresses;
pub mod platform_wallet_info;
pub mod provider_key_at_index;
mod runtime;
pub mod secp256k1_primitives;
#[cfg(feature = "shielded")]
pub mod shielded_persistence;
#[cfg(feature = "shielded")]
pub mod shielded_send;
#[cfg(feature = "shielded")]
pub mod shielded_sync;
pub mod shielded_types;
pub mod sign_with_mnemonic_resolver;
pub mod spv;
pub mod token_persistence;
pub mod tokens;
pub mod types;
pub mod utils;
pub mod wallet;
pub mod wallet_registration_persistence;
pub mod wallet_restore_types;
pub mod xpub_render;

// Re-exports
pub use address_private_key::*;
pub use asset_lock::*;
pub use asset_lock_persistence::*;
pub use contact::*;
pub use contact_persistence::*;
pub use contact_request::*;
pub use core_address_types::*;
pub use core_wallet::*;
pub use core_wallet_types::*;
pub use dashpay::*;
pub use dashpay_payment::*;
pub use dashpay_profile::*;
pub use dashpay_sync::*;
pub use data_contract::*;
pub use derivation::*;
pub use derive_and_persist_callbacks::*;
pub use derive_identity_key_at_slot::*;
pub use document::*;
pub use dpns::*;
pub use error::*;
pub use established_contact::*;
pub use event_handler::*;
pub use handle::*;
pub use identity_derive_and_persist::*;
pub use identity_discovery::*;
pub use identity_key_preview::*;
pub use identity_keys_from_mnemonic::*;
pub use identity_loading::*;
pub use identity_manager::*;
pub use identity_persistence::*;
pub use identity_registration::*;
pub use identity_registration_funded_with_signer::*;
pub use identity_registration_with_signer::*;
pub use identity_sync::*;
pub use identity_top_up::*;
pub use identity_transfer::*;
pub use identity_update::*;
pub use identity_withdrawal::*;
pub use invitation::*;
pub use invitation_persistence::*;
pub use logging::*;
pub use managed_identity::*;
pub use manager::*;
pub use manager_diagnostics::*;
pub use memory_explorer::*;
pub use mnemonic_words::*;
pub use persistence::*;
pub use platform_address_sync::*;
pub use platform_address_types::*;
pub use platform_addresses::*;
pub use platform_wallet_info::*;
pub use provider_key_at_index::*;
pub use secp256k1_primitives::*;
#[cfg(feature = "shielded")]
pub use shielded_send::*;
#[cfg(feature = "shielded")]
pub use shielded_sync::*;
pub use shielded_types::*;
pub use sign_with_mnemonic_resolver::*;
pub use spv::*;
pub use token_persistence::*;
pub use tokens::*;
pub use types::*;
pub use utils::*;
pub use wallet::*;
pub use wallet_registration_persistence::*;
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
