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
/// `persistence` and `event_handler` are callback vtables. A vtable that
/// carries a non-null `context` MUST also set `release_fn` — creation
/// fails with `ErrorInvalidParameter` otherwise. Ownership of the context
/// then transfers to Rust: the manager keeps it alive for exactly as long
/// as any internal worker can still invoke a callback, and calls
/// `release_fn` once — possibly on a background thread, possibly *after*
/// a later `destroy` returns if a worker straggles — when the last
/// reference drops. A borrowed (non-null context, null `release_fn`)
/// vtable is rejected rather than accepted-and-hoped-for, because nothing
/// but ownership can make a straggling worker safe: `destroy` returns
/// `Success` without proving quiescence, so a borrowed context would be
/// freeable by the host while a straggler can still call through it. A
/// host whose context needs no cleanup passes a no-op `release_fn`.
///
/// On a non-`Success` return Rust has NOT taken ownership of either
/// context; a host that pre-retained them must release them itself.
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

    // Ownership is mandatory for a context-carrying vtable: `destroy`
    // returns `Success` without proving every worker joined, which is only
    // sound because a straggler's `Arc` keeps the host context alive via
    // `release_fn`. A borrowed context (non-null, no destructor) would
    // reintroduce the freed-context callback exactly on the non-clean
    // path, so it is rejected up front instead of accepted unsafely.
    if !(*persistence).context.is_null() && (*persistence).release_fn.is_none() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "persistence callbacks carry a context but no release_fn; the manager owns \
             callback contexts (released when its last worker drops) and cannot accept \
             a borrowed context — pass a release_fn (a no-op one if the context needs \
             no cleanup)"
                .to_string(),
        );
    }
    if !(*event_handler).context.is_null() && (*event_handler).release_fn.is_none() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "event-handler callbacks carry a context but no release_fn; the manager owns \
             callback contexts (released when its last worker drops) and cannot accept \
             a borrowed context — pass a release_fn (a no-op one if the context needs \
             no cleanup)"
                .to_string(),
        );
    }

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

/// Whether the manager has frozen its durable sync watermark this session
/// (dashpay/platform#4069).
///
/// `true` means a persistence `store()` was rejected — the one remaining
/// fault trigger; the lossless persistence channel cannot drop or lag
/// events — so the affected wallet's persisted
/// `syncedHeight` is deliberately held behind the chain tip and a rescan is
/// pending on the next launch. Hosts poll this to surface a hard
/// "verification failed / rescan pending" state instead of the fault being
/// visible only in error logs.
///
/// The flag latches: once `true` it stays `true` for this manager
/// instance's lifetime (a destroyed-and-recreated manager starts unlatched).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_sync_fault_detected(
    handle: Handle,
    out_detected: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_detected);

    let option =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| manager.sync_fault_detected());
    *out_detected = unwrap_option_or_return!(option);
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
///
/// Runs the full lifecycle shutdown (bounded: quiesce + join every
/// coordinator, SPV, the payment-hook tasks, and the event adapter) and
/// removes the handle. Always returns `Success` for a live handle.
///
/// A non-clean shutdown — a worker that outlived its join budget — is
/// logged, **not** surfaced as an error, because it is no longer a
/// safety problem the host could act on: a straggling worker holds a
/// strong reference to the callback vtables, and creation guarantees
/// every context-carrying vtable is owned (`release_fn` required), so
/// the host objects stay alive until that worker exits, at which point
/// Rust releases them. Nothing dangles, nothing needs a retry, nothing
/// needs a deliberate leak on the host side.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_destroy(
    handle: Handle,
) -> PlatformWalletFFIResult {
    if let Some(manager) = PLATFORM_WALLET_MANAGER_STORAGE.remove(handle) {
        // Run the full lifecycle shutdown to completion, not just the
        // platform-address sync. `shutdown()` is idempotent, so this is
        // safe even if the host already stopped some sync managers
        // before calling destroy.
        let report = runtime().block_on(manager.shutdown());
        if !report.all_clean() {
            // A worker panicked, exceeded its join budget, or stayed
            // detached. Its persister/event-handler Arcs keep the host
            // callback contexts alive until it actually exits, so this
            // is diagnostic, not a UAF hazard.
            tracing::warn!(
                ?report,
                "platform wallet manager shutdown did not join every worker \
                 cleanly; stragglers keep their callback contexts alive and \
                 release them on exit"
            );
        }
        // Dropping the manager here releases its persister/event-handler
        // references; the host contexts are released (via `release_fn`)
        // as soon as the last worker's reference drops — typically right
        // now, or later if a straggler is still draining.
    }
    PlatformWalletFFIResult::ok()
}

