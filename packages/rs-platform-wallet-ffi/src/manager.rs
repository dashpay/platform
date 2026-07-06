//! FFI bindings for PlatformWalletManager (wallet lifecycle management).

use crate::check_ptr;
use crate::error::*;
use crate::event_handler::{EventHandlerCallbacks, FFIEventHandler};
use crate::handle::*;
use crate::persistence::{FFIPersister, PersistenceCallbacks};
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

/// `reason_code`: the persisted row had no usable account manifest to
/// rebuild the account collection from.
pub const LOAD_SKIP_REASON_MISSING_MANIFEST: u32 = 100;
/// `reason_code`: a manifest `account_xpub` failed to parse as a
/// well-formed extended public key.
pub const LOAD_SKIP_REASON_MALFORMED_XPUB: u32 = 101;
/// `reason_code`: any other structural decode / projection failure on
/// the persisted row.
pub const LOAD_SKIP_REASON_DECODE_ERROR: u32 = 102;
/// `reason_code`: the carried managed-info snapshot does not describe its
/// persisted row (wallet_id/network differ, or its account set diverges
/// from the row's account manifest) — a wrong-row snapshot.
pub const LOAD_SKIP_REASON_SNAPSHOT_IDENTITY_MISMATCH: u32 = 103;
/// `reason_code`: a persisted account-manifest row failed its integrity
/// checksum (`SHA-256(wallet_id ‖ account_xpub_bytes)` mismatch — a row
/// bound to the wrong wallet or a blob mutated in place).
pub const LOAD_SKIP_REASON_MANIFEST_INTEGRITY_MISMATCH: u32 = 104;
/// `reason_code`: an unrecognized `CorruptKind` — forward-compat
/// fallback until this crate maps a newly added corrupt-row family.
pub const LOAD_SKIP_REASON_CORRUPT_OTHER: u32 = 199;
/// `reason_code`: an unrecognized `SkipReason` — forward-compat
/// fallback until this crate maps a newly added skip reason.
pub const LOAD_SKIP_REASON_OTHER: u32 = 200;
/// `reason_code`: the wallet was already registered before this load
/// pass reached it (a prior load, or a runtime-created wallet), so its
/// persisted row was not freshly loaded. Not corruption.
pub const LOAD_SKIP_REASON_ALREADY_REGISTERED: u32 = 300;

/// One wallet skipped during `load_from_persistor` because its
/// persisted row was structurally corrupt (per-row decode failure).
/// The load path is seedless and watch-only, so this is the only skip
/// reason. `reason_code` is per-`CorruptKind` family — see its table.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SkippedWalletFFI {
    /// The (public) 32-byte wallet id that was skipped.
    pub wallet_id: [u8; 32],
    /// Skip reason — one of the `LOAD_SKIP_REASON_*` constants:
    /// [`LOAD_SKIP_REASON_MISSING_MANIFEST`] (100),
    /// [`LOAD_SKIP_REASON_MALFORMED_XPUB`] (101),
    /// [`LOAD_SKIP_REASON_DECODE_ERROR`] (102),
    /// [`LOAD_SKIP_REASON_SNAPSHOT_IDENTITY_MISMATCH`] (103),
    /// [`LOAD_SKIP_REASON_MANIFEST_INTEGRITY_MISMATCH`] (104),
    /// [`LOAD_SKIP_REASON_CORRUPT_OTHER`] (199),
    /// [`LOAD_SKIP_REASON_OTHER`] (200), or
    /// [`LOAD_SKIP_REASON_ALREADY_REGISTERED`] (300). No secret material
    /// is ever carried.
    pub reason_code: u32,
}

/// C-visible summary of one `load_from_persistor` pass so the host can
/// see which wallets loaded and which were skipped (and why) instead
/// of the outcome being silently discarded.
///
/// The count pair encodes the Rust `LoadOutcome` 3-state: `skipped_count
/// == 0` is a full load, `loaded_count == 0` with skips is
/// nothing-usable, and both non-zero is a partial load.
///
/// `skipped` is a heap array of length `skipped_count`; pass this
/// struct (by pointer) to
/// [`platform_wallet_load_outcome_free`] exactly once to release it.
#[repr(C)]
#[derive(Debug)]
pub struct LoadOutcomeFFI {
    /// Number of wallets fully reconstructed + registered.
    pub loaded_count: usize,
    /// Length of the `skipped` array.
    pub skipped_count: usize,
    /// Heap-allocated skipped-wallet array (null iff `skipped_count`
    /// is 0). Owned by Rust until `platform_wallet_load_outcome_free`.
    pub skipped: *mut SkippedWalletFFI,
}

