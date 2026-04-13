//! FFI bindings for platform address wallet operations.
//!
//! Mirrors the structure of `platform_wallet::wallet::platform_addresses`.

mod fund_from_asset_lock;
mod sync;
mod transfer;
mod wallet;
mod withdrawal;

// Re-export all FFI types and functions.
pub use fund_from_asset_lock::*;
pub use sync::*;
pub use transfer::*;
pub use wallet::*;
pub use withdrawal::*;

use crate::error::*;
use crate::platform_address_types::*;
use dpp::address_funds::PlatformAddress;
use platform_wallet::wallet::platform_addresses::InputSelection;

/// Shared tokio runtime for blocking on async wallet operations.
pub(crate) fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: once_cell::sync::Lazy<tokio::runtime::Runtime> = once_cell::sync::Lazy::new(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for platform-wallet-ffi")
    });
    &RT
}

/// Parse an `InputSelectionType` + raw arrays into a Rust `InputSelection`.
///
/// # Safety
/// Pointers must be valid for their respective counts.
pub(crate) unsafe fn parse_input_selection(
    input_type: InputSelectionType,
    explicit_inputs: *const ExplicitInputFFI,
    explicit_inputs_count: usize,
    nonce_inputs: *const ExplicitInputWithNonceFFI,
    nonce_inputs_count: usize,
    out_error: *mut PlatformWalletFFIError,
) -> Result<InputSelection, PlatformWalletFFIResult> {
    match input_type {
        InputSelectionType::Auto => Ok(InputSelection::Auto),
        InputSelectionType::Explicit => {
            match parse_explicit_inputs(explicit_inputs, explicit_inputs_count) {
                Ok(m) => Ok(InputSelection::Explicit(m)),
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidParameter,
                            e,
                        );
                    }
                    Err(PlatformWalletFFIResult::ErrorInvalidParameter)
                }
            }
        }
        InputSelectionType::ExplicitWithNonces => {
            match parse_explicit_inputs_with_nonces(nonce_inputs, nonce_inputs_count) {
                Ok(m) => Ok(InputSelection::ExplicitWithNonces(m)),
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidParameter,
                            e,
                        );
                    }
                    Err(PlatformWalletFFIResult::ErrorInvalidParameter)
                }
            }
        }
    }
}
