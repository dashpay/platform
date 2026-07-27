//! FFI bindings for PlatformWalletManager (wallet lifecycle management).

use crate::check_ptr;
use crate::error::*;
use crate::event_handler::{EventHandlerCallbacks, FFIEventHandler};
use crate::handle::*;
use crate::persistence::{FFIPersister, PersistenceCallbacks, PersistenceCapabilitiesFFI};
use crate::runtime::runtime;
use crate::types::{FFINetwork, Network};
use crate::{unwrap_option_or_return, unwrap_result_or_return};

use dash_sdk::Sdk;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use platform_wallet::PlatformWalletManager;
use platform_wallet::{PersistenceCapabilities, PERSISTENCE_CAPABILITIES_VERSION};
use std::os::raw::c_void;
use std::sync::Arc;

fn persistence_capabilities_declaration(
    value: &PersistenceCapabilitiesFFI,
) -> PersistenceCapabilities {
    if value.version == PERSISTENCE_CAPABILITIES_VERSION {
        PersistenceCapabilities::from_bits_retain(value.bits)
    } else {
        PersistenceCapabilities::NONE
    }
}

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
    platform_wallet_manager_create_impl(
        sdk_ptr,
        persistence,
        event_handler,
        PersistenceCapabilities::NONE,
        out_handle,
    )
}

/// Create a manager with an explicit, versioned persistence capability
/// declaration. This additive entry point preserves the exact layout of the
/// established [`PersistenceCallbacks`] vtable used by
/// [`platform_wallet_manager_create`]. Unknown versions fail closed to no
/// capabilities; known bits are still intersected with callback structure.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_create_with_persistence_capabilities(
    sdk_ptr: *const c_void,
    persistence: *const PersistenceCallbacks,
    event_handler: *const EventHandlerCallbacks,
    persistence_capabilities: *const PersistenceCapabilitiesFFI,
    out_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(persistence_capabilities);
    let declaration = persistence_capabilities_declaration(&*persistence_capabilities);
    platform_wallet_manager_create_impl(
        sdk_ptr,
        persistence,
        event_handler,
        declaration,
        out_handle,
    )
}

unsafe fn platform_wallet_manager_create_impl(
    sdk_ptr: *const c_void,
    persistence: *const PersistenceCallbacks,
    event_handler: *const EventHandlerCallbacks,
    declared_capabilities: PersistenceCapabilities,
    out_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(sdk_ptr);
    check_ptr!(persistence);
    check_ptr!(event_handler);
    check_ptr!(out_handle);

    let sdk = Arc::new((*(sdk_ptr as *const Sdk)).clone());
    let persister = Arc::new(FFIPersister::new_with_persistence_capabilities(
        std::ptr::read(persistence),
        declared_capabilities,
    ));
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

/// Query the versioned persistence capabilities of a live manager.
///
/// The returned mask is derived from the callback vtable copied at manager
/// creation. Callers must check `version` before interpreting known bits and
/// ignore unknown bits for forward compatibility.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_persistence_capabilities(
    manager_handle: Handle,
    out_capabilities: *mut PersistenceCapabilitiesFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_capabilities);

    let capabilities = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |manager| manager.persistence_capabilities());
    let capabilities = unwrap_option_or_return!(capabilities);
    *out_capabilities = capabilities.into();
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

    // Zeroize the FFI-boundary copy of the master secret on drop. Passed by
    // reference so the manager method doesn't take an un-zeroized owned copy.
    let mut seed = zeroize::Zeroizing::new([0u8; 64]);
    std::ptr::copy_nonoverlapping(seed_bytes, seed.as_mut_ptr(), 64);

    let accounts = match account_options {
        0 => WalletAccountCreationOptions::None,
        1 => WalletAccountCreationOptions::Default,
        _ => WalletAccountCreationOptions::Default,
    };

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        runtime().block_on(manager.create_wallet_from_seed_bytes(
            network,
            &seed,
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

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn begin_changeset(_context: *mut c_void, _wallet_id: *const u8) -> i32 {
        0
    }

    unsafe extern "C" fn end_changeset(
        _context: *mut c_void,
        _wallet_id: *const u8,
        _success: bool,
    ) -> i32 {
        0
    }

    fn persistence_callbacks() -> PersistenceCallbacks {
        PersistenceCallbacks {
            on_changeset_begin_fn: Some(begin_changeset),
            on_changeset_end_fn: Some(end_changeset),
            ..Default::default()
        }
    }

    fn event_callbacks() -> EventHandlerCallbacks {
        EventHandlerCallbacks {
            context: std::ptr::null_mut(),
            on_wallet_event_fn: None,
            on_error_fn: None,
            on_platform_address_sync_completed_fn: None,
            on_shielded_sync_completed_fn: None,
            on_shielded_sync_progress_fn: None,
            on_shielded_tree_progress_fn: None,
        }
    }

    fn query(handle: Handle) -> PersistenceCapabilitiesFFI {
        let mut out = PersistenceCapabilitiesFFI {
            version: 0,
            reserved: u32::MAX,
            bits: u64::MAX,
        };
        let result = unsafe { platform_wallet_manager_persistence_capabilities(handle, &mut out) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        out
    }

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

    #[test]
    fn unknown_capability_declaration_version_fails_closed() {
        let declaration = PersistenceCapabilitiesFFI {
            version: PERSISTENCE_CAPABILITIES_VERSION + 1,
            reserved: 0,
            bits: u64::MAX,
        };
        assert_eq!(
            persistence_capabilities_declaration(&declaration),
            PersistenceCapabilities::NONE
        );
    }

    #[test]
    fn legacy_create_is_abi_stable_and_fail_closed() {
        let sdk = dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk");
        let callbacks = persistence_callbacks();
        let event_callbacks = event_callbacks();
        let mut handle = 0;
        let result = unsafe {
            platform_wallet_manager_create(
                &sdk as *const Sdk as *const c_void,
                &callbacks,
                &event_callbacks,
                &mut handle,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        let out = query(handle);
        assert_eq!(out.version, PERSISTENCE_CAPABILITIES_VERSION);
        assert_eq!(out.reserved, 0);
        assert_eq!(out.bits, 0);

        let result = unsafe { platform_wallet_manager_destroy(handle) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
    }

    #[test]
    fn additive_create_versions_and_intersects_capabilities() {
        let sdk = dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk");
        let callbacks = persistence_callbacks();
        let event_callbacks = event_callbacks();
        let declaration = PersistenceCapabilitiesFFI {
            version: PERSISTENCE_CAPABILITIES_VERSION,
            reserved: 0,
            bits: PersistenceCapabilities::ATOMIC_CHANGESETS
                .union(PersistenceCapabilities::INVITATIONS)
                .bits(),
        };
        let mut handle = 0;
        let result = unsafe {
            platform_wallet_manager_create_with_persistence_capabilities(
                &sdk as *const Sdk as *const c_void,
                &callbacks,
                &event_callbacks,
                &declaration,
                &mut handle,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        let out = query(handle);
        assert_eq!(out.version, PERSISTENCE_CAPABILITIES_VERSION);
        assert_eq!(out.reserved, 0);
        // Invitations were declared but lack their required callback, proving
        // the manager query returns the declaration/structure intersection.
        assert_eq!(out.bits, PersistenceCapabilities::ATOMIC_CHANGESETS.bits());

        let result = unsafe { platform_wallet_manager_destroy(handle) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
    }
}
