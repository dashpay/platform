//! FFI bindings for PlatformWalletManager (wallet lifecycle management).

use crate::check_ptr;
use crate::error::*;
use crate::event_handler::{EventHandlerCallbacks, FFIEventHandler};
use crate::handle::*;
use crate::persistence::{FFIPersister, PersistenceCallbacks};
use crate::identity_keys_from_mnemonic::parse_mnemonic_any_language;
use crate::runtime::runtime;
use crate::types::{FFINetwork, Network};
use crate::{unwrap_option_or_return, unwrap_result_or_return};

use dash_sdk::Sdk;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
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
) -> PlatformWalletFFIResult {
    check_ptr!(sdk_ptr);
    check_ptr!(persistence);
    check_ptr!(event_handler);
    check_ptr!(out_handle);

    let sdk = Arc::new((*(sdk_ptr as *const Sdk)).clone());
    let persister = Arc::new(FFIPersister::new(std::ptr::read(persistence)));
    let handler: Arc<dyn platform_wallet::PlatformEventHandler> =
        Arc::new(FFIEventHandler::new(std::ptr::read(event_handler)));

    // `PlatformWalletManager::new` spawns the wallet-event adapter
    // task on construction (the subscriber that translates upstream
    // `WalletEvent`s into `PlatformWalletChangeSet`s). `tokio::spawn`
    // panics if no runtime is in scope, which is the default state on
    // the FFI thread — Swift calls us synchronously, no reactor
    // attached. Enter the FFI's shared runtime for the duration of
    // the constructor so the spawn lands on it; the guard drops on
    // return and leaves the spawned task running on that runtime.
    let _runtime_guard = runtime().enter();

    let manager = PlatformWalletManager::new(sdk, persister, handler);
    let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(manager);
    *out_handle = handle;

    PlatformWalletFFIResult::ok()
}

/// Map the C `has_x: bool` + `x` companion-pair idiom to a Rust `Option<u32>`.
///
/// `has == true` yields `Some(value)` — including `Some(0)`, kept distinct
/// from `has == false` which yields `None`. Mirrors the crate's `has_config`
/// optional-scalar convention; there is no `u32::MAX` sentinel.
fn birth_height_override_opt(has: bool, value: u32) -> Option<u32> {
    if has {
        Some(value)
    } else {
        None
    }
}

/// Shared body for the seed-based wallet-creation exports.
///
/// `birth_height_override` is threaded verbatim into
/// `create_wallet_from_seed_bytes`; the no-override export passes `None`.
#[allow(clippy::too_many_arguments)]
unsafe fn create_wallet_from_seed_impl(
    manager_handle: Handle,
    network: FFINetwork,
    seed_bytes: *const u8,
    seed_len: usize,
    account_options: u32,
    birth_height_override: Option<u32>,
    out_wallet_handle: *mut Handle,
    out_wallet_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(seed_bytes);
    check_ptr!(out_wallet_handle);
    check_ptr!(out_wallet_id);
    if seed_len != 64 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("Seed must be 64 bytes, got {seed_len}"),
        );
    }

    let network: Network = network.into();

    let mut seed = [0u8; 64];
    std::ptr::copy_nonoverlapping(seed_bytes, seed.as_mut_ptr(), 64);

    let accounts = match account_options {
        0 => WalletAccountCreationOptions::None,
        1 => WalletAccountCreationOptions::Default,
        _ => WalletAccountCreationOptions::Default,
    };

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        runtime().block_on(manager.create_wallet_from_seed_bytes(
            network,
            seed,
            accounts,
            birth_height_override,
        ))
    });
    let result = unwrap_option_or_return!(option);
    let wallet = unwrap_result_or_return!(result);
    let wallet_id = wallet.wallet_id();
    let wallet_handle = PLATFORM_WALLET_STORAGE.insert(wallet);
    *out_wallet_handle = wallet_handle;
    *out_wallet_id = wallet_id;
    PlatformWalletFFIResult::ok()
}

