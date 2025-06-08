use crate::error::{SwiftDashError, SwiftDashResult};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Information about an identity
#[repr(C)]
pub struct SwiftDashIdentityInfo {
    pub id: *mut c_char,
    pub balance: u64,
    pub revision: u64,
    pub public_keys_count: u32,
}

/// Result of a credit transfer operation
#[repr(C)]
pub struct SwiftDashTransferCreditsResult {
    pub amount: u64,
    pub recipient_id: *mut c_char,
    pub transaction_data: *mut u8,
    pub transaction_data_len: usize,
}

/// Binary data container for results
#[repr(C)]
pub struct SwiftDashBinaryData {
    pub data: *mut u8,
    pub len: usize,
}

/// Fetch an identity by ID
#[no_mangle]
pub extern "C" fn swift_dash_identity_fetch(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    identity_id: *const c_char,
) -> *mut c_char {
    if sdk_handle.is_null() || identity_id.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_identity_fetch(sdk_handle, identity_id);

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut c_char
    }
}

/// Get identity balance
#[no_mangle]
pub extern "C" fn swift_dash_identity_get_balance(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    identity_id: *const c_char,
) -> u64 {
    if sdk_handle.is_null() || identity_id.is_null() {
        return 0;
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_identity_fetch_balance(sdk_handle, identity_id);

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return 0;
        }

        if result.data_type == rs_sdk_ffi::DashSDKResultDataType::String && !result.data.is_null() {
            let balance_str = CStr::from_ptr(result.data as *const c_char);
            if let Ok(balance_str) = balance_str.to_str() {
                if let Ok(balance) = balance_str.parse::<u64>() {
                    rs_sdk_ffi::dash_sdk_string_free(result.data as *mut c_char);
                    return balance;
                }
            }
            rs_sdk_ffi::dash_sdk_string_free(result.data as *mut c_char);
        }

        0
    }
}

/// Resolve identity name
#[no_mangle]
pub extern "C" fn swift_dash_identity_resolve_name(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    name: *const c_char,
) -> *mut c_char {
    if sdk_handle.is_null() || name.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_identity_resolve_name(sdk_handle, name);

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut c_char
    }
}

