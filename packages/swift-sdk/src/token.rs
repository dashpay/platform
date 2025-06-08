use crate::error::{SwiftDashError, SwiftDashResult};
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// Token information
#[repr(C)]
pub struct SwiftDashTokenInfo {
    pub contract_id: *mut c_char,
    pub name: *mut c_char,
    pub symbol: *mut c_char,
    pub total_supply: u64,
    pub decimals: u8,
}

/// Get token total supply
#[no_mangle]
pub extern "C" fn swift_dash_token_get_total_supply(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    token_contract_id: *const c_char,
) -> *mut c_char {
    if sdk_handle.is_null() || token_contract_id.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_token_get_total_supply(sdk_handle, token_contract_id);

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut c_char
    }
}

/// Token transfer parameters
#[repr(C)]
pub struct SwiftDashTokenTransferParams {
    pub token_contract_id: *const c_char, // Base58 encoded
    pub serialized_contract: *const u8,   // Optional contract data
    pub serialized_contract_len: usize,
    pub token_position: u16,
    pub recipient_id: *const u8, // 32 bytes
    pub amount: u64,
    pub public_note: *const c_char,            // Optional, can be null
    pub private_encrypted_note: *const c_char, // Optional
    pub shared_encrypted_note: *const c_char,  // Optional
}

/// Transfer tokens
#[no_mangle]
pub extern "C" fn swift_dash_token_transfer(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    transition_owner_id: *const u8, // 32 bytes - sender identity ID
    params: *const SwiftDashTokenTransferParams,
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> SwiftDashResult {
    if sdk_handle.is_null()
        || transition_owner_id.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter(
            "Missing required parameters",
        ));
    }

    unsafe {
        let params = &*params;

        // Create FFI params
        let ffi_params = rs_sdk_ffi::DashSDKTokenTransferParams {
            token_contract_id: params.token_contract_id,
            serialized_contract: params.serialized_contract,
            serialized_contract_len: params.serialized_contract_len,
            token_position: params.token_position,
            recipient_id: params.recipient_id,
            amount: params.amount,
            public_note: params.public_note,
            private_encrypted_note: params.private_encrypted_note,
            shared_encrypted_note: params.shared_encrypted_note,
        };

        let result = rs_sdk_ffi::dash_sdk_token_transfer(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            transition_owner_id,
            &ffi_params,
            identity_public_key_handle,
            signer_handle,
            ptr::null(), // Use default put settings
            ptr::null(), // Use default state transition creation options
        );

        if !result.error.is_null() {
            let error = Box::from_raw(result.error);
            return SwiftDashResult::error(SwiftDashError::from_ffi_error(&*error));
        }

        SwiftDashResult::success()
    }
}

/// Token mint parameters
#[repr(C)]
pub struct SwiftDashTokenMintParams {
    pub token_contract_id: *const c_char, // Base58 encoded
    pub serialized_contract: *const u8,   // Optional contract data
    pub serialized_contract_len: usize,
    pub token_position: u16,
    pub recipient_id: *const u8, // 32 bytes - optional, can be null (defaults to minter)
    pub amount: u64,
    pub public_note: *const c_char, // Optional, can be null
}

/// Mint tokens
#[no_mangle]
pub extern "C" fn swift_dash_token_mint(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    transition_owner_id: *const u8, // 32 bytes - minter identity ID
    params: *const SwiftDashTokenMintParams,
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> SwiftDashResult {
    if sdk_handle.is_null()
        || transition_owner_id.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter(
            "Missing required parameters",
        ));
    }

    unsafe {
        let params = &*params;

        // Create FFI params
        let ffi_params = rs_sdk_ffi::DashSDKTokenMintParams {
            token_contract_id: params.token_contract_id,
            serialized_contract: params.serialized_contract,
            serialized_contract_len: params.serialized_contract_len,
            token_position: params.token_position,
            recipient_id: params.recipient_id,
            amount: params.amount,
            public_note: params.public_note,
        };

        let result = rs_sdk_ffi::dash_sdk_token_mint(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            transition_owner_id,
            &ffi_params,
            identity_public_key_handle,
            signer_handle,
            ptr::null(), // Use default put settings
            ptr::null(), // Use default state transition creation options
        );

        if !result.error.is_null() {
            let error = Box::from_raw(result.error);
            return SwiftDashResult::error(SwiftDashError::from_ffi_error(&*error));
        }

        SwiftDashResult::success()
    }
}

/// Token burn parameters
#[repr(C)]
pub struct SwiftDashTokenBurnParams {
    pub token_contract_id: *const c_char, // Base58 encoded
    pub serialized_contract: *const u8,   // Optional contract data
    pub serialized_contract_len: usize,
    pub token_position: u16,
    pub amount: u64,
    pub public_note: *const c_char, // Optional, can be null
}

/// Burn tokens
#[no_mangle]
pub extern "C" fn swift_dash_token_burn(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    transition_owner_id: *const u8, // 32 bytes - burner identity ID
    params: *const SwiftDashTokenBurnParams,
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> SwiftDashResult {
    if sdk_handle.is_null()
        || transition_owner_id.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter(
            "Missing required parameters",
        ));
    }

    unsafe {
        let params = &*params;

        // Create FFI params
        let ffi_params = rs_sdk_ffi::DashSDKTokenBurnParams {
            token_contract_id: params.token_contract_id,
            serialized_contract: params.serialized_contract,
            serialized_contract_len: params.serialized_contract_len,
            token_position: params.token_position,
            amount: params.amount,
            public_note: params.public_note,
        };

        let result = rs_sdk_ffi::dash_sdk_token_burn(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            transition_owner_id,
            &ffi_params,
            identity_public_key_handle,
            signer_handle,
            ptr::null(), // Use default put settings
            ptr::null(), // Use default state transition creation options
        );

        if !result.error.is_null() {
            let error = Box::from_raw(result.error);
            return SwiftDashResult::error(SwiftDashError::from_ffi_error(&*error));
        }

        SwiftDashResult::success()
    }
}

/// Free token info structure
#[no_mangle]
pub unsafe extern "C" fn swift_dash_token_info_free(info: *mut SwiftDashTokenInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);
    if !info.contract_id.is_null() {
        let _ = CString::from_raw(info.contract_id);
    }
    if !info.name.is_null() {
        let _ = CString::from_raw(info.name);
    }
    if !info.symbol.is_null() {
        let _ = CString::from_raw(info.symbol);
    }
}
