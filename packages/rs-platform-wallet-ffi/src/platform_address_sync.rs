//! FFI bindings for PlatformWalletManager's platform-address sync coordinator.

use std::os::raw::c_char;
use std::time::Duration;

use dash_sdk::platform::address_sync::AddressSyncMetrics;

use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::AddressSyncConfigFFI;
use crate::runtime::{run_on_big_stack_thread, runtime};
use crate::{check_ptr, unwrap_option_or_return};

/// Flattened sync metrics for one wallet result in a platform-address sync pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PlatformAddressSyncMetricsFFI {
    pub trunk_queries: u32,
    pub branch_queries: u32,
    pub total_elements_seen: u32,
    pub total_proof_bytes: u32,
    pub iterations: u32,
    pub compacted_queries: u32,
    pub recent_queries: u32,
    pub recent_entries_returned: u32,
    pub compacted_entries_returned: u32,
}

impl From<&AddressSyncMetrics> for PlatformAddressSyncMetricsFFI {
    fn from(metrics: &AddressSyncMetrics) -> Self {
        let clamp = |value: usize| u32::try_from(value).unwrap_or(u32::MAX);
        Self {
            trunk_queries: clamp(metrics.trunk_queries),
            branch_queries: clamp(metrics.branch_queries),
            total_elements_seen: clamp(metrics.total_elements_seen),
            total_proof_bytes: clamp(metrics.total_proof_bytes),
            iterations: clamp(metrics.iterations),
            compacted_queries: clamp(metrics.compacted_queries),
            recent_queries: clamp(metrics.recent_queries),
            recent_entries_returned: clamp(metrics.recent_entries_returned),
            compacted_entries_returned: clamp(metrics.compacted_entries_returned),
        }
    }
}

/// Per-wallet outcome from a completed platform-address sync pass.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlatformAddressSyncWalletResultFFI {
    pub wallet_id: [u8; 32],
    pub success: bool,
    pub found_count: usize,
    pub absent_count: usize,
    pub checkpoint_height: u64,
    pub new_sync_height: u64,
    pub new_sync_timestamp: u64,
    pub last_known_recent_block: u64,
    pub metrics: PlatformAddressSyncMetricsFFI,
    pub error_message: *const c_char,
}

impl Default for PlatformAddressSyncWalletResultFFI {
    fn default() -> Self {
        Self {
            wallet_id: [0; 32],
            success: false,
            found_count: 0,
            absent_count: 0,
            checkpoint_height: 0,
            new_sync_height: 0,
            new_sync_timestamp: 0,
            last_known_recent_block: 0,
            metrics: PlatformAddressSyncMetricsFFI::default(),
            error_message: std::ptr::null(),
        }
    }
}

/// Start the platform-address sync manager in the background.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_platform_address_sync_start(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let _entered = runtime().enter();
        manager.platform_address_sync_arc().start();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Stop the platform-address sync manager if it is running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_platform_address_sync_stop(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.platform_address_sync().stop();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Whether the platform-address sync manager background loop is running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_platform_address_sync_is_running(
    handle: Handle,
    out_running: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_running);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.platform_address_sync().is_running()
    });
    *out_running = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Whether a platform-address sync pass is currently in flight.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_platform_address_sync_is_syncing(
    handle: Handle,
    out_syncing: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_syncing);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.platform_address_sync().is_syncing()
    });
    *out_syncing = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Unix seconds of the last completed platform-address sync pass, or 0 if none ran.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_platform_address_sync_last_sync_unix_seconds(
    handle: Handle,
    out_last_sync_unix: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_last_sync_unix);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager
            .platform_address_sync()
            .last_sync_unix_seconds()
            .unwrap_or(0)
    });
    *out_last_sync_unix = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Set the background platform-address sync interval in seconds.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_platform_address_sync_set_interval(
    handle: Handle,
    interval_seconds: u64,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager
            .platform_address_sync()
            .set_interval(Duration::from_secs(interval_seconds));
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Replace the shared platform-address sync config used on each pass.
///
/// Passing `config = NULL` restores the SDK defaults.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_platform_address_sync_set_config(
    handle: Handle,
    config: *const AddressSyncConfigFFI,
) -> PlatformWalletFFIResult {
    let cfg = if config.is_null() {
        None
    } else {
        Some((*config).into())
    };
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.platform_address_sync().set_config(cfg);
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Run one platform-address sync pass across all registered wallets.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_platform_address_sync_sync_now(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        // Big-stack polling, NOT bare `runtime().block_on`: the pass
        // verifies GroveDB proofs whose recursion blows the ~512 KB
        // stack of the host's dispatch/concurrency calling thread
        // (same SIGBUS as the shielded/dashpay Sync Now buttons).
        // `run_on_big_stack_thread` rather than `block_on_worker`
        // because this future trips rustc's implied-lifetime-bound
        // limitation (rust-lang/rust issue #100013) against
        // `block_on_worker`'s `Send + 'static` bounds.
        run_on_big_stack_thread(|| {
            runtime().block_on(manager.platform_address_sync().sync_now());
        });
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Reset the platform-address (BLAST/DIP-17) incremental-sync watermark
/// and drop every cached balance across all registered wallets, forcing
/// a full rescan on the next sync. Backs the SwiftExampleApp "Clear"
/// button.
///
/// `reset_platform_address_sync_state` quiesces the background sync loop
/// before resetting so no in-flight pass can re-write the watermark. The
/// loop is left stopped (not restarted) — the host re-arms it via
/// `..._start`, or uses one-shot `..._sync_now`, afterward.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_platform_address_sync_reset(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.reset_platform_address_sync_state())
    });
    let result = unwrap_option_or_return!(option);
    if let Err(e) = result {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("reset_platform_address_sync_state failed: {e}"),
        );
    }
    PlatformWalletFFIResult::ok()
}
