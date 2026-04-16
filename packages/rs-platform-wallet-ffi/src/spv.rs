//! FFI bindings for PlatformWalletManager's SPV runtime.
//!
//! Exposes a minimal subset of the SPV client API needed by the UI:
//! sync progress polling, lifecycle controls (stop, clear storage),
//! and masternode toggle. The `start` entry point mirrors the old
//! `dash_spv_ffi_client_new` + `run` flow — it accepts a minimal
//! config (network + data dir + peer overrides) and fires up the
//! SpvRuntime on a background task until stopped.

use std::ffi::CStr;
use std::os::raw::c_char;

use platform_wallet::spv::{ClientConfig, ProgressPercentage, SyncProgress, SyncState};

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;

// ---------------------------------------------------------------------------
// Sync progress
// ---------------------------------------------------------------------------

/// Sync state enum mirroring dash_spv::sync::SyncState.
///
/// 0 = WaitForEvents, 1 = WaitingForConnections, 2 = Syncing,
/// 3 = Synced, 4 = Error.
pub const SPV_SYNC_STATE_WAIT_FOR_EVENTS: u32 = 0;
pub const SPV_SYNC_STATE_WAITING_FOR_CONNECTIONS: u32 = 1;
pub const SPV_SYNC_STATE_SYNCING: u32 = 2;
pub const SPV_SYNC_STATE_SYNCED: u32 = 3;
pub const SPV_SYNC_STATE_ERROR: u32 = 4;

/// Flattened sync progress summary for FFI.
///
/// Each `has_*` flag indicates whether the corresponding manager has
/// started reporting progress. When `has_*` is false, the numeric fields
/// for that manager should be ignored.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FFISpvSyncProgress {
    pub overall_state: u32,
    pub overall_percentage: f64,

    pub has_headers: bool,
    pub headers_state: u32,
    pub headers_current: u32,
    pub headers_target: u32,
    pub headers_percentage: f64,

    pub has_filter_headers: bool,
    pub filter_headers_state: u32,
    pub filter_headers_current: u32,
    pub filter_headers_target: u32,
    pub filter_headers_percentage: f64,

    pub has_filters: bool,
    pub filters_state: u32,
    pub filters_current: u32,
    pub filters_target: u32,
    pub filters_percentage: f64,

    pub has_masternodes: bool,
    pub masternodes_state: u32,
    pub masternodes_current: u32,
    pub masternodes_target: u32,
    pub masternodes_percentage: f64,
}

impl Default for FFISpvSyncProgress {
    fn default() -> Self {
        Self {
            overall_state: SPV_SYNC_STATE_WAIT_FOR_EVENTS,
            overall_percentage: 0.0,
            has_headers: false,
            headers_state: SPV_SYNC_STATE_WAIT_FOR_EVENTS,
            headers_current: 0,
            headers_target: 0,
            headers_percentage: 0.0,
            has_filter_headers: false,
            filter_headers_state: SPV_SYNC_STATE_WAIT_FOR_EVENTS,
            filter_headers_current: 0,
            filter_headers_target: 0,
            filter_headers_percentage: 0.0,
            has_filters: false,
            filters_state: SPV_SYNC_STATE_WAIT_FOR_EVENTS,
            filters_current: 0,
            filters_target: 0,
            filters_percentage: 0.0,
            has_masternodes: false,
            masternodes_state: SPV_SYNC_STATE_WAIT_FOR_EVENTS,
            masternodes_current: 0,
            masternodes_target: 0,
            masternodes_percentage: 0.0,
        }
    }
}

fn state_to_u32(s: SyncState) -> u32 {
    match s {
        SyncState::WaitForEvents => SPV_SYNC_STATE_WAIT_FOR_EVENTS,
        SyncState::WaitingForConnections => SPV_SYNC_STATE_WAITING_FOR_CONNECTIONS,
        SyncState::Syncing => SPV_SYNC_STATE_SYNCING,
        SyncState::Synced => SPV_SYNC_STATE_SYNCED,
        SyncState::Error => SPV_SYNC_STATE_ERROR,
    }
}

