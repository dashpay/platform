//! Dash Unified SDK FFI bindings
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]
//!
//! This crate provides C-compatible FFI bindings for both Dash Core (SPV) and Platform SDKs,
//! enabling cross-platform applications to interact with the complete Dash ecosystem through C interfaces.

mod address;
mod address_sync;
mod callback_bridge;
mod contested_resource;
mod context_callbacks;
pub mod context_provider;
#[cfg(test)]
mod context_provider_stubs;
mod crypto;
mod dashpay;
mod data_contract;
mod document;
mod dpns;
mod error;
mod evonode;
mod group;
mod identity;
mod nullifier_sync;
mod platform_wallet_types;
mod protocol_version;
mod sdk;
mod shielded;
mod signer;
mod signer_simple;
mod system;
mod token;
mod types;
mod unified;
mod utils;
mod voting;

#[cfg(test)]
mod test_utils;

pub use address::*;
pub use address_sync::*;
pub use callback_bridge::*;
pub use contested_resource::*;
pub use context_callbacks::*;
pub use context_provider::*;
pub use crypto::*;
pub use dashpay::*;
pub use data_contract::*;
pub use document::*;
pub use dpns::*;
pub use error::*;
pub use evonode::*;
pub use group::*;
pub use identity::*;
pub use nullifier_sync::*;
#[allow(unused_imports)]
pub use platform_wallet_types::*;
pub use protocol_version::*;
pub use sdk::*;
pub use shielded::*;
pub use signer::*;
pub use signer_simple::*;
pub use system::*;
pub use token::*;
pub use types::*;
pub use unified::*;
pub use utils::*;
pub use voting::*;

// Re-export all Core SDK functions and types for unified access when linked
#[cfg(feature = "dash_spv")]
pub use dash_spv_ffi as core_ffi;

// Re-export Platform Wallet FFI functions and types for DashPay support
// Note: We re-export selectively to avoid conflicts with rs-sdk-ffi's own modules
pub use platform_wallet_ffi::{
    // Contact request functions
    contact_request_create,
    contact_request_destroy,
    contact_request_get_account_reference,
    contact_request_get_created_at,
    contact_request_get_encrypted_public_key,
    contact_request_get_recipient_id,
    contact_request_get_recipient_key_index,
    contact_request_get_sender_id,
    contact_request_get_sender_key_index,
    established_contact_clear_alias,
    established_contact_clear_note,
    established_contact_destroy,
    established_contact_get_alias,
    // Established contact functions
    established_contact_get_contact_identity_id,
    established_contact_get_note,
    established_contact_hide,
    established_contact_is_hidden,
    established_contact_set_alias,
    established_contact_set_note,
    established_contact_unhide,
    identity_manager_add_identity,
    // IdentityManager functions
    identity_manager_create,
    identity_manager_destroy,
    identity_manager_get_all_identity_ids,
    identity_manager_get_identity,
    identity_manager_get_identity_count,
    identity_manager_get_primary_identity_id,
    identity_manager_remove_identity,
    identity_manager_set_primary_identity,
    managed_identity_accept_contact_request,
    // ManagedIdentity functions
    managed_identity_create_from_identity_bytes,
    managed_identity_destroy,
    managed_identity_get_balance,
    managed_identity_get_established_contact,
    managed_identity_get_established_contact_ids,
    managed_identity_get_id,
    managed_identity_get_incoming_contact_request,
    managed_identity_get_incoming_contact_request_ids,
    managed_identity_get_label,
    managed_identity_get_last_synced_keys_block_time,
    managed_identity_get_last_updated_balance_block_time,
    managed_identity_get_sent_contact_request,
    // Contact management functions
    managed_identity_get_sent_contact_request_ids,
    managed_identity_is_contact_established,
    managed_identity_reject_contact_request,
    managed_identity_send_contact_request,
    managed_identity_set_label,
    managed_identity_set_last_updated_balance_block_time,
    platform_wallet_bytes_free,
    platform_wallet_ffi_error_free,
    // Core functions
    platform_wallet_ffi_init,
    platform_wallet_ffi_version,
    // Utility functions
    platform_wallet_generate_random_identifier,
    platform_wallet_identifier_array_free,
    platform_wallet_identifier_from_hex,
    platform_wallet_identifier_to_hex,
    platform_wallet_info_create_from_mnemonic,
    // PlatformWalletInfo functions
    platform_wallet_info_create_from_seed,
    platform_wallet_info_destroy,
    platform_wallet_info_get_identity_manager,
    platform_wallet_info_set_identity_manager,
    platform_wallet_string_free,
    BlockTime,
    // Types
    Handle,
    IdentifierArray,
    IdentifierBytes,
    PlatformWalletFFIError,
    PlatformWalletFFIResult,
    NULL_HANDLE,
};

