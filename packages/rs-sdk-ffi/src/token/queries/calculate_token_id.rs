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

#[cfg(test)]
mod tests {
    use super::*;

    /// Lock the FFI marshalling against the protocol formula. The
    /// FFI is a thin direct call into
    /// `dash_sdk::dpp::tokens::calculate_token_id`, so functional
    /// drift from the formula isn't really possible — what this
    /// test pins down is the C-ABI shape: CString in, base58 round-
    /// trip, success-string out.
    #[test]
    fn matches_dpp_calculate_token_id() {
        // Arbitrary fixed contract id — any 32-byte identifier
        // works. Picking one that exercises both the high and low
        // halves so a marshalling slip would be visible.
        let contract_id_bytes: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0xf0, 0xe1, 0xd2, 0xc3, 0xb4, 0xa5, 0x96, 0x87, 0x78, 0x69, 0x5a, 0x4b,
            0x3c, 0x2d, 0x1e, 0x0f,
        ];
        let position: u16 = 7;

        // Reference: call the protocol function directly.
        let expected_bytes =
            dash_sdk::dpp::tokens::calculate_token_id(&contract_id_bytes, position);
        let expected_b58 = Identifier::new(expected_bytes).to_string(Encoding::Base58);

        // FFI: marshal contract id through base58 + CString, run
        // the FFI, decode the returned base58 back to bytes, and
        // compare.
        let contract_id_b58 = Identifier::new(contract_id_bytes).to_string(Encoding::Base58);
        let contract_id_cstring = CString::new(contract_id_b58).expect("contract id has no NUL");

        let result = unsafe { dash_sdk_calculate_token_id(contract_id_cstring.as_ptr(), position) };

        // No error.
        assert!(result.error.is_null(), "FFI surfaced an error");
        assert!(!result.data.is_null(), "FFI returned null data");

        let returned_b58 = unsafe {
            CStr::from_ptr(result.data as *const c_char)
                .to_str()
                .expect("returned token id is not valid UTF-8")
                .to_owned()
        };

        // Free the returned CString through the same allocator
        // shape `success_string` produced (CString::into_raw).
        unsafe {
            let _ = CString::from_raw(result.data as *mut c_char);
        }

        assert_eq!(
            returned_b58, expected_b58,
            "FFI token id ({returned_b58}) disagrees with dpp::calculate_token_id ({expected_b58})"
        );
    }

    /// Null contract id should round-trip into an InvalidParameter
    /// error rather than crash.
    #[test]
    fn rejects_null_contract_id() {
        let result = unsafe { dash_sdk_calculate_token_id(std::ptr::null(), 0) };
        assert!(result.data.is_null(), "expected no data on null input");
        assert!(!result.error.is_null(), "expected an error on null input");
        unsafe {
            // `DashSDKError` is heap-allocated via `Box::into_raw`; reclaim it.
            let _ = Box::from_raw(result.error);
        }
    }
}
