use crate::sdk::SwiftDashPutSettings;
use std::ffi::CString;
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
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_id: *const c_char,
) -> *mut rs_sdk_ffi::IdentityHandle {
    if sdk_handle.is_null() || identity_id.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_fetch(sdk_handle, identity_id);

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::IdentityHandle
    }
}

/// Get identity information
#[no_mangle]
pub extern "C" fn swift_dash_identity_get_info(
    identity_handle: *mut rs_sdk_ffi::IdentityHandle,
) -> *mut SwiftDashIdentityInfo {
    if identity_handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_get_info(identity_handle);

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data.is_null() {
            return ptr::null_mut();
        }

        let ffi_info_ptr = result.data as *mut rs_sdk_ffi::IOSSDKIdentityInfo;
        let ffi_info = *Box::from_raw(ffi_info_ptr);

        // Convert to Swift-friendly structure
        let swift_info = Box::new(SwiftDashIdentityInfo {
            id: ffi_info.id, // Transfer ownership
            balance: ffi_info.balance,
            revision: ffi_info.revision,
            public_keys_count: ffi_info.public_keys_count,
        });

        Box::into_raw(swift_info)
    }
}

/// Put identity to platform with instant lock and return serialized state transition
#[no_mangle]
pub extern "C" fn swift_dash_identity_put_to_platform_with_instant_lock(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_handle: *mut rs_sdk_ffi::IdentityHandle,
    public_key_id: u32,
    signer_handle: *mut rs_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null() || identity_handle.is_null() || signer_handle.is_null() {
        return ptr::null_mut();
    }

    let ffi_settings: *const rs_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_put_to_platform_with_instant_lock(
            sdk_handle,
            identity_handle,
            public_key_id,
            signer_handle,
            ffi_settings,
        );

        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data_type != rs_sdk_ffi::IOSSDKResultDataType::BinaryData {
            return ptr::null_mut();
        }

        if result.data.is_null() {
            return ptr::null_mut();
        }

        let ffi_binary_ptr = result.data as *mut rs_sdk_ffi::IOSSDKBinaryData;
        let ffi_binary = *Box::from_raw(ffi_binary_ptr);

        // Convert to Swift-friendly structure
        let swift_binary = Box::new(SwiftDashBinaryData {
            data: ffi_binary.data, // Transfer ownership
            len: ffi_binary.len,
        });

        Box::into_raw(swift_binary)
    }
}

/// Put identity to platform with instant lock and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_identity_put_to_platform_with_instant_lock_and_wait(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_handle: *mut rs_sdk_ffi::IdentityHandle,
    public_key_id: u32,
    signer_handle: *mut rs_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut rs_sdk_ffi::IdentityHandle {
    if sdk_handle.is_null() || identity_handle.is_null() || signer_handle.is_null() {
        return ptr::null_mut();
    }

    let ffi_settings: *const rs_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_put_to_platform_with_instant_lock_and_wait(
            sdk_handle,
            identity_handle,
            public_key_id,
            signer_handle,
            ffi_settings,
        );

        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data_type != rs_sdk_ffi::IOSSDKResultDataType::IdentityHandle {
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::IdentityHandle
    }
}

/// Put identity to platform with chain lock and return serialized state transition
#[no_mangle]
pub extern "C" fn swift_dash_identity_put_to_platform_with_chain_lock(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_handle: *mut rs_sdk_ffi::IdentityHandle,
    public_key_id: u32,
    signer_handle: *mut rs_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null() || identity_handle.is_null() || signer_handle.is_null() {
        return ptr::null_mut();
    }

    let ffi_settings: *const rs_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_put_to_platform_with_chain_lock(
            sdk_handle,
            identity_handle,
            public_key_id,
            signer_handle,
            ffi_settings,
        );

        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data_type != rs_sdk_ffi::IOSSDKResultDataType::BinaryData {
            return ptr::null_mut();
        }

        if result.data.is_null() {
            return ptr::null_mut();
        }

        let ffi_binary_ptr = result.data as *mut rs_sdk_ffi::IOSSDKBinaryData;
        let ffi_binary = *Box::from_raw(ffi_binary_ptr);

        // Convert to Swift-friendly structure
        let swift_binary = Box::new(SwiftDashBinaryData {
            data: ffi_binary.data, // Transfer ownership
            len: ffi_binary.len,
        });

        Box::into_raw(swift_binary)
    }
}