/// Remove one wallet from the manager, tearing down its generation's deferred
/// state in the same linearization step.
///
/// Generic over the persister so tests can drive the exact production sequence
/// with the in-crate test fixture (the FFI handle storage is pinned to
/// [`FFIPersister`](crate::persistence::FFIPersister)). The ordering here is the
/// invariant under test — see the `remove_wallet_lifecycle_tests` module.
pub(crate) async fn remove_wallet_and_tear_down_generation<
    P: platform_wallet::changeset::PlatformWalletPersistence + 'static,
>(
    manager: &platform_wallet::PlatformWalletManager<P>,
    wallet_id: &[u8; 32],
) -> Result<(), platform_wallet::PlatformWalletError> {
    // The removal and the sweep below are ONE linearization point, taken under
    // the REMOVED GENERATION'S OWN lifecycle gate. `remove_wallet_with_teardown`
    // owns that gate, so the ordering cannot be got wrong here — or by any other
    // caller, including a direct Rust embedder that never goes through this FFI.
    //
    // What the single step buys (`dashpay/platform#4185`): the removal's own
    // `.await`s (shielded-coordinator and identity-sync unregistration) used to
    // sit in a gap between the removal and the sweep, and a concurrent
    // `core_wallet_signed_payment_broadcast` on a retained handle slipped through
    // it — its entry was still registered, `is_same_generation` passes (a removed
    // generation matches itself), the age guard is skipped (`last_processed_height`
    // is `None` once the wallet is gone, which the guard maps to "not expired"),
    // and it reached the broadcaster, pushing a removed wallet's payment onto the
    // network.
    //
    // Acquiring the gate waits for this generation's payment operations that have
    // entered their liveness-check/publish section. It does NOT wait for one still
    // awaiting an external signer: those take the gate only after the signature
    // returns (see `core_wallet_signed_payment_finalize` and
    // `core_wallet_tx_builder_finalize`), deliberately, so an open signing prompt
    // cannot stall teardown. Such a late finalizer instead observes the removed
    // generation at its own liveness check and abandons rather than publishing.
    manager
        .remove_wallet_with_teardown(wallet_id, |removed| {
            // The wallet and its accounts' `ReservationSet`s are now gone from
            // the manager, so the deferred-payment reservations cease to exist —
            // there is nothing to reconcile. DROP (do not release) this
            // generation's registry tokens and its finalized-tx V2 handles. This
            // is the teardown half of the single generation policy both deferred
            // paths share: it makes any stale handle to the removed generation
            // inert, so a later destroy/release of a lingering handle can never
            // release-by-outpoint against a re-created generation's inputs.
            let core = removed.core();
            crate::core_wallet::signed_payment::SIGNED_PAYMENT_REGISTRY
                .remove_entries_for_wallet(core);
            crate::handle::CORE_SIGNED_TRANSACTION_V2_STORAGE
                .remove_matching(|tx| tx.wallet.is_same_generation(core));
        })
        .await?;
    Ok(())
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
        runtime().block_on(remove_wallet_and_tear_down_generation(
            manager,
            &wallet_id_value,
        ))
    });
    let result = unwrap_option_or_return!(option);
    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
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
            release_fn: None,
        }
    }

    /// Counts invocations through a `*mut AtomicUsize` context — stands in
    /// for the host's release trampoline.
    unsafe extern "C" fn counting_release(context: *mut c_void) {
        if let Some(counter) = (context as *const std::sync::atomic::AtomicUsize).as_ref() {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// A context-carrying vtable without a `release_fn` must be rejected
    /// at creation. `destroy` returns `Success` without proving every
    /// worker joined; that is only sound because a straggler's `Arc`
    /// keeps the host context alive via ownership. Accepting a borrowed
    /// context would let a legacy caller free it after a "successful"
    /// destroy while a straggler can still call through it — the exact
    /// use-after-free this FFI exists to prevent.
    #[test]
    fn create_rejects_context_carrying_vtable_without_release_fn() {
        let sdk = dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk");
        let sentinel = 0xC0FFEEusize as *mut c_void;

        // Persistence vtable with a context but no destructor.
        let mut callbacks = persistence_callbacks();
        callbacks.context = sentinel;
        let event_cbs = event_callbacks();
        let mut handle = 0;
        let result = unsafe {
            platform_wallet_manager_create(
                &sdk as *const Sdk as *const c_void,
                &callbacks,
                &event_cbs,
                &mut handle,
            )
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "borrowed persistence context must be rejected"
        );

        // Event vtable with a context but no destructor.
        let callbacks = persistence_callbacks();
        let mut event_cbs = event_callbacks();
        event_cbs.context = sentinel;
        let result = unsafe {
            platform_wallet_manager_create(
                &sdk as *const Sdk as *const c_void,
                &callbacks,
                &event_cbs,
                &mut handle,
            )
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "borrowed event context must be rejected"
        );

        // Null contexts stay valid without a destructor (the
        // `configure(modelContainer: nil)` shape).
        let callbacks = persistence_callbacks();
        let event_cbs = event_callbacks();
        let result = unsafe {
            platform_wallet_manager_create(
                &sdk as *const Sdk as *const c_void,
                &callbacks,
                &event_cbs,
                &mut handle,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        let result = unsafe { platform_wallet_manager_destroy(handle) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
    }

    /// The owned-context contract end to end through the public FFI: when
    /// both vtables set `release_fn`, destroying the manager releases each
    /// context exactly once — after shutdown has joined every worker, so
    /// the release IS the proof that nothing can call back into the host
    /// anymore. This is the contract that lets Swift `passRetained` /
    /// JNI box-transfer their callback objects instead of leaking them
    /// whenever teardown is not provably clean.
    #[test]
    fn destroy_releases_owned_callback_contexts_exactly_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let persistence_releases = AtomicUsize::new(0);
        let event_releases = AtomicUsize::new(0);

        let sdk = dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk");
        let mut callbacks = persistence_callbacks();
        callbacks.context = &persistence_releases as *const AtomicUsize as *mut c_void;
        callbacks.release_fn = Some(counting_release);
        let mut event_cbs = event_callbacks();
        event_cbs.context = &event_releases as *const AtomicUsize as *mut c_void;
        event_cbs.release_fn = Some(counting_release);

        let mut handle = 0;
        let result = unsafe {
            platform_wallet_manager_create(
                &sdk as *const Sdk as *const c_void,
                &callbacks,
                &event_cbs,
                &mut handle,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(
            persistence_releases.load(Ordering::SeqCst),
            0,
            "context must stay alive while the manager lives"
        );
        assert_eq!(event_releases.load(Ordering::SeqCst), 0);

        let result = unsafe { platform_wallet_manager_destroy(handle) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        assert_eq!(
            persistence_releases.load(Ordering::SeqCst),
            1,
            "destroy must release the persistence context exactly once"
        );
        assert_eq!(
            event_releases.load(Ordering::SeqCst),
            1,
            "destroy must release the event context exactly once"
        );

        // Destroying a stale handle must not double-release.
        let result = unsafe { platform_wallet_manager_destroy(handle) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(persistence_releases.load(Ordering::SeqCst), 1);
        assert_eq!(event_releases.load(Ordering::SeqCst), 1);
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

/// Wallet-generation teardown vs. the deferred-payment registry
/// (`dashpay/platform#4185`).
///
/// The invariant every test here defends is one sentence: **no deferred-payment
/// token for a wallet that is not currently registered in the manager is ever
/// actionable.** Removal and the registry sweep used to be two independent steps
/// with the removal's own `.await`s in the gap, and `register` could land after
/// the sweep, so the invariant held only by timing.
///
/// These drive [`remove_wallet_and_tear_down_generation`] — the exact sequence
/// `platform_wallet_manager_remove_wallet` runs — rather than the `extern "C"`
/// wrapper, because the FFI handle storage is pinned to `FFIPersister` while the
/// wallet fixture uses the in-crate test persister. The wrapper adds only handle
/// resolution and error-code mapping on top.
#[cfg(test)]
mod remove_wallet_lifecycle_tests {
    use super::*;
    use crate::core_wallet::signed_payment::SIGNED_PAYMENT_REGISTRY;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
    use platform_wallet::test_support::test_platform_wallet_manager;
    use platform_wallet::{
        CoreWallet, ReservationToken, SignedCoreTransaction, SignedPaymentError,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    fn dummy_tx() -> dashcore::Transaction {
        dashcore::Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        }
    }

    /// Mint a token against `core`. The dummy tx reserved nothing (height 0, no
    /// funding token), so these tests exercise the lifecycle guards rather than
    /// the age or owner guard.
    fn register_token(
        core: &CoreWallet<platform_wallet::broadcaster::SpvBroadcaster>,
    ) -> ReservationToken {
        SIGNED_PAYMENT_REGISTRY
            .register(
                core.clone(),
                SignedCoreTransaction::new_for_test(
                    dummy_tx(),
                    0,
                    AccountTypePreference::BIP44,
                    0,
                    0,
                    None,
                    core.test_generation_marker(),
                ),
            )
            .expect("register binds to the finalizing generation")
    }

    /// A token is dead iff broadcasting it reports it as unknown/consumed. Token-
    /// scoped on purpose: the registry is a process-global shared with every
    /// other test in the binary, so `outstanding()` deltas are not reliable under
    /// the default parallel test harness.
    async fn assert_token_is_gone(
        token: ReservationToken,
        core: &CoreWallet<platform_wallet::broadcaster::SpvBroadcaster>,
    ) {
        match SIGNED_PAYMENT_REGISTRY.broadcast(token, core).await {
            Err(SignedPaymentError::StaleToken(t)) if t == token => {}
            other => panic!("token {token} should have been swept, got {other:?}"),
        }
    }

    /// Requirement: a broadcast must FAIL CLEANLY when the wallet is no longer in
    /// the manager, rather than silently proceeding to the network.
    ///
    /// The setup reproduces the in-flight-finalizer resurrection directly:
    /// register AFTER teardown has already swept, which is exactly what
    /// `core_wallet_signed_payment_finalize` used to do when the host removed the
    /// wallet during the signer await. Before the fix this token was fully
    /// actionable — `is_same_generation` passes (a removed generation matches
    /// itself), `last_processed_height` is `None` so the age guard is skipped,
    /// and `broadcast_payment_releasing_reservation` has no wallet-existence gate
    /// — so the payment went to the broadcaster.
    #[test]
    fn broadcasting_a_token_for_a_removed_wallet_is_refused_before_the_network() {
        // Shares the process-global registry with `wallet::destroy_tests`,
        // which asserts on `outstanding()` counts — serialize against it.
        let _registry = crate::core_wallet::signed_payment::registry_test_guard();

        runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;
            let wallet = manager
                .get_wallet(&wallet_id)
                .await
                .expect("wallet present");
            let core = wallet.core().clone();

            remove_wallet_and_tear_down_generation(&manager, &wallet_id)
                .await
                .expect("remove succeeds");
            assert!(
                !core.is_current_generation().await,
                "the retained handle must observe its generation as gone"
            );

            let token = register_token(&core);

            match SIGNED_PAYMENT_REGISTRY.broadcast(token, &core).await {
                Err(SignedPaymentError::WalletRemoved(t)) if t == token => {}
                other => panic!(
                    "a token whose wallet was removed must be refused without a send, got {other:?}"
                ),
            }

            // Refusing still CONSUMES the token: the generation is gone, so there
            // is nothing to reconcile and nothing to retry.
            assert_token_is_gone(token, &core).await;
        });
    }

    /// Requirement: removal and the registry sweep are linearized with respect to
    /// broadcast. Run repeatedly to shake the interleaving.
    ///
    /// Both orderings are legal, so the assertion cannot simply be "the broadcast
    /// is refused":
    ///
    ///  * teardown first → the entry is swept and the broadcast is refused
    ///    (`StaleToken`), or the wallet is gone and it is refused
    ///    (`WalletRemoved`);
    ///  * broadcast first → it holds the shared gate, the wallet is genuinely
    ///    still live, the payment legitimately goes to the broadcaster, and the
    ///    teardown waits.
    ///
    /// What must be impossible is the combination the pre-fix gap allowed:
    /// reaching the broadcaster even though teardown had ALREADY completed. That
    /// is what the completion-order tickets pin down. Because the gate serializes
    /// the two, a broadcast that reached the broadcaster must have been holding
    /// the gate, so the teardown cannot have finished before it — i.e. the
    /// sender's ticket must precede the remover's. Without the gate the remover
    /// could finish first and the send still go out, which is exactly the
    /// `dashpay/platform#4185` finding.
    #[test]
    fn a_broadcast_never_reaches_the_broadcaster_after_teardown_completed() {
        // Shares the process-global registry with `wallet::destroy_tests`,
        // which asserts on `outstanding()` counts — serialize against it.
        let _registry = crate::core_wallet::signed_payment::registry_test_guard();

        for iteration in 0..25 {
            runtime().block_on(async {
                let (manager, wallet_id) = test_platform_wallet_manager().await;
                let wallet = manager
                    .get_wallet(&wallet_id)
                    .await
                    .expect("wallet present");
                let core = wallet.core().clone();
                let token = register_token(&core);

                let barrier = Arc::new(tokio::sync::Barrier::new(2));
                // Monotonic tickets stamped the instant each operation returns,
                // giving a total order over the two completions.
                let ticket = Arc::new(AtomicUsize::new(0));

                let remover = {
                    let barrier = Arc::clone(&barrier);
                    let manager = Arc::clone(&manager);
                    let ticket = Arc::clone(&ticket);
                    tokio::spawn(async move {
                        barrier.wait().await;
                        let outcome =
                            remove_wallet_and_tear_down_generation(&manager, &wallet_id).await;
                        (outcome, ticket.fetch_add(1, Ordering::SeqCst))
                    })
                };
                let sender = {
                    let barrier = Arc::clone(&barrier);
                    let core = core.clone();
                    let ticket = Arc::clone(&ticket);
                    tokio::spawn(async move {
                        barrier.wait().await;
                        let outcome = SIGNED_PAYMENT_REGISTRY.broadcast(token, &core).await;
                        (outcome, ticket.fetch_add(1, Ordering::SeqCst))
                    })
                };

                let (removed, remover_ticket) = remover.await.expect("remover task");
                removed.expect("remove succeeds");
                let (sent, sender_ticket) = sender.await.expect("sender task");

                // `Ok` is unreachable in-test (the fixture's SPV client is not
                // started, so the broadcaster errors), but it is the same class
                // of outcome: the payment was handed to the network layer.
                let reached_broadcaster =
                    matches!(sent, Ok(_) | Err(SignedPaymentError::Broadcast(_)));
                if reached_broadcaster {
                    assert!(
                        sender_ticket < remover_ticket,
                        "iteration {iteration}: a payment reached the broadcaster even though \
                         wallet teardown had already completed — removal is not linearized with \
                         broadcast (got {sent:?})"
                    );
                } else {
                    // The only other legal outcomes are the two clean refusals.
                    assert!(
                        matches!(
                            sent,
                            Err(SignedPaymentError::StaleToken(_))
                                | Err(SignedPaymentError::WalletRemoved(_))
                        ),
                        "iteration {iteration}: unexpected outcome {sent:?}"
                    );
                }

                // Whichever way it went, nothing survives teardown.
                assert!(!core.is_current_generation().await);
                assert_token_is_gone(token, &core).await;
            });
        }
    }

    /// Requirement: teardown WAITS for an in-flight finalizer, so a late
    /// `register` cannot resurrect a token for a removed generation.
    ///
    /// Deterministic. The held shared guard stands in for
    /// `core_wallet_signed_payment_finalize` sitting between its liveness check
    /// and its synchronous `register`. Before the fix nothing connected those two
    /// operations: the teardown ran to completion — sweep included — while the
    /// finalizer was signing, and the token it then inserted was permanently
    /// outside any sweep.
    #[test]
    fn teardown_waits_for_an_in_flight_finalizer_and_then_sweeps_its_token() {
        // Shares the process-global registry with `wallet::destroy_tests`,
        // which asserts on `outstanding()` counts — serialize against it.
        let _registry = crate::core_wallet::signed_payment::registry_test_guard();

        runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;
            let wallet = manager
                .get_wallet(&wallet_id)
                .await
                .expect("wallet present");
            let core = wallet.core().clone();

            // The finalizer enters THIS generation's gate (as the FFI does after
            // signing).
            let in_flight = core.generation_payment_guard().await;

            let teardown = {
                let manager = Arc::clone(&manager);
                tokio::spawn(async move {
                    remove_wallet_and_tear_down_generation(&manager, &wallet_id).await
                })
            };

            // Teardown must block on the exclusive side of the gate.
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert!(
                !teardown.is_finished(),
                "teardown must wait for the in-flight finalizer to leave the gate"
            );

            // Because teardown is still waiting, the finalizer's liveness check
            // sees a live wallet and its register is legitimate.
            assert!(
                core.is_current_generation().await,
                "the wallet must still be live while a finalizer holds the gate"
            );
            let token = register_token(&core);

            drop(in_flight);
            teardown
                .await
                .expect("teardown task")
                .expect("remove succeeds");

            // The teardown that was waiting sweeps the token the finalizer
            // inserted — the invariant the gate exists to restore.
            assert_token_is_gone(token, &core).await;
        });
    }

    // ---------------------------------------------------------------------
    // V2 finalized-transaction-handle path (`dashpay/platform#4185` review).
    //
    // The registry-token path above was gated first; the V2 path
    // (`core_wallet_tx_builder_finalize` → `CORE_SIGNED_TRANSACTION_V2_STORAGE`
    // → `core_wallet_broadcast_signed_transaction_v2`) reaches the SAME
    // broadcaster through a retained handle and was left ungated. Its
    // `is_same_generation` check compares two HANDLES, and a removed generation
    // matches itself, so two retained handles pushed a deleted wallet's
    // transaction onto the network.
    // ---------------------------------------------------------------------

    /// Publish a V2 finalized-transaction handle for `core`'s generation, the
    /// way `core_wallet_tx_builder_finalize` does.
    fn publish_v2_handle(
        core: &CoreWallet<platform_wallet::broadcaster::SpvBroadcaster>,
    ) -> Handle {
        crate::handle::CORE_SIGNED_TRANSACTION_V2_STORAGE.insert(
            crate::core_wallet::FFICoreSignedTransactionV2 {
                wallet: core.clone(),
                transaction: SignedCoreTransaction::new_for_test(
                    dummy_tx(),
                    0,
                    AccountTypePreference::BIP44,
                    0,
                    0,
                    None,
                    core.test_generation_marker(),
                ),
            },
        )
    }

    /// Requirement: a V2 handle whose wallet generation was removed must be
    /// refused BEFORE the network, exactly as the registry-token path is.
    ///
    /// Deterministic. The setup reproduces the late-finalizer publication
    /// directly: publish the handle AFTER teardown has already swept, which is
    /// what an ungated `core_wallet_tx_builder_finalize` does when the host
    /// removes the wallet during the signer await.
    ///
    /// Before the fix this handle was fully actionable — the caller handle and
    /// the embedded originating handle name the same removed generation, so
    /// `is_same_generation` passes, and `broadcast_finalized_transaction` goes
    /// straight to the broadcaster with no manager lookup at all — so the
    /// transaction went to the network.
    #[test]
    fn broadcasting_a_v2_handle_for_a_removed_wallet_is_refused_before_the_network() {
        let _registry = crate::core_wallet::signed_payment::registry_test_guard();

        // The extern "C" entry points call `runtime().block_on` themselves, so
        // they must be invoked from OUTSIDE a runtime context — do the async
        // setup first, then call across the boundary.
        let core = runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;
            let wallet = manager
                .get_wallet(&wallet_id)
                .await
                .expect("wallet present");
            let core = wallet.core().clone();

            remove_wallet_and_tear_down_generation(&manager, &wallet_id)
                .await
                .expect("remove succeeds");
            assert!(
                !core.is_current_generation().await,
                "the retained handle must observe its generation as gone"
            );
            core
        });

        let transaction_handle = publish_v2_handle(&core);
        let core_handle = crate::handle::CORE_WALLET_STORAGE.insert(core.clone());

        let mut out_txid: *mut std::os::raw::c_char = std::ptr::null_mut();
        let result = unsafe {
            crate::core_wallet::core_wallet_broadcast_signed_transaction_v2(
                core_handle,
                transaction_handle,
                &mut out_txid,
            )
        };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::NotFound,
            "a V2 handle whose wallet was removed must be refused without a send"
        );
        assert!(
            out_txid.is_null(),
            "no txid may be produced for a refused broadcast"
        );

        // Refusing still CONSUMES the handle: the generation is gone, so there is
        // nothing to reconcile and nothing to retry.
        assert!(
            crate::handle::CORE_SIGNED_TRANSACTION_V2_STORAGE
                .remove(transaction_handle)
                .is_none(),
            "the refused V2 handle must have been consumed"
        );
    }

    /// Requirement: a teardown WAITS for an in-flight V2 operation on that
    /// generation, then sweeps its handle — so a V2 handle can never be published
    /// into an already-swept storage and outlive its wallet.
    ///
    /// Deterministic. The held shared guard stands in for
    /// `core_wallet_tx_builder_finalize` sitting between its liveness check and
    /// its insert, or `core_wallet_broadcast_signed_transaction_v2` sitting
    /// between its liveness check and the send. Before the fix the V2 path took no
    /// gate at all: the teardown ran to completion — sweep included — while the
    /// finalizer was signing, and the handle it then published was permanently
    /// outside any sweep and fully broadcastable.
    #[test]
    fn teardown_waits_for_an_in_flight_v2_operation_and_then_sweeps_its_handle() {
        let _registry = crate::core_wallet::signed_payment::registry_test_guard();

        let (core, transaction_handle) = runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;
            let wallet = manager
                .get_wallet(&wallet_id)
                .await
                .expect("wallet present");
            let core = wallet.core().clone();

            // The V2 operation enters this generation's gate (as the FFI does
            // after signing).
            let in_flight = core.generation_payment_guard().await;

            let teardown = {
                let manager = Arc::clone(&manager);
                tokio::spawn(async move {
                    remove_wallet_and_tear_down_generation(&manager, &wallet_id).await
                })
            };

            // Teardown must block on the exclusive side of the gate.
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert!(
                !teardown.is_finished(),
                "teardown must wait for the in-flight V2 operation to leave the gate"
            );

            // Because teardown is still waiting, the operation's liveness check
            // sees a live wallet and its publish is legitimate.
            assert!(
                core.is_current_generation().await,
                "the wallet must still be live while a V2 operation holds the gate"
            );
            let transaction_handle = publish_v2_handle(&core);

            drop(in_flight);
            teardown
                .await
                .expect("teardown task")
                .expect("remove succeeds");

            assert!(!core.is_current_generation().await);
            (core, transaction_handle)
        });

        // The teardown that was waiting swept the handle the V2 operation
        // published — the invariant the gate exists to restore.
        assert!(
            crate::handle::CORE_SIGNED_TRANSACTION_V2_STORAGE
                .remove(transaction_handle)
                .is_none(),
            "the in-flight V2 operation's handle must have been swept by teardown"
        );
        drop(core);
    }

    /// Requirement: the lifecycle gate belongs to shared wallet-generation state,
    /// so removal driven through the PUBLIC Rust API is excluded too — not just
    /// removal driven through this crate's FFI wrapper.
    ///
    /// Deterministic. The held shared guard stands in for any payment operation
    /// sitting between its liveness check and the action that check authorizes (a
    /// register, or a send). `PlatformWalletManager` is public and
    /// `SignedPaymentRegistry` is re-exported from `platform-wallet`, so a direct
    /// Rust embedder reaches `remove_wallet` without ever touching
    /// [`remove_wallet_and_tear_down_generation`]. While the gate lived on the
    /// FFI's process-global registry singleton, that path took the write side
    /// nowhere at all: removal ran straight through, and the payment then acted on
    /// a generation the manager had already dropped.
    #[test]
    fn public_remove_wallet_waits_for_an_in_flight_payment_on_that_generation() {
        let _registry = crate::core_wallet::signed_payment::registry_test_guard();

        runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;
            let wallet = manager
                .get_wallet(&wallet_id)
                .await
                .expect("wallet present");
            let core = wallet.core().clone();

            // A payment operation on this generation is in flight.
            let in_flight = core.generation_payment_guard().await;

            let remover = {
                let manager = Arc::clone(&manager);
                tokio::spawn(async move { manager.remove_wallet(&wallet_id).await.map(|_| ()) })
            };

            tokio::time::sleep(Duration::from_millis(200)).await;
            assert!(
                !remover.is_finished(),
                "PlatformWalletManager::remove_wallet must wait for an in-flight payment on the \
                 generation it is removing — the public removal path takes no lifecycle exclusion"
            );

            // Because the removal is still waiting, the in-flight payment's
            // liveness check sees a live wallet and its action is legitimate.
            assert!(
                core.is_current_generation().await,
                "the wallet must still be live while a payment holds its generation gate"
            );

            drop(in_flight);
            remover
                .await
                .expect("remover task")
                .expect("remove succeeds");
            assert!(!core.is_current_generation().await);
        });
    }

    /// Requirement: the gate is scoped to ONE generation, so a slow payment on one
    /// wallet cannot stall an unrelated wallet's teardown.
    ///
    /// Deterministic, and the reason the gate could not stay on the FFI's
    /// process-global `SIGNED_PAYMENT_REGISTRY` singleton. A deferred broadcast
    /// holds the shared side across an SPV send — up to the broadcaster's timeout
    /// — and tokio's `RwLock` is write-preferring, so on one global lock that send
    /// blocked teardown, and every payment operation queued behind the waiting
    /// writer, for every unrelated wallet in the process.
    #[test]
    fn an_in_flight_payment_does_not_block_an_unrelated_wallets_teardown() {
        let _registry = crate::core_wallet::signed_payment::registry_test_guard();

        runtime().block_on(async {
            let (manager_a, wallet_id_a) = test_platform_wallet_manager().await;
            let (manager_b, wallet_id_b) = test_platform_wallet_manager().await;

            let core_a = manager_a
                .get_wallet(&wallet_id_a)
                .await
                .expect("wallet A present")
                .core()
                .clone();
            let core_b = manager_b
                .get_wallet(&wallet_id_b)
                .await
                .expect("wallet B present")
                .core()
                .clone();
            assert!(
                !core_a.is_same_generation(&core_b),
                "the fixture must produce two distinct generations"
            );

            // Wallet A has a payment in flight, holding A's gate.
            let in_flight_a = core_a.generation_payment_guard().await;

            // Wallet B's teardown must not care.
            let teardown_b = tokio::time::timeout(
                Duration::from_secs(5),
                remove_wallet_and_tear_down_generation(&manager_b, &wallet_id_b),
            )
            .await
            .expect(
                "an unrelated wallet's teardown must not wait on wallet A's in-flight payment — \
                 the lifecycle gate is not scoped to the wallet generation",
            );
            teardown_b.expect("remove B succeeds");

            // A is untouched and still live; B is gone.
            assert!(core_a.is_current_generation().await);
            assert!(!core_b.is_current_generation().await);

            drop(in_flight_a);
        });
    }
}
