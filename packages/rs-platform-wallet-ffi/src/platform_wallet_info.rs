use crate::error::*;
use crate::handle::*;
use crate::types::*;
use platform_wallet::platform_wallet_info::PlatformWalletInfo;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use std::os::raw::{c_char, c_uchar};

/// Create a new PlatformWalletInfo from seed bytes
#[no_mangle]
pub extern "C" fn platform_wallet_info_create_from_seed(
    seed_bytes: *const c_uchar,
    seed_len: usize,
    out_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if seed_bytes.is_null() || out_handle.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // Validate seed length (should be 64 bytes for BIP39)
    if seed_len != 64 {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("Invalid seed length: expected 64 bytes, got {}", seed_len),
                );
            }
        }
        return PlatformWalletFFIResult::ErrorInvalidParameter;
    }

    let seed_slice = unsafe { std::slice::from_raw_parts(seed_bytes, seed_len) };

    // Convert to fixed-size array
    let mut seed_array = [0u8; 64];
    seed_array.copy_from_slice(seed_slice);

    // Create wallet from seed - use empty network list, accounts can be added later
    let wallet = match key_wallet::Wallet::from_seed_bytes(
        seed_array,
        &[],  // No networks initially
        WalletAccountCreationOptions::None,  // No accounts initially
    ) {
        Ok(w) => w,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorWalletOperation,
                        format!("Failed to create wallet from seed: {}", e),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorWalletOperation;
        }
    };

    // Create ManagedWalletInfo from the wallet
    let wallet_info = key_wallet::wallet::ManagedWalletInfo::from_wallet(&wallet);

    // Create PlatformWalletInfo wrapping the ManagedWalletInfo
    let platform_wallet = PlatformWalletInfo {
        wallet_info,
        identity_managers: std::collections::BTreeMap::new(),
    };

    // Store in handle storage
    let handle = WALLET_INFO_STORAGE.insert(platform_wallet);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::Success
}

/// Create a new PlatformWalletInfo from mnemonic
#[no_mangle]
pub extern "C" fn platform_wallet_info_create_from_mnemonic(
    mnemonic: *const c_char,
    passphrase: *const c_char,
    out_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if mnemonic.is_null() || out_handle.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let mnemonic_str = unsafe {
        match std::ffi::CStr::from_ptr(mnemonic).to_str() {
            Ok(s) => s,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        "Invalid UTF-8 in mnemonic",
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        }
    };

    let passphrase_str = if passphrase.is_null() {
        None
    } else {
        unsafe {
            match std::ffi::CStr::from_ptr(passphrase).to_str() {
                Ok(s) => Some(s),
                Err(_) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorUtf8Conversion,
                            "Invalid UTF-8 in passphrase",
                        );
                    }
                    return PlatformWalletFFIResult::ErrorUtf8Conversion;
                }
            }
        }
    };

    // Parse mnemonic string
    let mnemonic_obj = match mnemonic_str.parse::<key_wallet::Mnemonic>() {
        Ok(m) => m,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        format!("Failed to parse mnemonic: {}", e),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };

    // Create wallet from mnemonic with or without passphrase
    let wallet = if let Some(pass) = passphrase_str {
        match key_wallet::Wallet::from_mnemonic_with_passphrase(
            mnemonic_obj,
            pass.to_string(),
            &[],  // No networks initially
            WalletAccountCreationOptions::None,  // No accounts initially
        ) {
            Ok(w) => w,
            Err(e) => {
                if !out_error.is_null() {
                    unsafe {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("Failed to create wallet from mnemonic with passphrase: {}", e),
                        );
                    }
                }
                return PlatformWalletFFIResult::ErrorWalletOperation;
            }
        }
    } else {
        match key_wallet::Wallet::from_mnemonic(
            mnemonic_obj,
            &[],  // No networks initially
            WalletAccountCreationOptions::None,  // No accounts initially
        ) {
            Ok(w) => w,
            Err(e) => {
                if !out_error.is_null() {
                    unsafe {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("Failed to create wallet from mnemonic: {}", e),
                        );
                    }
                }
                return PlatformWalletFFIResult::ErrorWalletOperation;
            }
        }
    };

    // Create ManagedWalletInfo from the wallet
    let wallet_info = key_wallet::wallet::ManagedWalletInfo::from_wallet(&wallet);

    // Create PlatformWalletInfo wrapping the ManagedWalletInfo
    let platform_wallet = PlatformWalletInfo {
        wallet_info,
        identity_managers: std::collections::BTreeMap::new(),
    };

    // Store in handle storage
    let handle = WALLET_INFO_STORAGE.insert(platform_wallet);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::Success
}