/// Shared body for the mnemonic-based wallet-creation exports.
///
/// `birth_height_override` is threaded verbatim into
/// `create_wallet_from_mnemonic`; the no-override export passes `None`.
unsafe fn create_wallet_from_mnemonic_impl(
    manager_handle: Handle,
    mnemonic: *const std::os::raw::c_char,
    network: FFINetwork,
    account_options: u32,
    birth_height_override: Option<u32>,
    out_wallet_handle: *mut Handle,
    out_wallet_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(mnemonic);
    check_ptr!(out_wallet_handle);
    check_ptr!(out_wallet_id);

    let mnemonic_str = unwrap_result_or_return!(std::ffi::CStr::from_ptr(mnemonic).to_str());

    let network: Network = network.into();

    let accounts = match account_options {
        0 => WalletAccountCreationOptions::None,
        _ => WalletAccountCreationOptions::Default,
    };

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        runtime().block_on(manager.create_wallet_from_mnemonic(
            mnemonic_str,
            network,
            accounts,
            birth_height_override,
        ))
    });
    let result = unwrap_option_or_return!(option);
    let wallet = unwrap_result_or_return!(result);
    let wallet_id = wallet.wallet_id();
    let wallet_handle = PLATFORM_WALLET_STORAGE.insert(wallet);
    *out_wallet_handle = wallet_handle;
    *out_wallet_id = wallet_id;
    PlatformWalletFFIResult::ok()
}

/// Create a wallet from raw seed bytes (64 bytes).
///
/// On success, `out_wallet_handle` is set to a `PlatformWallet` handle and
/// `out_wallet_id` is filled with the 32-byte wallet ID.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_create_wallet_from_seed(
    manager_handle: Handle,
    network: FFINetwork,
    seed_bytes: *const u8,
    seed_len: usize,
    account_options: u32,
    out_wallet_handle: *mut Handle,
    out_wallet_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    create_wallet_from_seed_impl(
        manager_handle,
        network,
        seed_bytes,
        seed_len,
        account_options,
        None,
        out_wallet_handle,
        out_wallet_id,
    )
}

/// Create a wallet from raw seed bytes (64 bytes) with an optional
/// birth-height override.
///
/// Identical to [`platform_wallet_manager_create_wallet_from_seed`] but lets
/// the caller pin the wallet's birth height. `has_birth_height_override ==
/// false` behaves exactly like the no-override export (`None`);
/// `has_birth_height_override == true` passes `Some(birth_height_override)`,
/// including `Some(0)`.
///
/// On success, `out_wallet_handle` is set to a `PlatformWallet` handle and
/// `out_wallet_id` is filled with the 32-byte wallet ID.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_create_wallet_from_seed_with_birth_height(
    manager_handle: Handle,
    network: FFINetwork,
    seed_bytes: *const u8,
    seed_len: usize,
    account_options: u32,
    has_birth_height_override: bool,
    birth_height_override: u32,
    out_wallet_handle: *mut Handle,
    out_wallet_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    create_wallet_from_seed_impl(
        manager_handle,
        network,
        seed_bytes,
        seed_len,
        account_options,
        birth_height_override_opt(has_birth_height_override, birth_height_override),
        out_wallet_handle,
        out_wallet_id,
    )
}

/// Create a wallet from a BIP39 mnemonic phrase (English).
///
/// On success, `out_wallet_handle` is set to a `PlatformWallet` handle and
/// `out_wallet_id` is filled with the 32-byte wallet ID.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_create_wallet_from_mnemonic(
    manager_handle: Handle,
    mnemonic: *const std::os::raw::c_char,
    network: FFINetwork,
    account_options: u32,
    out_wallet_handle: *mut Handle,
    out_wallet_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    create_wallet_from_mnemonic_impl(
        manager_handle,
        mnemonic,
        network,
        account_options,
        None,
        out_wallet_handle,
        out_wallet_id,
    )
}

/// Create a wallet from a BIP39 mnemonic phrase (English) with an optional
/// birth-height override.
///
/// Identical to [`platform_wallet_manager_create_wallet_from_mnemonic`] but
/// lets the caller pin the wallet's birth height. `has_birth_height_override
/// == false` behaves exactly like the no-override export (`None`);
/// `has_birth_height_override == true` passes `Some(birth_height_override)`,
/// including `Some(0)`.
///
/// On success, `out_wallet_handle` is set to a `PlatformWallet` handle and
/// `out_wallet_id` is filled with the 32-byte wallet ID.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_create_wallet_from_mnemonic_with_birth_height(
    manager_handle: Handle,
    mnemonic: *const std::os::raw::c_char,
    network: FFINetwork,
    account_options: u32,
    has_birth_height_override: bool,
    birth_height_override: u32,
    out_wallet_handle: *mut Handle,
    out_wallet_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    create_wallet_from_mnemonic_impl(
        manager_handle,
        mnemonic,
        network,
        account_options,
        birth_height_override_opt(has_birth_height_override, birth_height_override),
        out_wallet_handle,
        out_wallet_id,
    )
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
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        runtime().block_on(manager.load_from_persistor())
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Get a `PlatformWallet` handle for a wallet registered in the
/// manager. Returns `NotFound` if no wallet with the given
/// id is currently held.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_get_wallet(
    manager_handle: Handle,
    wallet_id: *const [u8; 32],
    out_wallet_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_wallet_handle);
    let wallet_id_value = *wallet_id;

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        runtime().block_on(manager.get_wallet(&wallet_id_value))
    });
    let inner = unwrap_option_or_return!(option);
    match inner {
        Some(wallet) => {
            let handle = PLATFORM_WALLET_STORAGE.insert(wallet);
            *out_wallet_handle = handle;
            PlatformWalletFFIResult::ok()
        }
        None => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            format!(
                "Wallet {} not found in manager",
                hex::encode(wallet_id_value)
            ),
        ),
    }
}