fn skip_reason_code(reason: &platform_wallet::SkipReason) -> u32 {
    use platform_wallet::manager::load_outcome::CorruptKind;
    match reason {
        platform_wallet::SkipReason::CorruptPersistedRow { kind } => match kind {
            CorruptKind::MissingManifest => LOAD_SKIP_REASON_MISSING_MANIFEST,
            CorruptKind::MalformedXpub => LOAD_SKIP_REASON_MALFORMED_XPUB,
            CorruptKind::SnapshotIdentityMismatch => LOAD_SKIP_REASON_SNAPSHOT_IDENTITY_MISMATCH,
            CorruptKind::DecodeError(_) => LOAD_SKIP_REASON_DECODE_ERROR,
            CorruptKind::ManifestIntegrityMismatch => LOAD_SKIP_REASON_MANIFEST_INTEGRITY_MISMATCH,
            // `CorruptKind` is #[non_exhaustive]; a future variant maps to a
            // generic corrupt-row code until this mapping is extended.
            _ => LOAD_SKIP_REASON_CORRUPT_OTHER,
        },
        platform_wallet::SkipReason::AlreadyRegistered => LOAD_SKIP_REASON_ALREADY_REGISTERED,
        // `SkipReason` is #[non_exhaustive]; a future reason maps to a
        // generic skip code until this mapping is extended.
        _ => LOAD_SKIP_REASON_OTHER,
    }
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
/// builds a keyless reconstruction payload per wallet, then registers
/// each one as a **watch-only** wallet. No signing keys are derived
/// here — signing happens later, on demand, via the configured
/// `MnemonicResolverHandle` (`sign_with_mnemonic_resolver` and its
/// siblings), which fail-closed gate the resolver-supplied seed
/// against the loaded `wallet_id`. Does not produce wallet handles —
/// follow up with [`platform_wallet_manager_get_wallet`] per
/// `wallet_id`.
///
/// A wallet whose persisted row is structurally corrupt is
/// **skipped**, not failed: the call still returns `Success`, every
/// skipped `(wallet_id, reason)` is logged, and — when `out_outcome`
/// is non-null — surfaced through it.
///
/// # Safety
/// - `out_outcome` may be null (caller doesn't want the summary);
///   otherwise it must point to writable `LoadOutcomeFFI` storage and
///   the caller must later release it via
///   [`platform_wallet_load_outcome_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_load_from_persistor(
    manager_handle: Handle,
    out_outcome: *mut LoadOutcomeFFI,
) -> PlatformWalletFFIResult {
    // Initialize the out-param first so every early-return path below
    // leaves it releasable (zeroed counts, null `skipped`) — matches this
    // crate's null-init-first out-pointer idiom and keeps
    // `platform_wallet_load_outcome_free` safe on the error paths too.
    if !out_outcome.is_null() {
        std::ptr::write(
            out_outcome,
            LoadOutcomeFFI {
                loaded_count: 0,
                skipped_count: 0,
                skipped: std::ptr::null_mut(),
            },
        );
    }

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        runtime().block_on(manager.load_from_persistor())
    });
    let result = unwrap_option_or_return!(option);
    let outcome = unwrap_result_or_return!(result);

    // Never silently drop the outcome: log a structured summary plus
    // one line per skipped wallet (the host can inspect / clear the
    // corrupt rows). The `loaded_count`/`skipped_count` pair below
    // encodes the Rust `LoadOutcome` 3-state for the host: skipped == 0
    // is a full load, loaded == 0 with skips is nothing-usable, and both
    // non-zero is a partial load.
    tracing::info!(
        loaded = outcome.loaded().len(),
        skipped = outcome.skipped().len(),
        "platform_wallet_manager_load_from_persistor complete"
    );
    for (wid, reason) in outcome.skipped() {
        tracing::warn!(
            wallet_id = %hex::encode(wid),
            reason = %reason,
            "load_from_persistor skipped a persisted wallet"
        );
    }

    if !out_outcome.is_null() {
        let skipped_vec: Vec<SkippedWalletFFI> = outcome
            .skipped()
            .iter()
            .map(|(wid, reason)| SkippedWalletFFI {
                wallet_id: *wid,
                reason_code: skip_reason_code(reason),
            })
            .collect();
        let skipped_count = skipped_vec.len();
        let skipped_ptr = crate::core_wallet_types::vec_to_ptr(skipped_vec);
        std::ptr::write(
            out_outcome,
            LoadOutcomeFFI {
                loaded_count: outcome.loaded().len(),
                skipped_count,
                skipped: skipped_ptr,
            },
        );
    }
    PlatformWalletFFIResult::ok()
}