/// Put identity to platform with chain lock and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_identity_put_to_platform_with_chain_lock_and_wait(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_handle: *mut rs_sdk_ffi::IdentityHandle,
    public_key_id: u32,
    signer_handle: *mut rs_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut rs_sdk_ffi::IdentityHandle {
    if sdk_handle.is_null() || identity_handle.is_null() || signer_handle.is_null() {
        return ptr::null_mut();
    }

    let ffi_settings: *const rs_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_put_to_platform_with_chain_lock_and_wait(
            sdk_handle,
            identity_handle,
            public_key_id,
            signer_handle,
            ffi_settings,
        );

        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data_type != rs_sdk_ffi::IOSSDKResultDataType::IdentityHandle {
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::IdentityHandle
    }
}

/// Transfer credits to another identity
#[no_mangle]
pub extern "C" fn swift_dash_identity_transfer_credits(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_handle: *mut rs_sdk_ffi::IdentityHandle,
    recipient_id: *const c_char,
    amount: u64,
    public_key_id: u32,
    signer_handle: *mut rs_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashTransferCreditsResult {
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || recipient_id.is_null()
        || signer_handle.is_null()
    {
        return ptr::null_mut();
    }

    let ffi_settings: *const rs_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_transfer_credits(
            sdk_handle,
            identity_handle,
            recipient_id,
            amount,
            public_key_id,
            signer_handle,
            ffi_settings,
        );

        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data.is_null() {
            return ptr::null_mut();
        }

        let ffi_transfer_ptr = result.data as *mut rs_sdk_ffi::IOSSDKTransferCreditsResult;
        let ffi_transfer = *Box::from_raw(ffi_transfer_ptr);

        // Convert to Swift-friendly structure
        let swift_transfer = Box::new(SwiftDashTransferCreditsResult {
            amount: ffi_transfer.amount,
            recipient_id: ffi_transfer.recipient_id, // Transfer ownership
            transaction_data: ffi_transfer.transaction_data, // Transfer ownership
            transaction_data_len: ffi_transfer.transaction_data_len,
        });

        Box::into_raw(swift_transfer)
    }
}

/// Free a Swift identity info structure
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

/// Free a Swift binary data structure
#[no_mangle]
pub unsafe extern "C" fn swift_dash_binary_data_free(binary_data: *mut SwiftDashBinaryData) {
    if binary_data.is_null() {
        return;
    }

    let data = Box::from_raw(binary_data);
    if !data.data.is_null() && data.len > 0 {
        // Reconstruct the Vec to properly deallocate
        let _ = Vec::from_raw_parts(data.data, data.len, data.len);
    }
}

/// Create a new identity
#[no_mangle]
pub extern "C" fn swift_dash_identity_create(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
) -> *mut rs_sdk_ffi::IdentityHandle {
    if sdk_handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_create(sdk_handle);

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::IdentityHandle
    }
}

/// Top up identity with instant lock
#[no_mangle]
pub extern "C" fn swift_dash_identity_topup_with_instant_lock(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_handle: *mut rs_sdk_ffi::IdentityHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const u8,
    private_key_len: usize,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
    {
        return ptr::null_mut();
    }

    let ffi_settings: *const rs_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_topup_with_instant_lock(
            sdk_handle,
            identity_handle,
            instant_lock_bytes,
            instant_lock_len,
            transaction_bytes,
            transaction_len,
            output_index,
            private_key,
            private_key_len,
            ffi_settings,
        );

        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data.is_null() {
            return ptr::null_mut();
        }

        let ffi_binary_ptr = result.data as *mut rs_sdk_ffi::IOSSDKBinaryData;
        let ffi_binary = *Box::from_raw(ffi_binary_ptr);

        // Convert to Swift-friendly structure
        let swift_binary = Box::new(SwiftDashBinaryData {
            data: ffi_binary.data, // Transfer ownership
            len: ffi_binary.len,
        });

        Box::into_raw(swift_binary)
    }
}

