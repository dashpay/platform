use crate::error::*;
use crate::handle::*;
use crate::types::Network;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use platform_wallet::PlatformWalletInfo;
use std::os::raw::{c_char, c_uchar};

/// Create a new PlatformWalletInfo from seed bytes.
///
/// `network` encoding matches other entry points in this crate:
/// 0 = Mainnet, 1 = Testnet, 2 = Devnet, 3 = Regtest.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_info_create_from_seed(
    network: u32,
    seed_bytes: *const c_uchar,
    seed_len: usize,
    out_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(seed_bytes);
    check_ptr!(out_handle);

    let network = match network {
        0 => Network::Mainnet,
        1 => Network::Testnet,
        2 => Network::Devnet,
        3 => Network::Regtest,
        _ => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidNetwork,
                format!("Unknown network: {network}"),
            );
        }
    };

    // Validate seed length (should be 64 bytes for BIP39)
    if seed_len != 64 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("Invalid seed length: expected 64 bytes, got {seed_len}"),
        );
    }

    let seed_slice = unsafe { std::slice::from_raw_parts(seed_bytes, seed_len) };

    // Convert to fixed-size array
    let mut seed_array = [0u8; 64];
    seed_array.copy_from_slice(seed_slice);

    let wallet = unwrap_result_or_return!(key_wallet::Wallet::from_seed_bytes(
        seed_array,
        network,
        WalletAccountCreationOptions::None,
    ));

    let platform_wallet = PlatformWalletInfo::from_wallet(&wallet, 0);

    // Store in handle storage
    let handle = WALLET_INFO_STORAGE.insert(platform_wallet);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::ok()
}

/// Create a new PlatformWalletInfo from mnemonic.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_info_create_from_mnemonic(
    network: u32,
    mnemonic: *const c_char,
    passphrase: *const c_char,
    out_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(mnemonic);
    check_ptr!(out_handle);

    let network = match network {
        0 => Network::Mainnet,
        1 => Network::Testnet,
        2 => Network::Devnet,
        3 => Network::Regtest,
        _ => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidNetwork,
                format!("Unknown network: {network}"),
            );
        }
    };

    let mnemonic_str =
        unwrap_result_or_return!(unsafe { std::ffi::CStr::from_ptr(mnemonic).to_str() });

    let passphrase_str = if passphrase.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(unsafe {
            std::ffi::CStr::from_ptr(passphrase).to_str()
        }))
    };

    let mnemonic_obj = unwrap_result_or_return!(mnemonic_str.parse::<key_wallet::Mnemonic>());

    // Create wallet from mnemonic with or without passphrase
    let wallet = if let Some(pass) = passphrase_str {
        unwrap_result_or_return!(key_wallet::Wallet::from_mnemonic_with_passphrase(
            mnemonic_obj,
            pass.to_string(),
            network,
            WalletAccountCreationOptions::None,
        ))
    } else {
        unwrap_result_or_return!(key_wallet::Wallet::from_mnemonic(
            mnemonic_obj,
            network,
            WalletAccountCreationOptions::None,
        ))
    };

    // Create PlatformWalletInfo from the wallet
    let platform_wallet = PlatformWalletInfo::from_wallet(&wallet, 0);

    // Store in handle storage
    let handle = WALLET_INFO_STORAGE.insert(platform_wallet);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::ok()
}

/// Get the identity manager
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_info_get_identity_manager(
    wallet_handle: Handle,
    out_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_handle);

    let option = WALLET_INFO_STORAGE.with_item(wallet_handle, |wallet_info| {
        wallet_info.identity_manager.clone()
    });
    let manager = unwrap_option_or_return!(option);
    let handle = IDENTITY_MANAGER_STORAGE.insert(manager);
    unsafe { *out_handle = handle };
    PlatformWalletFFIResult::ok()
}

/// Set the identity manager
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_info_set_identity_manager(
    wallet_handle: Handle,
    manager_handle: Handle,
) -> PlatformWalletFFIResult {
    let manager_option =
        IDENTITY_MANAGER_STORAGE.with_item(manager_handle, |manager| manager.clone());
    let manager = unwrap_option_or_return!(manager_option);

    let option = WALLET_INFO_STORAGE.with_item_mut(wallet_handle, |wallet_info| {
        wallet_info.identity_manager = manager;
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Destroy PlatformWalletInfo and free resources
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_info_destroy(
    wallet_handle: Handle,
) -> PlatformWalletFFIResult {
    if WALLET_INFO_STORAGE.remove(wallet_handle).is_some() {
        PlatformWalletFFIResult::ok()
    } else {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Invalid wallet handle",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_from_seed() {
        unsafe {
            let seed = [0u8; 64];
            let mut handle: Handle = NULL_HANDLE;

            let result = platform_wallet_info_create_from_seed(
                1, // Testnet
                seed.as_ptr(),
                seed.len(),
                &mut handle,
            );

            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_ne!(handle, NULL_HANDLE);

            platform_wallet_info_destroy(handle);
        }
    }

    #[test]
    fn test_create_from_mnemonic() {
        unsafe {
            let mnemonic = std::ffi::CString::new(
                "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
            ).unwrap();

            let mut handle: Handle = NULL_HANDLE;

            let result = platform_wallet_info_create_from_mnemonic(
                1, // Testnet
                mnemonic.as_ptr(),
                std::ptr::null(),
                &mut handle,
            );

            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_ne!(handle, NULL_HANDLE);

            platform_wallet_info_destroy(handle);
        }
    }

    #[test]
    fn test_destroy_invalid_handle() {
        unsafe {
            let result = platform_wallet_info_destroy(9999);
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        }
    }
}
