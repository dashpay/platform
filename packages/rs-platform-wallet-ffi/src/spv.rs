//! FFI bindings for PlatformWalletManager's SPV runtime.

use std::ffi::CStr;
use std::os::raw::c_char;

use dashcore::sml::llmq_type::LlmqDevnetParams;
use platform_wallet::spv::{ClientConfig, ProgressPercentage, SyncProgress, SyncState};

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::types::FFINetwork;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

pub const SPV_SYNC_STATE_WAIT_FOR_EVENTS: u32 = 0;
pub const SPV_SYNC_STATE_WAITING_FOR_CONNECTIONS: u32 = 1;
pub const SPV_SYNC_STATE_SYNCING: u32 = 2;
pub const SPV_SYNC_STATE_SYNCED: u32 = 3;
pub const SPV_SYNC_STATE_ERROR: u32 = 4;

/// Flattened sync progress summary for FFI.
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

#[allow(clippy::field_reassign_with_default)]
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
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_sync_progress(
    handle: Handle,
    out_progress: *mut FFISpvSyncProgress,
) -> PlatformWalletFFIResult {
    check_ptr!(out_progress);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.spv().sync_progress())
    });
    let progress = unwrap_option_or_return!(option);
    *out_progress = match progress {
        Some(p) => progress_to_ffi(&p),
        None => FFISpvSyncProgress::default(),
    };
    PlatformWalletFFIResult::ok()
}

/// Whether the SPV client is currently running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_is_running(
    handle: Handle,
    out_running: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_running);
    let option =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| manager.spv().is_started());
    *out_running = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Read the unix-seconds block time of the SPV header storage's
/// current tip. Useful as a "is core producing blocks?" indicator —
/// a stale value across multiple polls means the chain has stalled.
///
/// `*out_unix_seconds` is set to `0` when the SPV client isn't
/// running, no headers have been stored yet, or the tip header
/// can't be read for any reason. The function still returns
/// success in those cases — `0` is the in-band sentinel.
///
/// # Safety
/// - `out_unix_seconds` must be writable for one `u64` value.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_tip_unix_seconds(
    handle: Handle,
    out_unix_seconds: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_unix_seconds);
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.spv().tip_block_time())
    });
    let tip = unwrap_option_or_return!(option);
    *out_unix_seconds = tip.map(|t| t as u64).unwrap_or(0);
    PlatformWalletFFIResult::ok()
}

/// Start SPV sync in the background.
///
/// `enable_masternodes` is NOT a caller knob: platform-wallet always
/// needs `ChainLockManager` + `InstantSendManager` running so
/// asset-lock proofs can resolve via the `CLSig` / `ISLock` P2P
/// messages. Disabling masternode sync silently breaks
/// `AssetLockManager::wait_for_proof`, which is a published feature.
/// Hardcoded to `true` here; if a future caller has a real reason to
/// run SPV without masternode sync, it can construct the wallet
/// manager without the asset-lock path instead.
///
/// Devnet support:
/// - `devnet_name` must be set (non-null) when `network == Devnet` and
///   must be null on every other network. When set, the SPV user agent
///   is rewritten to embed the `devnet.devnet-<name>` substring Dash
///   Core peers gate inbound devnet handshakes on (matches the format
///   the `dash-spv` binary uses).
/// - `llmq_devnet_size` / `llmq_devnet_threshold` mirror Dash Core's
///   `-llmqdevnetparams=<size>:<threshold>`. Both zero means "no
///   override". Setting one without the other, or setting them on a
///   non-devnet network, is rejected.
#[no_mangle]
#[allow(clippy::field_reassign_with_default)]
pub unsafe extern "C" fn platform_wallet_manager_spv_start(
    handle: Handle,
    data_dir: *const c_char,
    network: FFINetwork,
    user_agent: *const c_char,
    peers: *const *const c_char,
    peer_count: usize,
    restrict_to_configured_peers: bool,
    start_from_height: u32,
    devnet_name: *const c_char,
    llmq_devnet_size: u32,
    llmq_devnet_threshold: u32,
) -> PlatformWalletFFIResult {
    check_ptr!(data_dir);
    let data_dir_str = unwrap_result_or_return!(CStr::from_ptr(data_dir).to_str()).to_string();
    let mut user_agent_str = if user_agent.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(CStr::from_ptr(user_agent).to_str()).to_string())
    };

    let net: crate::types::Network = network.into();

    let devnet_name_str = if devnet_name.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(CStr::from_ptr(devnet_name).to_str()).to_string())
    };

    if net == crate::types::Network::Devnet && devnet_name_str.is_none() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "devnet_name is required when network=Devnet",
        );
    }
    if net != crate::types::Network::Devnet && devnet_name_str.is_some() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "devnet_name is only valid on devnet",
        );
    }
    if (llmq_devnet_size > 0) ^ (llmq_devnet_threshold > 0) {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "llmq_devnet_size and llmq_devnet_threshold must both be set or both zero",
        );
    }
    if llmq_devnet_size > 0 && net != crate::types::Network::Devnet {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "llmq_devnet_params is only valid on devnet",
        );
    }

    // Dash Core devnet peers disconnect any inbound connection whose
    // user agent doesn't contain `devnet.devnet-<name>`. Rebuild the
    // user agent to embed the substring while keeping whatever base
    // the caller (or our default) provided. Mirrors the dash-spv
    // binary's `/rust-dash-spv:VERSION(devnet.devnet-NAME)/` format.
    if let Some(name) = devnet_name_str.as_deref() {
        let base = user_agent_str
            .clone()
            .unwrap_or_else(|| format!("platform-wallet-ffi:{}", env!("CARGO_PKG_VERSION")));
        user_agent_str = Some(format!("/{base}(devnet.devnet-{name})/"));
    }

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

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let mut config = ClientConfig::default();
        config.network = net;
        config.storage_path = std::path::PathBuf::from(&data_dir_str);
        if let Some(ua) = user_agent_str.clone() {
            config.user_agent = Some(ua);
        }
        if start_from_height > 0 {
            config.start_from_height = Some(start_from_height);
        }
        // Asset-lock proof acquisition (`AssetLockManager::wait_for_proof`)
        // depends on `CLSig` / `ISLock` P2P messages reaching the wallet,
        // which only happens when `ChainLockManager` + `InstantSendManager`
        // are spawned (see dash-spv/src/client/lifecycle.rs). Hardcode
        // here so trusted-SDK callers (who'd otherwise disable masternode
        // sync) don't silently break asset-lock-funded identity
        // registration.
        config.enable_masternodes = true;
        config.restrict_to_configured_peers = restrict_to_configured_peers;
        for p in &peer_list {
            if let Ok(addr) = p.parse() {
                config.peers.push(addr);
            }
        }
        if llmq_devnet_size > 0 {
            config.llmq_devnet_params = Some(LlmqDevnetParams {
                size: llmq_devnet_size,
                threshold: llmq_devnet_threshold,
            });
        }

        let _guard = runtime().enter();
        manager.spv_arc().spawn_in_background(config);
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Stop the SPV client.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_stop(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(async {
            let _ = manager.spv().stop().await;
        });
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Clear all persisted SPV storage (headers, filters, state).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_clear_storage(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.spv().clear_storage())
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}
