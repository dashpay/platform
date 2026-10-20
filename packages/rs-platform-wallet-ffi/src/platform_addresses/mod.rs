//! FFI bindings for platform address wallet operations.
//!
//! Mirrors the structure of `platform_wallet::wallet::platform_addresses`.

mod fund_from_asset_lock;
mod funding_fee;
mod sync;
mod transfer;
mod wallet;
mod withdrawal;

// Re-export all FFI types and functions.
pub use fund_from_asset_lock::*;
pub use funding_fee::*;
pub use sync::*;
pub use transfer::*;
pub use wallet::*;
pub use withdrawal::*;

use crate::error::*;
use crate::platform_address_types::*;
use platform_wallet::wallet::platform_addresses::InputSelection;

use crate::runtime::runtime;

/// Parse an `InputSelectionType` + raw arrays into a Rust `InputSelection`.
///
/// On error, returns the populated `PlatformWalletFFIResult` so the caller
/// can early-return it directly:
/// `let sel = unwrap_result_or_return!(parse_input_selection(...));`
///
/// # Safety
/// Pointers must be valid for their respective counts.
pub(crate) unsafe fn parse_input_selection(
    input_type: InputSelectionType,
    explicit_inputs: *const ExplicitInputFFI,
    explicit_inputs_count: usize,
    nonce_inputs: *const ExplicitInputWithNonceFFI,
    nonce_inputs_count: usize,
) -> Result<InputSelection, PlatformWalletFFIResult> {
    match input_type {
        InputSelectionType::Auto => Ok(InputSelection::Auto),
        InputSelectionType::Explicit => {
            match parse_explicit_inputs(explicit_inputs, explicit_inputs_count) {
                Ok(m) => Ok(InputSelection::Explicit(m)),
                Err(e) => Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    e,
                )),
            }
        }
        InputSelectionType::ExplicitWithNonces => {
            match parse_explicit_inputs_with_nonces(nonce_inputs, nonce_inputs_count) {
                Ok(m) => Ok(InputSelection::ExplicitWithNonces(m)),
                Err(e) => Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    e,
                )),
            }
        }
    }
}
