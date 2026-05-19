//! FFI bindings for `PlatformWalletManager`'s shielded sync
//! coordinator + the host-driven `bind_shielded` entry point.
//!
//! Mirror of [`platform_address_sync`](crate::platform_address_sync)
//! for the Orchard/ZK path. The whole module is feature-gated behind
//! `shielded`; builds without the feature emit none of these symbols
//! and the upstream [`ShieldedSyncManager`] doesn't exist.
//!
//! [`ShieldedSyncManager`]: platform_wallet::manager::shielded_sync::ShieldedSyncManager

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::time::Duration;

use platform_wallet::wallet::shielded::ShieldedSyncSummary;

use zeroize::Zeroizing;

use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::parse_mnemonic_any_language;
use crate::runtime::runtime;
use crate::shielded_types::ShieldedSyncWalletResultFFI;
use crate::{check_ptr, unwrap_option_or_return};
use rs_sdk_ffi::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};

impl ShieldedSyncWalletResultFFI {
    pub(crate) fn ok(wallet_id: [u8; 32], summary: &ShieldedSyncSummary) -> Self {
        let new_notes = u32::try_from(summary.notes_result.new_notes).unwrap_or(u32::MAX);
        let newly_spent = u32::try_from(summary.newly_spent).unwrap_or(u32::MAX);
        Self {
            wallet_id,
            success: true,
            skipped: false,
            new_notes,
            total_scanned: summary.notes_result.total_scanned,
            newly_spent,
            balance: summary.balance,
            error_message: std::ptr::null(),
        }
    }
}

// ---------------------------------------------------------------------------
// Shielded sync coordinator FFI
// ---------------------------------------------------------------------------

/// Start the shielded sync manager in the background.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_start(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let _entered = runtime().enter();
        manager.shielded_sync_arc().start();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Stop the shielded sync manager if it is running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_stop(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.shielded_sync().stop();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Whether the shielded sync background loop is running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_is_running(
    handle: Handle,
    out_running: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_running);

    let option = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.shielded_sync().is_running());
    *out_running = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Whether a shielded sync pass is currently in flight.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_is_syncing(
    handle: Handle,
    out_syncing: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_syncing);

    let option = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.shielded_sync().is_syncing());
    *out_syncing = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Unix seconds of the last completed shielded sync pass, or 0 if
/// none has ever completed.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_last_sync_unix_seconds(
    handle: Handle,
    out_last_sync_unix: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_last_sync_unix);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager
            .shielded_sync()
            .last_sync_unix_seconds()
            .unwrap_or(0)
    });
    *out_last_sync_unix = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Set the background shielded sync interval in seconds.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_set_interval(
    handle: Handle,
    interval_seconds: u64,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager
            .shielded_sync()
            .set_interval(Duration::from_secs(interval_seconds));
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Run one shielded sync pass across all registered wallets.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_sync_now(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.shielded_sync().sync_now());
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Bind shielded
// ---------------------------------------------------------------------------

/// Derive Orchard keys for the given wallet from the host-supplied
/// mnemonic resolver, open the per-network commitment tree at
/// `db_path`, and bind the resulting [`ShieldedWallet`] to the
/// `PlatformWallet`.
///
/// The resolver fires exactly once per call. The mnemonic and the
/// derived seed live in `Zeroizing` buffers and are scrubbed before
/// this function returns; only the FVK / IVK / OVK / default
/// payment address survive on the wallet.
///
/// `db_path` is owned by the host (typically
/// `<docs>/shielded_tree_<network>.sqlite`). The same path is fine
/// to share across wallets on the same network — the commitment
/// tree is global per network and per-wallet decrypted notes live
/// in memory.
///
/// Idempotent: a second call with a different db path / account
/// replaces the previously-bound shielded wallet.
///
/// # Safety
/// - `wallet_id_bytes` must point at 32 readable bytes.
/// - `mnemonic_resolver_handle` must come from
///   [`crate::dash_sdk_mnemonic_resolver_create`].
/// - `db_path_cstr` must be a valid NUL-terminated UTF-8 C string.
///
/// [`ShieldedWallet`]: platform_wallet::wallet::shielded::ShieldedWallet
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_bind_shielded(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    db_path_cstr: *const c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(db_path_cstr);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    let db_path = match CStr::from_ptr(db_path_cstr).to_str() {
        Ok(s) => PathBuf::from(s),
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                format!("db_path is not valid UTF-8: {e}"),
            );
        }
    };

    // Resolve mnemonic via the host callback.
    let mut mnemonic_buf: Zeroizing<[u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]> =
        Zeroizing::new([0u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]);
    let mut mnemonic_len: usize = 0;

    let resolver = &*mnemonic_resolver_handle;
    let resolver_vtable = &*resolver.vtable;
    let rc = (resolver_vtable.resolve)(
        resolver.ctx as *const std::os::raw::c_void,
        wallet_id_bytes,
        mnemonic_buf.as_mut_ptr() as *mut c_char,
        MNEMONIC_RESOLVER_BUFFER_CAPACITY,
        &mut mnemonic_len,
    );

    match rc {
        x if x == mnemonic_resolver_result::SUCCESS => {}
        x if x == mnemonic_resolver_result::NOT_FOUND => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic missing for wallet",
            );
        }
        x if x == mnemonic_resolver_result::BUFFER_TOO_SMALL => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic resolver buffer too small",
            );
        }
        _ => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic resolver failed",
            );
        }
    }
    if mnemonic_len == 0 || mnemonic_len > MNEMONIC_RESOLVER_BUFFER_CAPACITY {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "mnemonic resolver returned empty buffer",
        );
    }

    // Parse and derive seed. Both intermediate forms live in
    // `Zeroizing` so they're scrubbed when this function exits.
    let mnemonic_str = match std::str::from_utf8(&mnemonic_buf[..mnemonic_len]) {
        Ok(s) => s,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                format!("mnemonic is not valid UTF-8: {e}"),
            );
        }
    };
    let mnemonic = match parse_mnemonic_any_language(mnemonic_str) {
        Ok(m) => m,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("invalid mnemonic: {e}"),
            );
        }
    };
    let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(""));
    drop(mnemonic);

    // Look up the wallet on the manager and bind shielded.
    let wallet_arc = {
        let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
            runtime().block_on(manager.get_wallet(&wallet_id))
        });
        unwrap_option_or_return!(option)
    };
    let wallet_arc = match wallet_arc {
        Some(w) => w,
        None => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("wallet not found: {}", hex::encode(wallet_id)),
            );
        }
    };

    if let Err(e) = runtime().block_on(wallet_arc.bind_shielded(seed.as_ref(), account, &db_path)) {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("bind_shielded failed: {e}"),
        );
    }

    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Default Orchard payment address
