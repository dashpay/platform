//! FFI bindings for PlatformWalletManager (wallet lifecycle management).

use crate::error::*;
use crate::event_handler::{EventHandlerCallbacks, FFIEventHandler};
use crate::handle::*;
use crate::persistence::{FFIPersister, PersistenceCallbacks};
use crate::runtime::runtime;

use dash_sdk::Sdk;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::PlatformWalletManager;
use std::os::raw::c_void;
use std::sync::Arc;

/// Create a new PlatformWalletManager.
///
/// `sdk_ptr` must point to a valid `dash_sdk::Sdk` instance — typically
/// obtained from `dash_sdk_get_inner_sdk_ptr` in `rs-sdk-ffi`, which
/// hands back a `*const c_void` pointing at the live `Sdk` field of the
/// wrapper. The FFI caller is responsible for keeping that `Sdk` alive
/// for the duration of this call. The Sdk is cloned and wrapped in Arc
/// before being stored on the manager.
///
/// We deliberately accept `*const c_void` here rather than `*const Sdk`
/// so that this header is self-contained — `Sdk` is a `dash-sdk` type
/// that cbindgen cannot expose without dragging the entire crate's
/// internal layout into the C ABI.
///
/// `persistence` and `event_handler` are callback vtables whose `context`
/// pointers must remain valid for the lifetime of the manager.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_create(
    sdk_ptr: *const c_void,
    persistence: *const PersistenceCallbacks,
    event_handler: *const EventHandlerCallbacks,
    out_handle: *mut Handle,
    // No detailed error path today — the constructor is infallible
    // once the null-pointer check passes. Kept in the ABI for
    // forward-compat.
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if sdk_ptr.is_null() || persistence.is_null() || event_handler.is_null() || out_handle.is_null()
    {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let sdk = Arc::new((*(sdk_ptr as *const Sdk)).clone());
    let persister = Arc::new(FFIPersister::new(std::ptr::read(persistence)));
    let handler: Arc<dyn platform_wallet::PlatformEventHandler> =
        Arc::new(FFIEventHandler::new(std::ptr::read(event_handler)));

    let manager = PlatformWalletManager::new(sdk, persister, handler);
    let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(manager);
    *out_handle = handle;

    PlatformWalletFFIResult::Success
}

/// Create a wallet from raw seed bytes (64 bytes).
///
/// On success, `out_wallet_handle` is set to a `PlatformWallet` handle and
/// `out_wallet_id` is filled with the 32-byte wallet ID.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_create_wallet_from_seed(
    manager_handle: Handle,
    network: u32,
    seed_bytes: *const u8,
    seed_len: usize,
    account_options: u32,
    out_wallet_handle: *mut Handle,
    out_wallet_id: *mut [u8; 32],
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if seed_bytes.is_null() || out_wallet_handle.is_null() || out_wallet_id.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    if seed_len != 64 {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorInvalidParameter,
                format!("Seed must be 64 bytes, got {}", seed_len),
            );
        }
        return PlatformWalletFFIResult::ErrorInvalidParameter;
    }

    let network = match network {
        0 => Network::Mainnet,
        1 => Network::Testnet,
        2 => Network::Devnet,
        3 => Network::Regtest,
        _ => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidNetwork,
                    format!("Unknown network: {}", network),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidNetwork;
        }
    };

    let mut seed = [0u8; 64];
    std::ptr::copy_nonoverlapping(seed_bytes, seed.as_mut_ptr(), 64);

    let accounts = match account_options {
        0 => WalletAccountCreationOptions::None,
        1 => WalletAccountCreationOptions::Default,
        _ => WalletAccountCreationOptions::Default,
    };

    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |manager| {
            match runtime().block_on(manager.create_wallet_from_seed_bytes(network, seed, accounts))
            {
                Ok(wallet) => {
                    let wallet_id = wallet.wallet_id();
                    let wallet_handle = PLATFORM_WALLET_STORAGE.insert(wallet);
                    *out_wallet_handle = wallet_handle;
                    *out_wallet_id = wallet_id;
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Create a wallet from a BIP39 mnemonic phrase (English).
///
/// On success, `out_wallet_handle` is set to a `PlatformWallet` handle and
/// `out_wallet_id` is filled with the 32-byte wallet ID.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_create_wallet_from_mnemonic(
    manager_handle: Handle,
    mnemonic: *const std::os::raw::c_char,
    network: u32,
    account_options: u32,
    out_wallet_handle: *mut Handle,
    out_wallet_id: *mut [u8; 32],
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if mnemonic.is_null() || out_wallet_handle.is_null() || out_wallet_id.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let mnemonic_str = match std::ffi::CStr::from_ptr(mnemonic).to_str() {
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
    };

    let network = match network {
        0 => Network::Mainnet,
        1 => Network::Testnet,
        2 => Network::Devnet,
        3 => Network::Regtest,
        _ => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidNetwork,
                    format!("Unknown network: {}", network),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidNetwork;
        }
    };

    let accounts = match account_options {
        0 => WalletAccountCreationOptions::None,
        _ => WalletAccountCreationOptions::Default,
    };

    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |manager| {
            match runtime().block_on(manager.create_wallet_from_mnemonic(
                mnemonic_str,
                network,
                accounts,
            )) {
                Ok(wallet) => {
                    let wallet_id = wallet.wallet_id();
                    let wallet_handle = PLATFORM_WALLET_STORAGE.insert(wallet);
                    *out_wallet_handle = wallet_handle;
                    *out_wallet_id = wallet_id;
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Hydrate the manager from its persister.
///
/// Triggers `on_load_wallet_list_fn` on the persistence callbacks to
/// fetch the persisted wallet list from the client side (SwiftData),
/// reconstructs each wallet as **watch-only** via its stored root +
/// per-account xpubs, and registers them inside the manager. Does not
/// produce wallet handles — the caller should follow up with
/// [`platform_wallet_manager_get_wallet`] per `wallet_id` it knows
/// about.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_load_from_persistor(
    manager_handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |manager| {
            match runtime().block_on(manager.load_from_persistor()) {
                Ok(()) => PlatformWalletFFIResult::Success,
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Get a `PlatformWallet` handle for a wallet registered in the
/// manager. Returns `ErrorWalletNotFound` if no wallet with the given
/// id is currently held.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_get_wallet(
    manager_handle: Handle,
    wallet_id: *const [u8; 32],
    out_wallet_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if wallet_id.is_null() || out_wallet_handle.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    let wallet_id_value = *wallet_id;

    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |manager| {
            match runtime().block_on(manager.get_wallet(&wallet_id_value)) {
                Some(wallet) => {
                    let handle = PLATFORM_WALLET_STORAGE.insert(wallet);
                    *out_wallet_handle = handle;
                    PlatformWalletFFIResult::Success
                }
                None => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!(
                                "Wallet {} not found in manager",
                                hex::encode(wallet_id_value)
                            ),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Destroy a PlatformWalletManager handle.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_destroy(
    handle: Handle,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if let Some(manager) = PLATFORM_WALLET_MANAGER_STORAGE.remove(handle) {
        manager.platform_address_sync().stop();
    }
    PlatformWalletFFIResult::Success
}