/// Release the heap `skipped` array a successful
/// [`platform_wallet_manager_load_from_persistor`] wrote into a
/// `LoadOutcomeFFI`. Idempotent: nulls the pointer after freeing, and
/// a null `outcome` (or already-freed array) is a no-op.
///
/// # Safety
/// `outcome` must point to a `LoadOutcomeFFI` previously populated by
/// `platform_wallet_manager_load_from_persistor`, not freed already.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_load_outcome_free(outcome: *mut LoadOutcomeFFI) {
    if outcome.is_null() {
        return;
    }
    let o = &mut *outcome;
    if !o.skipped.is_null() && o.skipped_count > 0 {
        let slice = std::slice::from_raw_parts_mut(o.skipped, o.skipped_count);
        drop(Box::from_raw(slice as *mut [SkippedWalletFFI]));
    }
    o.skipped = std::ptr::null_mut();
    o.skipped_count = 0;
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
    fn load_skip_reason_wire_values_are_stable() {
        // FFI consumers hardcode these numbers; the ABI must not drift.
        assert_eq!(LOAD_SKIP_REASON_MISSING_MANIFEST, 100);
        assert_eq!(LOAD_SKIP_REASON_MALFORMED_XPUB, 101);
        assert_eq!(LOAD_SKIP_REASON_DECODE_ERROR, 102);
        assert_eq!(LOAD_SKIP_REASON_SNAPSHOT_IDENTITY_MISMATCH, 103);
        assert_eq!(LOAD_SKIP_REASON_MANIFEST_INTEGRITY_MISMATCH, 104);
        assert_eq!(LOAD_SKIP_REASON_CORRUPT_OTHER, 199);
        assert_eq!(LOAD_SKIP_REASON_OTHER, 200);
    }

    #[test]
    fn skip_reason_code_maps_known_kinds_to_constants() {
        use platform_wallet::manager::load_outcome::CorruptKind;
        use platform_wallet::SkipReason;

        let corrupt = |kind| SkipReason::CorruptPersistedRow { kind };
        assert_eq!(
            skip_reason_code(&corrupt(CorruptKind::MissingManifest)),
            LOAD_SKIP_REASON_MISSING_MANIFEST
        );
        assert_eq!(
            skip_reason_code(&corrupt(CorruptKind::MalformedXpub)),
            LOAD_SKIP_REASON_MALFORMED_XPUB
        );
        assert_eq!(
            skip_reason_code(&corrupt(CorruptKind::SnapshotIdentityMismatch)),
            LOAD_SKIP_REASON_SNAPSHOT_IDENTITY_MISMATCH
        );
        assert_eq!(
            skip_reason_code(&corrupt(CorruptKind::DecodeError("boom".into()))),
            LOAD_SKIP_REASON_DECODE_ERROR
        );
        assert_eq!(
            skip_reason_code(&corrupt(CorruptKind::ManifestIntegrityMismatch)),
            LOAD_SKIP_REASON_MANIFEST_INTEGRITY_MISMATCH
        );
    }

    #[test]
    fn load_from_persistor_initializes_out_param_on_early_return() {
        // An unknown handle early-returns before the success block. The
        // out-param must be reset to a releasable zeroed state so a caller
        // that later calls `platform_wallet_load_outcome_free` never does
        // `Box::from_raw` on the uninitialized `skipped` pointer.
        let mut outcome = LoadOutcomeFFI {
            loaded_count: 42,
            skipped_count: 7,
            skipped: std::ptr::NonNull::<SkippedWalletFFI>::dangling().as_ptr(),
        };

        let result =
            unsafe { platform_wallet_manager_load_from_persistor(NULL_HANDLE, &mut outcome) };

        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(outcome.loaded_count, 0);
        assert_eq!(outcome.skipped_count, 0);
        assert!(outcome.skipped.is_null());

        // Null `skipped` now makes the release path a safe no-op.
        unsafe { platform_wallet_load_outcome_free(&mut outcome) };
    }
}