/// Destroy a PlatformWalletManager handle.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_destroy(
    handle: Handle,
) -> PlatformWalletFFIResult {
    if let Some(manager) = PLATFORM_WALLET_MANAGER_STORAGE.remove(handle) {
        // Run the full lifecycle shutdown to completion, not just the
        // platform-address sync. Every background task (identity sync,
        // shielded sync, the wallet-event adapter) can fire callbacks
        // through the host-owned `context` pointer; once `destroy`
        // returns the host may free that context, so no task may be
        // left alive to fire a callback against freed memory.
        // `shutdown()` is idempotent, so this is safe even if the host
        // already stopped some sync managers before calling destroy.
        runtime().block_on(manager.shutdown());
    }
    PlatformWalletFFIResult::ok()
}

/// Remove one wallet from the manager. Idempotent on missing wallets.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_remove_wallet(
    manager_handle: Handle,
    wallet_id: *const [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    let wallet_id_value = *wallet_id;

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        runtime().block_on(manager.remove_wallet(&wallet_id_value))
    });
    let result = unwrap_option_or_return!(option);
    match result {
        Ok(_) => PlatformWalletFFIResult::ok(),
        // Idempotency: a wallet that's already gone is the success
        // state callers want. Everything else is a real failure.
        Err(platform_wallet::PlatformWalletError::WalletNotFound(_)) => {
            PlatformWalletFFIResult::ok()
        }
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!(
                "Failed to remove wallet {}: {}",
                hex::encode(wallet_id_value),
                e
            ),
        ),
    }
}