fn progress_to_ffi(p: &SyncProgress) -> FFISpvSyncProgress {
    let mut out = FFISpvSyncProgress::default();
    out.overall_state = state_to_u32(p.state());
    out.overall_percentage = p.percentage();

    if let Ok(h) = p.headers() {
        out.has_headers = true;
        out.headers_state = state_to_u32(h.state());
        out.headers_current = h.current_height();
        out.headers_target = h.target_height();
        out.headers_percentage = h.percentage();
    }
    if let Ok(fh) = p.filter_headers() {
        out.has_filter_headers = true;
        out.filter_headers_state = state_to_u32(fh.state());
        out.filter_headers_current = fh.current_height();
        out.filter_headers_target = fh.target_height();
        out.filter_headers_percentage = fh.percentage();
    }
    if let Ok(fl) = p.filters() {
        out.has_filters = true;
        out.filters_state = state_to_u32(fl.state());
        out.filters_current = fl.current_height();
        out.filters_target = fl.target_height();
        out.filters_percentage = fl.percentage();
    }
    if let Ok(mn) = p.masternodes() {
        out.has_masternodes = true;
        out.masternodes_state = state_to_u32(mn.state());
        out.masternodes_current = mn.current_height();
        out.masternodes_target = mn.target_height();
        out.masternodes_percentage = if mn.target_height() == 0 {
            0.0
        } else {
            (mn.current_height() as f64 / mn.target_height() as f64).min(1.0)
        };
    }

    out
}

/// Poll the current sync progress.
///
/// Writes a snapshot of SPV sync state into `out_progress`. If the SPV
/// client is not running, all `has_*` flags are false and the state is
/// `WaitForEvents`.
///
/// # Safety
/// Caller must ensure `handle` is a valid manager handle and `out_progress`
/// is a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_sync_progress(
    handle: Handle,
    out_progress: *mut FFISpvSyncProgress,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_progress.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| {
            let progress = runtime().block_on(manager.spv().sync_progress());
            let ffi = match progress {
                Some(p) => progress_to_ffi(&p),
                None => FFISpvSyncProgress::default(),
            };
            *out_progress = ffi;
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Whether the SPV client is currently running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_is_running(
    handle: Handle,
    out_running: *mut bool,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_running.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| {
            *out_running = manager.spv().is_started();
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Start SPV sync in the background.
///
/// `data_dir` — path to the SPV storage directory (must exist and be writable).
/// `network` — 0=Mainnet, 1=Testnet, 2=Devnet, 3=Regtest.
/// `user_agent` — optional user agent string (null to use default).
/// `peers` — optional null-terminated array of peer addresses ("ip:port" or
///           just "ip"); pass `peers=null, peer_count=0` for DNS discovery.
/// `restrict_to_configured_peers` — if true, only connect to listed peers.
/// `start_from_height` — checkpoint height to start syncing from (0 = genesis).
/// `masternode_sync_enabled` — whether to sync masternode lists.
///
/// Returns immediately after spawning the background task.
///
/// # Safety
/// `handle` must be a valid manager handle. String pointers must be valid
/// null-terminated UTF-8 for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_start(
    handle: Handle,
    data_dir: *const c_char,
    network: u32,
    user_agent: *const c_char,
    peers: *const *const c_char,
    peer_count: usize,
    restrict_to_configured_peers: bool,
    start_from_height: u32,
    masternode_sync_enabled: bool,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if data_dir.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    let data_dir_str = match CStr::from_ptr(data_dir).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return PlatformWalletFFIResult::ErrorUtf8Conversion,
    };
    let user_agent_str = if user_agent.is_null() {
        None
    } else {
        match CStr::from_ptr(user_agent).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => return PlatformWalletFFIResult::ErrorUtf8Conversion,
        }
    };

    let net = match network {
        0 => dashcore::Network::Mainnet,
        1 => dashcore::Network::Testnet,
        2 => dashcore::Network::Devnet,
        3 => dashcore::Network::Regtest,
        _ => return PlatformWalletFFIResult::ErrorInvalidNetwork,
    };

    let mut peer_list: Vec<String> = Vec::new();
    if !peers.is_null() && peer_count > 0 {
        for i in 0..peer_count {
            let p = *peers.add(i);
            if p.is_null() {
                continue;
            }
            if let Ok(s) = CStr::from_ptr(p).to_str() {
                peer_list.push(s.to_string());
            }
        }
    }

    let _ = out_error;
    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| {
            let mut config = ClientConfig::default();
            config.network = net;
            config.storage_path = std::path::PathBuf::from(&data_dir_str);
            if let Some(ua) = user_agent_str.clone() {
                config.user_agent = Some(ua);
            }
            if start_from_height > 0 {
                config.start_from_height = Some(start_from_height);
            }
            config.enable_masternodes = masternode_sync_enabled;
            config.restrict_to_configured_peers = restrict_to_configured_peers;
            for p in &peer_list {
                if let Ok(addr) = p.parse() {
                    config.peers.push(addr);
                }
            }

            // Enter the runtime so `spawn_in_background` can call `tokio::spawn`.
            let _guard = runtime().enter();
            manager.spv_arc().spawn_in_background(config);
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Stop the SPV client.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_stop(
    handle: Handle,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| {
            runtime().block_on(async {
                let _ = manager.spv().stop().await;
            });
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Clear all persisted SPV storage (headers, filters, state).
///
/// The SPV client must be running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_clear_storage(
    handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| {
            match runtime().block_on(manager.spv().clear_storage()) {
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
