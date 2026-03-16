//! Orchard address derivation FFI function.

use std::ffi::CString;
use std::os::raw::c_void;

use dash_sdk::grovedb_commitment_tree::{FullViewingKey, Scope, SpendingKey};
use zeroize::Zeroize;

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};

/// Derive an Orchard payment address from a 32-byte spending key.
///
/// # Parameters
/// - `spending_key_bytes`: Pointer to a 32-byte spending key.
/// - `diversifier_index`: Address diversifier index (usually 0).
///
/// # Returns
/// `DashSDKResult` with a hex-encoded 43-byte Orchard raw address string on success.
/// The caller must free the returned string with `dash_sdk_string_free`.
///
/// # Safety
/// - `spending_key_bytes` must point to exactly 32 valid bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_derive_address(
    spending_key_bytes: *const [u8; 32],
    diversifier_index: u32,
) -> DashSDKResult {
    if spending_key_bytes.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "spending_key_bytes is null".to_string(),
        ));
    }

    let mut key_copy = *spending_key_bytes;

    let sk = match SpendingKey::from_bytes(key_copy).into_option() {
        Some(sk) => {
            key_copy.zeroize();
            sk
        }
        None => {
            key_copy.zeroize();
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid spending key bytes".to_string(),
            ));
        }
    };

    let fvk = FullViewingKey::from(&sk);
    let payment_address = fvk.address_at(diversifier_index, Scope::External);
    let raw_bytes = payment_address.to_raw_address_bytes();
    let hex_str = hex::encode(raw_bytes);

    match CString::new(hex_str) {
        Ok(c_str) => DashSDKResult {
            data_type: DashSDKResultDataType::String,
            data: c_str.into_raw() as *mut c_void,
            error: std::ptr::null_mut(),
        },
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("Failed to create CString: {}", e),
        )),
    }
}