// ---------------------------------------------------------------------------

/// Read the default Orchard payment address for the bound shielded
/// sub-wallet on `wallet_id`. The host receives the 43 raw bytes
/// (recipient + diversifier) and applies its own bech32m encoding.
///
/// `*out_present` is set to `true` and 43 bytes are written to
/// `out_bytes_43` when the wallet has been bound via
/// [`platform_wallet_manager_bind_shielded`]. When the wallet is
/// known but not bound, `*out_present` is set to `false` and
/// `out_bytes_43` is left untouched. An unknown wallet returns
/// `ErrorWalletOperation`.
///
/// # Safety
/// - `wallet_id_bytes` must point at 32 readable bytes.
/// - `out_bytes_43` must point at 43 writable bytes.
/// - `out_present` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_default_address(
    handle: Handle,
    wallet_id_bytes: *const u8,
    out_bytes_43: *mut u8,
    out_present: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(out_bytes_43);
    check_ptr!(out_present);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    enum Outcome {
        WalletMissing,
        Unbound,
        Bound([u8; 43]),
    }

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(async {
            match manager.get_wallet(&wallet_id).await {
                None => Outcome::WalletMissing,
                Some(w) => match w.shielded_default_address().await {
                    Some(bytes) => Outcome::Bound(bytes),
                    None => Outcome::Unbound,
                },
            }
        })
    });
    let outcome = unwrap_option_or_return!(option);

    match outcome {
        Outcome::WalletMissing => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("wallet not found: {}", hex::encode(wallet_id)),
        ),
        Outcome::Unbound => {
            *out_present = false;
            PlatformWalletFFIResult::ok()
        }
        Outcome::Bound(bytes) => {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes_43, 43);
            *out_present = true;
            PlatformWalletFFIResult::ok()
        }
    }
}

// ---------------------------------------------------------------------------
// Per-wallet sync_now
// ---------------------------------------------------------------------------

/// Run a shielded sync on a single wallet on demand.
///
/// Does not set the manager's global `is_syncing` flag — gate on
/// [`platform_wallet_manager_shielded_sync_is_syncing`] yourself if
/// you want to avoid concurrent passes. Returns an error if the
/// wallet doesn't exist or the sync itself fails. Wallets with no
/// bound shielded sub-wallet succeed silently with no observable
/// state change.
///
/// # Safety
/// - `wallet_id_bytes` must point at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_wallet(
    handle: Handle,
    wallet_id_bytes: *const u8,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.shielded_sync().sync_wallet(&wallet_id))
    });
    let result = unwrap_option_or_return!(option);
    match result {
        Ok(_) => PlatformWalletFFIResult::ok(),
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded sync failed: {e}"),
        ),
    }
}