/// Top up identity with instant lock and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_identity_topup_with_instant_lock_and_wait(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_handle: *mut rs_sdk_ffi::IdentityHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const u8,
    private_key_len: usize,
    settings: *const SwiftDashPutSettings,
) -> *mut rs_sdk_ffi::IdentityHandle {
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
    {
        return ptr::null_mut();
    }

    let ffi_settings: *const rs_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_topup_with_instant_lock_and_wait(
            sdk_handle,
            identity_handle,
            instant_lock_bytes,
            instant_lock_len,
            transaction_bytes,
            transaction_len,
            output_index,
            private_key,
            private_key_len,
            ffi_settings,
        );

        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::IdentityHandle
    }
}

/// Withdraw credits from identity to Dash address
#[no_mangle]
pub extern "C" fn swift_dash_identity_withdraw(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_handle: *mut rs_sdk_ffi::IdentityHandle,
    address: *const c_char,
    amount: u64,
    core_fee_per_byte: u32,
    public_key_id: u32,
    signer_handle: *mut rs_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || address.is_null()
        || signer_handle.is_null()
    {
        return ptr::null_mut();
    }

    let ffi_settings: *const rs_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_withdraw(
            sdk_handle,
            identity_handle,
            address,
            amount,
            core_fee_per_byte,
            public_key_id,
            signer_handle,
            ffi_settings,
        );

        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data.is_null() {
            return ptr::null_mut();
        }

        let ffi_binary_ptr = result.data as *mut rs_sdk_ffi::IOSSDKBinaryData;
        let ffi_binary = *Box::from_raw(ffi_binary_ptr);

        // Convert to Swift-friendly structure
        let swift_binary = Box::new(SwiftDashBinaryData {
            data: ffi_binary.data, // Transfer ownership
            len: ffi_binary.len,
        });

        Box::into_raw(swift_binary)
    }
}

/// Fetch identity balance only
#[no_mangle]
pub extern "C" fn swift_dash_identity_fetch_balance(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_id: *const c_char,
) -> u64 {
    if sdk_handle.is_null() || identity_id.is_null() {
        return 0;
    }

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_fetch_balance(sdk_handle, identity_id);

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return 0;
        }

        // Return balance directly as u64
        result.data as u64
    }
}

/// Fetch identity public keys as JSON
#[no_mangle]
pub extern "C" fn swift_dash_identity_fetch_public_keys(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_id: *const c_char,
) -> *mut c_char {
    if sdk_handle.is_null() || identity_id.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_fetch_public_keys(sdk_handle, identity_id);

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut c_char
    }
}

/// Register a DPNS name for identity
#[no_mangle]
pub extern "C" fn swift_dash_identity_register_name(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    identity_handle: *mut rs_sdk_ffi::IdentityHandle,
    name: *const c_char,
    public_key_id: u32,
    signer_handle: *mut rs_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || name.is_null()
        || signer_handle.is_null()
    {
        return ptr::null_mut();
    }

    let ffi_settings: *const rs_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_register_name(
            sdk_handle,
            identity_handle,
            name,
            public_key_id,
            signer_handle,
            ffi_settings,
        );

        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data.is_null() {
            return ptr::null_mut();
        }

        let ffi_binary_ptr = result.data as *mut rs_sdk_ffi::IOSSDKBinaryData;
        let ffi_binary = *Box::from_raw(ffi_binary_ptr);

        // Convert to Swift-friendly structure
        let swift_binary = Box::new(SwiftDashBinaryData {
            data: ffi_binary.data, // Transfer ownership
            len: ffi_binary.len,
        });

        Box::into_raw(swift_binary)
    }
}

/// Resolve a DPNS name to identity ID
#[no_mangle]
pub extern "C" fn swift_dash_identity_resolve_name(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    name: *const c_char,
) -> *mut c_char {
    if sdk_handle.is_null() || name.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_identity_resolve_name(sdk_handle, name);

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut c_char
    }
}

/// Free a Swift transfer credits result structure
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