/// Transfer credits from one identity to another
#[no_mangle]
pub extern "C" fn swift_dash_identity_transfer_credits(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    from_identity_handle: *const rs_sdk_ffi::IdentityHandle,
    to_identity_id: *const c_char,
    amount: u64,
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> *mut SwiftDashTransferCreditsResult {
    if sdk_handle.is_null()
        || from_identity_handle.is_null()
        || to_identity_id.is_null()
        || signer_handle.is_null()
    {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_identity_transfer_credits(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            from_identity_handle,
            to_identity_id,
            amount,
            identity_public_key_handle, // Can be null for auto-select
            signer_handle,
            ptr::null(), // Use default put settings
        );

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        // Cast the result data to DashSDKTransferCreditsResult
        let ffi_result = result.data as *const rs_sdk_ffi::DashSDKTransferCreditsResult;
        if ffi_result.is_null() {
            return ptr::null_mut();
        }

        let _transfer_result = &*ffi_result;

        // Copy the to_identity_id string
        let to_id_cstr = CStr::from_ptr(to_identity_id);
        let recipient_id = match CString::new(to_id_cstr.to_bytes()) {
            Ok(s) => s.into_raw(),
            Err(_) => return ptr::null_mut(),
        };

        let swift_result = Box::new(SwiftDashTransferCreditsResult {
            amount,
            recipient_id,
            transaction_data: ptr::null_mut(),
            transaction_data_len: 0,
        });

        Box::into_raw(swift_result)
    }
}

/// Put identity to platform with instant lock
#[no_mangle]
pub extern "C" fn swift_dash_identity_put_to_platform_with_instant_lock(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    identity_handle: *const rs_sdk_ffi::IdentityHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const [u8; 32],
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> SwiftDashResult {
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
        || signer_handle.is_null()
    {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter(
            "Missing required parameters",
        ));
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_identity_put_to_platform_with_instant_lock(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            identity_handle,
            instant_lock_bytes,
            instant_lock_len,
            transaction_bytes,
            transaction_len,
            output_index,
            private_key,
            signer_handle,
            ptr::null(), // Use default put settings
        );

        if !result.error.is_null() {
            let error = Box::from_raw(result.error);
            return SwiftDashResult::error(SwiftDashError::from_ffi_error(&*error));
        }

        // Extract binary data from result
        if result.data_type == rs_sdk_ffi::DashSDKResultDataType::BinaryData
            && !result.data.is_null()
        {
            let binary_data = result.data as *const rs_sdk_ffi::DashSDKBinaryData;
            let binary = &*binary_data;
            SwiftDashResult::success_binary(binary.data as *mut std::os::raw::c_void, binary.len)
        } else {
            SwiftDashResult::success()
        }
    }
}

/// Put identity to platform with instant lock and wait
#[no_mangle]
pub extern "C" fn swift_dash_identity_put_to_platform_with_instant_lock_and_wait(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    identity_handle: *const rs_sdk_ffi::IdentityHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const [u8; 32],
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> *mut rs_sdk_ffi::IdentityHandle {
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
        || signer_handle.is_null()
    {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_identity_put_to_platform_with_instant_lock_and_wait(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            identity_handle,
            instant_lock_bytes,
            instant_lock_len,
            transaction_bytes,
            transaction_len,
            output_index,
            private_key,
            signer_handle,
            ptr::null(), // Use default put settings
        );

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::IdentityHandle
    }
}

/// Create identity is done by creating Identity object locally and then putting to platform
/// This is a helper note - actual creation requires proper key generation and asset lock proof
#[no_mangle]
pub extern "C" fn swift_dash_identity_create_note() -> *const c_char {
    let note = CString::new(
        "To create identity: 1. Generate keys, 2. Create asset lock, 3. Use put_to_platform",
    )
    .unwrap();
    note.into_raw()
}

/// Fetch balances for multiple identities
/// 
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_ids`: Comma-separated list of Base58-encoded identity IDs
/// 
/// # Returns
/// JSON string containing identity IDs mapped to their balances
#[no_mangle]
pub extern "C" fn swift_dash_identities_fetch_balances(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    identity_ids: *const c_char,
) -> *mut c_char {
    if sdk_handle.is_null() || identity_ids.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_identities_fetch_balances(sdk_handle, identity_ids);

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut c_char
    }
}

/// Free identity handle
#[no_mangle]
pub unsafe extern "C" fn swift_dash_identity_destroy(handle: *mut rs_sdk_ffi::IdentityHandle) {
    if !handle.is_null() {
        rs_sdk_ffi::dash_sdk_identity_destroy(handle);
    }
}

/// Free identity info structure
#[no_mangle]
pub unsafe extern "C" fn swift_dash_identity_info_free(info: *mut SwiftDashIdentityInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);
    if !info.id.is_null() {
        let _ = CString::from_raw(info.id);
    }
}

/// Free transfer result structure
#[no_mangle]
pub unsafe extern "C" fn swift_dash_transfer_credits_result_free(
    result: *mut SwiftDashTransferCreditsResult,
) {
    if result.is_null() {
        return;
    }

    let result = Box::from_raw(result);
    if !result.recipient_id.is_null() {
        let _ = CString::from_raw(result.recipient_id);
    }
    if !result.transaction_data.is_null() && result.transaction_data_len > 0 {
        let _ = Vec::from_raw_parts(
            result.transaction_data,
            result.transaction_data_len,
            result.transaction_data_len,
        );
    }
}

/// Free binary data structure
#[no_mangle]
pub unsafe extern "C" fn swift_dash_binary_data_free(data: *mut SwiftDashBinaryData) {
    if data.is_null() {
        return;
    }

    let data = Box::from_raw(data);
    if !data.data.is_null() && data.len > 0 {
        let _ = Vec::from_raw_parts(data.data, data.len, data.len);
    }
}