/// Initialize the FFI library.
/// This should be called once at app startup before using any other functions.
#[no_mangle]
pub extern "C" fn dash_sdk_init() {
    // NOTE: Panic handler setup removed to avoid conflicts with dash-unified-ffi
    // The unified library sets its own panic handler in dash_unified_init()

    // Initialize context callbacks storage
    init_global_callbacks();

    // Initialize any other subsystems if needed
}

/// Enable logging with the specified level.
///
/// This function initializes a `tracing` subscriber with the given log level.
/// It configures per-crate filter directives directly, without touching
/// environment variables, making it safe to call from any thread context
/// (including after a Tokio runtime has started).
///
/// If a global subscriber has already been set (e.g., by a previous call),
/// subsequent calls are a no-op and the original level is retained.
///
/// Level values: 0 = Error, 1 = Warn, 2 = Info, 3 = Debug, 4 = Trace
#[no_mangle]
pub extern "C" fn dash_sdk_enable_logging(level: u8) {
    let log_level = match level {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        4 => "trace",
        _ => "info",
    };

    // Build the filter string with per-crate directives -- identical to what
    // was previously stored in RUST_LOG, but constructed in-process so there
    // is no data-race with concurrent `env::var` reads on other threads.
    let filter_string = format!(
        "dash_sdk={log_level},rs_sdk={log_level},rs_sdk_ffi={log_level},\
         dapi_grpc={log_level},h2={log_level},tower={log_level},\
         hyper={log_level},tonic={log_level}"
    );

    // Create an EnvFilter directly from the string -- this never reads
    // environment variables.
    let filter = tracing_subscriber::EnvFilter::new(filter_string);

    // Initialize the global tracing subscriber.  `try_init` returns Err if a
    // subscriber is already installed; we intentionally ignore that so that
    // calling this function more than once is harmless.
    if tracing_subscriber::fmt::fmt()
        .with_env_filter(filter)
        .try_init()
        .is_ok()
    {
        tracing::info!(level = log_level, "logging enabled");
    }
}

/// Get the version of the Dash SDK FFI library
#[no_mangle]
pub extern "C" fn dash_sdk_version() -> *const std::os::raw::c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const std::os::raw::c_char
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `dash_sdk_enable_logging` does NOT set the RUST_LOG
    /// environment variable.  This is the core property that makes the
    /// function safe to call from a multi-threaded context.
    #[test]
    fn enable_logging_does_not_set_env_var() {
        // If RUST_LOG is already set by the test harness or environment,
        // we cannot reliably detect whether our function sets it, so skip.
        if std::env::var_os("RUST_LOG").is_some() {
            return;
        }

        // Call the function under test with each supported level.
        for level in 0..=5 {
            dash_sdk_enable_logging(level);
        }

        // The function must NOT have set RUST_LOG.
        assert!(
            std::env::var("RUST_LOG").is_err(),
            "RUST_LOG should not be set by dash_sdk_enable_logging; \
             env::set_var must not be used because it is a data race \
             in multi-threaded programs"
        );
    }

    /// Verify that the function can be called from multiple threads
    /// concurrently without panicking (i.e., no data race).
    #[test]
    fn enable_logging_is_thread_safe() {
        let handles: Vec<_> = (0..4)
            .map(|i| {
                std::thread::spawn(move || {
                    dash_sdk_enable_logging(i % 5);
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread should not panic");
        }
    }
}
