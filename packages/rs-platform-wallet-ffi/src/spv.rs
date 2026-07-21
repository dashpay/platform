//! FFI bindings for PlatformWalletManager's SPV runtime.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

use dashcore::sml::llmq_type::LlmqDevnetParams;
use platform_wallet::spv::{
    ClientConfig, DevnetConfig, ProgressPercentage, SpvPeerNodeType, SyncProgress, SyncState,
};

use crate::error::*;
use crate::handle::*;
use crate::runtime::{block_on_worker, runtime};
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

pub const SPV_PEER_NODE_TYPE_UNKNOWN: u32 = 0;
pub const SPV_PEER_NODE_TYPE_NORMAL: u32 = 1;
pub const SPV_PEER_NODE_TYPE_MASTERNODE: u32 = 2;
pub const SPV_PEER_NODE_TYPE_EVONODE: u32 = 3;

/// One connected SPV peer, classified against the masternode list.
///
/// `address` is a heap-owned NUL-terminated `ip:port` string, freed by
/// the paired [`platform_wallet_manager_spv_connected_peers_free`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FFISpvPeerInfo {
    /// Heap-owned `ip:port` string. Always non-null on a successful row.
    pub address: *mut c_char,
    /// One of the `SPV_PEER_NODE_TYPE_*` constants.
    pub node_type: u32,
}

fn peer_node_type_to_u32(node_type: SpvPeerNodeType) -> u32 {
    match node_type {
        SpvPeerNodeType::Unknown => SPV_PEER_NODE_TYPE_UNKNOWN,
        SpvPeerNodeType::Normal => SPV_PEER_NODE_TYPE_NORMAL,
        SpvPeerNodeType::Masternode => SPV_PEER_NODE_TYPE_MASTERNODE,
        SpvPeerNodeType::Evonode => SPV_PEER_NODE_TYPE_EVONODE,
    }
}

/// Snapshot of the peers the SPV client is currently connected to, each
/// classified against the masternode list (`SPV_PEER_NODE_TYPE_UNKNOWN`
/// while the masternode list hasn't synced yet).
///
/// On success `*out_entries` points at a heap-owned `[FFISpvPeerInfo]`
/// of length `*out_count`; the caller must release it via
/// [`platform_wallet_manager_spv_connected_peers_free`]. No connected
/// peers (or a stopped client) returns `ok()` with
/// `*out_entries = null`, `*out_count = 0`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_connected_peers(
    handle: Handle,
    out_entries: *mut *const FFISpvPeerInfo,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_entries);
    check_ptr!(out_count);
    *out_entries = std::ptr::null();
    *out_count = 0;

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.spv().connected_peers())
    });
    let peers = unwrap_option_or_return!(option);
    if peers.is_empty() {
        return PlatformWalletFFIResult::ok();
    }

    let entries: Vec<FFISpvPeerInfo> = peers
        .into_iter()
        .filter_map(|peer| {
            let address = CString::new(peer.address.to_string()).ok()?;
            Some(FFISpvPeerInfo {
                address: address.into_raw(),
                node_type: peer_node_type_to_u32(peer.node_type),
            })
        })
        .collect();
    let count = entries.len();
    *out_entries = Box::into_raw(entries.into_boxed_slice()) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

/// Release a peer list returned by
/// [`platform_wallet_manager_spv_connected_peers`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_connected_peers_free(
    entries: *mut FFISpvPeerInfo,
    count: usize,
) {
    if entries.is_null() || count == 0 {
        return;
    }
    // Walk every row first to release its heap-owned `address` string
    // before reclaiming the parent slice.
    let slice = std::slice::from_raw_parts(entries, count);
    for entry in slice {
        if !entry.address.is_null() {
            let _ = CString::from_raw(entry.address);
        }
    }
    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count));
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
    // Reject empty / any-whitespace / `/`-containing names synchronously
    // here rather than letting `DevnetConfig::validate` surface them
    // asynchronously from `spawn_in_background`. Mirrors
    // `DevnetConfig::validate` (which only checks empty + `/`) and
    // additionally rejects any whitespace — leading, trailing, or
    // interior — so callers without a pre-filter (other language
    // bindings, integration tests) can't produce a malformed
    // `(devnet.devnet- foo )` user agent that Dash Core peers silently
    // drop. Rejecting (rather than auto-trimming) keeps the rule
    // deterministic and avoids the Unicode-whitespace asymmetry
    // between `str::trim` and Swift's `CharacterSet.whitespaces`.
    if let Some(name) = devnet_name_str.as_deref() {
        if name.is_empty() {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "devnet_name must not be empty",
            );
        }
        if name.chars().any(char::is_whitespace) {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "devnet_name must not contain whitespace",
            );
        }
        if name.contains('/') {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "devnet_name must not contain '/'",
            );
        }
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
        if let Some(name) = devnet_name_str.as_deref() {
            let mut devnet = DevnetConfig::new(name);
            if llmq_devnet_size > 0 {
                devnet.llmq_params = Some(LlmqDevnetParams {
                    size: llmq_devnet_size,
                    threshold: llmq_devnet_threshold,
                });
            }
            config.devnet = Some(devnet);
        }

        let spv = manager.spv_arc();
        let start_result = {
            let spv = spv.clone();
            block_on_worker(async move { spv.start(config).await })
        };

        if start_result.is_ok() {
            if let Err(message) = manager.acquire_spv_source() {
                let spv_to_stop = Arc::clone(&spv);
                let _ = block_on_worker(async move { spv_to_stop.stop().await });
                return Err(platform_wallet::PlatformWalletError::SpvError(message));
            }
            let _guard = runtime().enter();
            spv.spawn_run_loop();
        }

        start_result
    });

    let start_result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(start_result);

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
        manager.release_spv_source();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Arm an organic compact-filter rescan for one wallet by rewinding its
/// SPV filter-scan checkpoint (`synced_height`) to `from_height`.
///
/// This does not itself scan; it rewinds the checkpoint on the *shared*
/// wallet state the running `DashSpvClient` reads. On the filter-sync
/// loop's next tick, `dash-spv`'s `FiltersManager` observes this wallet
/// in `wallets_behind(committed_height)`, calls `reset_for_rescan()`,
/// rewinds its committed height to this wallet's `synced_height`, and
/// re-downloads / re-matches compact filters from there — matching any
/// scripts (e.g. newly watched addresses) that weren't in the watch set
/// when those blocks were first scanned.
///
/// `from_height` at or above the wallet's current checkpoint is written
/// verbatim but arms no rescan (the loop only rescans wallets strictly
/// behind the committed height), so a forward/equal set is a harmless
/// no-op for rescan purposes.
///
/// Requires SPV running for an immediate effect; otherwise the rewound
/// checkpoint takes effect when SPV next starts and its filter loop
/// first ticks. The rewound checkpoint is in-memory only (not persisted
/// by this call); if the process dies mid-rescan the wallet is simply
/// still behind and the next start re-arms the backfill.
///
/// # Safety
/// - `wallet_id` must point to 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_spv_rescan_filters(
    handle: Handle,
    wallet_id: *const u8,
    from_height: u32,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.spv_rescan_filters_blocking(&wid, from_height)
    });
    let found = unwrap_option_or_return!(option);
    if !found {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "Wallet not found".to_string(),
        );
    }
    tracing::info!(
        wallet_id = %hex::encode(wid),
        from_height,
        "platform_wallet_manager_spv_rescan_filters: armed filter rescan"
    );
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