/// Upgrade an already-loaded external-signable wallet to a fully
/// seeded signing wallet **in place**, from a BIP-39 mnemonic.
///
/// The persisted-restore path (`load_from_persistor`) rehydrates every
/// wallet watch-only (per-account xpubs only, no key material), so any
/// signing operation — DashPay contact-xpub derivation, identity-key
/// signing — fails after an app relaunch with `External signable wallet
/// has no private key`. The host calls this once per wallet right after
/// `load_from_persistor`, passing the mnemonic it holds in its Keychain,
/// to make the wallet signing-capable again.
///
/// `mnemonic` is parsed against every supported BIP-39 wordlist;
/// `passphrase` may be null (treated as the empty passphrase). The
/// mnemonic → seed conversion happens here in Rust — Swift never derives
/// the seed (per the Swift-SDK FFI boundary rules). The derived seed is
/// held in a `Zeroizing` buffer for the duration of the call.
///
/// The library verifies the seed actually belongs to `wallet_id`
/// (network-scoped id recomputed from the seed must match) before
/// attaching it; a mismatched mnemonic is rejected without touching the
/// wallet. Re-deriving a wallet that is already seed-backed is a no-op
/// success.
///
/// Returns `NotFound` if no wallet with `wallet_id` is registered,
/// `ErrorInvalidParameter` for an unparseable mnemonic or a mismatched
/// seed, and `ErrorWalletOperation` for other upgrade failures.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_attach_wallet_seed_from_mnemonic(
    manager_handle: Handle,
    wallet_id: *const [u8; 32],
    mnemonic: *const std::os::raw::c_char,
    passphrase: *const std::os::raw::c_char,
) -> PlatformWalletFFIResult {
    use std::ffi::CStr;
    use zeroize::Zeroizing;

    check_ptr!(wallet_id);
    check_ptr!(mnemonic);
    let wallet_id_value = *wallet_id;

    let mnemonic_str = unwrap_result_or_return!(CStr::from_ptr(mnemonic).to_str());
    let passphrase_str: &str = if passphrase.is_null() {
        ""
    } else {
        unwrap_result_or_return!(CStr::from_ptr(passphrase).to_str())
    };

    // Mnemonic → seed in Rust. `parse_mnemonic_any_language` walks every
    // supported wordlist so non-English phrases aren't rejected as
    // "invalid English". The 64-byte seed is zeroized on drop.
    let parsed = unwrap_result_or_return!(parse_mnemonic_any_language(mnemonic_str));
    let seed: Zeroizing<[u8; 64]> = Zeroizing::new(parsed.to_seed(passphrase_str));

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        // `runtime().block_on`, matching the sibling
        // `create_wallet_from_seed_impl`: the upgrade only re-derives an
        // HD wallet from the seed (BIP32 master + the fixed-depth account
        // paths) — bounded, shallow recursion, not the deep GroveDB
        // proof-verification recursion that forces the
        // `block_on_worker` 8 MB-stack dispatch elsewhere (see
        // `dashpay_sync.rs`). The work borrows `manager` from the
        // `with_item` closure, which a `'static` worker spawn could not
        // capture anyway.
        // `&seed` is `&Zeroizing<[u8; 64]>`; it coerces to the
        // `&[u8; 64]` the method takes at this argument position.
        runtime().block_on(manager.attach_wallet_seed(wallet_id_value, &seed))
    });
    let result = unwrap_option_or_return!(option);
    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
        Err(e @ platform_wallet::PlatformWalletError::SeedMismatch { .. }) => {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e.to_string(),
            )
        }
        Err(platform_wallet::PlatformWalletError::WalletNotFound(_)) => {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::NotFound,
                format!(
                    "Wallet {} not found in manager",
                    hex::encode(wallet_id_value)
                ),
            )
        }
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!(
                "Failed to attach seed to wallet {}: {}",
                hex::encode(wallet_id_value),
                e
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn birth_height_override_opt_true_zero_is_some_zero() {
        assert_eq!(birth_height_override_opt(true, 0), Some(0));
    }

    #[test]
    fn birth_height_override_opt_true_value_is_some_value() {
        assert_eq!(birth_height_override_opt(true, 42), Some(42));
    }

    #[test]
    fn birth_height_override_opt_false_is_none_regardless_of_value() {
        assert_eq!(birth_height_override_opt(false, 0), None);
        assert_eq!(birth_height_override_opt(false, 99), None);
    }

    // --- attach_wallet_seed_from_mnemonic input-validation paths ---
    //
    // The happy-path upgrade semantics (external-signable → signing,
    // wallet-id safety gate, idempotency) are pinned by the library
    // tests in `platform_wallet::manager::attach_seed::tests`. These FFI
    // tests cover the marshalling boundary the library can't see: null
    // handle, null pointers, and an unparseable mnemonic must be
    // rejected before any manager lookup — matching the contract the
    // other manager exports uphold.

    use std::ffi::CString;

    /// An unknown handle must surface `NotFound` (via
    /// `unwrap_option_or_return!`) rather than dereferencing a stale
    /// slot — but only after the pointer + mnemonic checks pass, since
    /// those run first.
    #[test]
    fn attach_wallet_seed_unknown_handle_returns_not_found() {
        let bogus: Handle = 0xDEAD_BEEF;
        let wallet_id = [0u8; 32];
        let mnemonic = CString::new(
            "abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon abandon about",
        )
        .unwrap();
        let r = unsafe {
            platform_wallet_manager_attach_wallet_seed_from_mnemonic(
                bogus,
                &wallet_id,
                mnemonic.as_ptr(),
                std::ptr::null(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
    }

    /// A null `wallet_id` is rejected with `ErrorNullPointer` (the
    /// `check_ptr!` contract) before the handle is looked up.
    #[test]
    fn attach_wallet_seed_null_wallet_id_is_null_pointer() {
        let mnemonic = CString::new("abandon abandon about").unwrap();
        let r = unsafe {
            platform_wallet_manager_attach_wallet_seed_from_mnemonic(
                1,
                std::ptr::null(),
                mnemonic.as_ptr(),
                std::ptr::null(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// A null `mnemonic` is rejected with `ErrorNullPointer`.
    #[test]
    fn attach_wallet_seed_null_mnemonic_is_null_pointer() {
        let wallet_id = [7u8; 32];
        let r = unsafe {
            platform_wallet_manager_attach_wallet_seed_from_mnemonic(
                1,
                &wallet_id,
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// An unparseable mnemonic is rejected with `ErrorInvalidParameter`
    /// (mapped from `parse_mnemonic_any_language`'s error via
    /// `unwrap_result_or_return!`) before any manager lookup.
    #[test]
    fn attach_wallet_seed_bad_mnemonic_is_invalid_parameter() {
        let wallet_id = [7u8; 32];
        let mnemonic = CString::new("not a real bip39 mnemonic at all").unwrap();
        let r = unsafe {
            platform_wallet_manager_attach_wallet_seed_from_mnemonic(
                1,
                &wallet_id,
                mnemonic.as_ptr(),
                std::ptr::null(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
    }
}
