//! Canonical token-id derivation
//!
//! Exposes [`dash_sdk::dpp::tokens::calculate_token_id`] over the C ABI so
//! Swift / iOS callers don't have to mirror the protocol formula
//! (`double_sha256("dash_token" || contract_id || u16_be(position))`)
//! on the client side. This is a pure CPU function — it takes no SDK
//! handle and performs no I/O.
//!
//! See `packages/rs-dpp/src/tokens/mod.rs` for the underlying definition.

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Derive the canonical platform token id for `(contract_id, position)`.
///
/// Wraps `dash_sdk::dpp::tokens::calculate_token_id`, which computes
/// `double_sha256("dash_token" || contract_id_32 || u16_be(position))`.
///
/// # Parameters
/// - `contract_id`: NUL-terminated base58-encoded contract id (32 bytes).
/// - `position`: token contract position within the data contract.
///
/// # Returns
/// Success: a NUL-terminated base58-encoded token id (32 bytes).
/// Error: an `InvalidParameter` error if `contract_id` is null, not
/// valid UTF-8, or not a valid base58 32-byte identifier.
///
/// # Safety
/// - `contract_id` must be a valid pointer to a NUL-terminated C string
///   and readable for the duration of the call.
/// - The returned C string pointer (on success) must be freed with the
///   SDK's string-free function (`dash_sdk_string_free`) by the caller.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_calculate_token_id(
    contract_id: *const c_char,
    position: u16,
) -> DashSDKResult {
    if contract_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "contract_id is null".to_string(),
        ));
    }

    let id_str = match CStr::from_ptr(contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let contract_identifier = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid contract ID: {}", e),
            ))
        }
    };

    let contract_id_bytes: [u8; 32] = contract_identifier.to_buffer();
    let token_id_bytes = dash_sdk::dpp::tokens::calculate_token_id(&contract_id_bytes, position);

    let token_id_base58 = Identifier::new(token_id_bytes).to_string(Encoding::Base58);

    let c_str = match CString::new(token_id_base58) {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(
                FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
            )
        }
    };
    DashSDKResult::success_string(c_str.into_raw())
}