/// Get the identity manager for a specific network
#[no_mangle]
pub extern "C" fn platform_wallet_info_get_identity_manager(
    wallet_handle: Handle,
    network: NetworkType,
    out_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_handle.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    WALLET_INFO_STORAGE
        .with_item(wallet_handle, |wallet_info| {
            let dash_network = network.to_dash_network();

            if let Some(manager) = wallet_info.identity_managers.get(&dash_network) {
                let handle = IDENTITY_MANAGER_STORAGE.insert(manager.clone());
                unsafe { *out_handle = handle };
                PlatformWalletFFIResult::Success
            } else {
                if !out_error.is_null() {
                    unsafe {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidNetwork,
                            format!("No identity manager for network: {:?}", network),
                        );
                    }
                }
                PlatformWalletFFIResult::ErrorInvalidNetwork
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Add or update identity manager for a network
#[no_mangle]
pub extern "C" fn platform_wallet_info_set_identity_manager(
    wallet_handle: Handle,
    network: NetworkType,
    manager_handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let manager_result =
        IDENTITY_MANAGER_STORAGE.with_item(manager_handle, |manager| manager.clone());

    let manager = match manager_result {
        Some(m) => m,
        None => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity manager handle",
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidHandle;
        }
    };

    WALLET_INFO_STORAGE
        .with_item_mut(wallet_handle, |wallet_info| {
            let dash_network = network.to_dash_network();
            wallet_info.identity_managers.insert(dash_network, manager);
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Serialize PlatformWalletInfo to JSON
/// TODO: Requires serde support on PlatformWalletInfo
#[allow(dead_code)]
fn platform_wallet_info_to_json(
    wallet_handle: Handle,
    out_json: *mut *mut c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_json.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // TODO: Implement once PlatformWalletInfo has Serialize derived
    if !out_error.is_null() {
        unsafe {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorSerialization,
                "Serialization not yet implemented",
            );
        }
    }
    PlatformWalletFFIResult::ErrorSerialization
}

/// Destroy PlatformWalletInfo and free resources
#[no_mangle]
pub extern "C" fn platform_wallet_info_destroy(wallet_handle: Handle) -> PlatformWalletFFIResult {
    if WALLET_INFO_STORAGE.remove(wallet_handle).is_some() {
        PlatformWalletFFIResult::Success
    } else {
        PlatformWalletFFIResult::ErrorInvalidHandle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_from_seed() {
        let seed = [0u8; 64];
        let mut handle: Handle = NULL_HANDLE;
        let mut error = PlatformWalletFFIError::success();

        let result = platform_wallet_info_create_from_seed(
            seed.as_ptr(),
            seed.len(),
            &mut handle,
            &mut error,
        );

        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_ne!(handle, NULL_HANDLE);

        // Cleanup
        platform_wallet_info_destroy(handle);
    }

    #[test]
    fn test_create_from_mnemonic() {
        let mnemonic = std::ffi::CString::new(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        ).unwrap();

        let mut handle: Handle = NULL_HANDLE;
        let mut error = PlatformWalletFFIError::success();

        let result = platform_wallet_info_create_from_mnemonic(
            mnemonic.as_ptr(),
            std::ptr::null(),
            &mut handle,
            &mut error,
        );

        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_ne!(handle, NULL_HANDLE);

        // Cleanup
        platform_wallet_info_destroy(handle);
    }

    #[test]
    #[ignore] // Stubbed - requires serde support on PlatformWalletInfo
    fn test_to_json() {
        let seed = [0u8; 64];
        let mut handle: Handle = NULL_HANDLE;
        let mut error = PlatformWalletFFIError::success();

        platform_wallet_info_create_from_seed(seed.as_ptr(), seed.len(), &mut handle, &mut error);

        let mut json_ptr: *mut c_char = std::ptr::null_mut();
        let result = platform_wallet_info_to_json(handle, &mut json_ptr, &mut error);

        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert!(!json_ptr.is_null());

        // Cleanup
        platform_wallet_string_free(json_ptr);
        platform_wallet_info_destroy(handle);
    }

    #[test]
    fn test_destroy_invalid_handle() {
        let result = platform_wallet_info_destroy(9999);
        assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidHandle);
    }
}
