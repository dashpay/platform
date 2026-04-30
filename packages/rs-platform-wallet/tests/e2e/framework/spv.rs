//! SPV runtime startup and readiness wait.
//!
//! Currently unused: the harness wires
//! [`rs_sdk_trusted_context_provider::TrustedHttpContextProvider`]
//! instead. Kept compilable for re-enablement (Task #15).
//!
//! [`start_spv`] spawns the SPV client; [`wait_for_mn_list_synced`]
//! polls until the masternode-list manager reaches
//! `SyncState::Synced`. The harness passes a 180s deadline (warm
//! cache); cold-cache runs need [`COLD_CACHE_TIMEOUT_FLOOR`] (600s)
//! and emit info-level progress logs every
//! [`PROGRESS_LOG_INTERVAL`] for debuggability.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dash_sdk::dapi_client::AddressList;
use dash_spv::client::config::MempoolStrategy;
use dash_spv::sync::{ProgressPercentage, SyncState};
use dash_spv::types::ValidationMode;
use dash_spv::ClientConfig;
use dashcore::Network;
use platform_wallet::{changeset::PlatformWalletPersistence, PlatformWalletManager, SpvRuntime};

use super::config::Config;
use super::{FrameworkError, FrameworkResult};

/// Polling interval for [`wait_for_mn_list_synced`].
const READINESS_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Cold-cache floor for [`wait_for_mn_list_synced`] — caller's 180s
/// timeout is sufficient warm but too short for cold testnet
/// (headers + filters + QRInfo). Matches `tests/spv_sync.rs`.
const COLD_CACHE_TIMEOUT_FLOOR: Duration = Duration::from_secs(600);

/// Period for "still waiting" progress logs.
const PROGRESS_LOG_INTERVAL: Duration = Duration::from_secs(30);

/// Spawn the SPV client backing the harness's
/// [`PlatformWalletManager`]. Storage is anchored under
/// `config.workdir_base.join("spv-data")`. Returns the same handle
/// as [`PlatformWalletManager::spv_arc`]; shut it down via
/// [`SpvRuntime::stop`].
///
/// `address_list` is the SDK's live DAPI address list (typically
/// `sdk.address_list()`). P2P peers are seeded from those same
/// IPs with the effective P2P port — keeping a single source of
/// truth instead of forking from `dash_network_seeds` and risking
/// drift between SDK-tracked and SPV-tracked endpoints.
pub async fn start_spv<P>(
    manager: &Arc<PlatformWalletManager<P>>,
    config: &Config,
    address_list: &AddressList,
) -> FrameworkResult<Arc<SpvRuntime>>
where
    P: PlatformWalletPersistence + 'static,
{
    let spv = manager.spv_arc();
    let client_config = build_client_config(config, address_list)?;

    spv.spawn_in_background(client_config);
    tracing::info!(
        target: "platform_wallet::e2e::spv",
        network = ?config.network,
        "SPV runtime spawned in background"
    );

    Ok(spv)
}

/// Block until the SPV mn-list manager reports `Synced`, or the
/// effective timeout (`timeout.max(COLD_CACHE_TIMEOUT_FLOOR)`)
/// elapses. Polls every [`READINESS_POLL_INTERVAL`] and emits an
/// info-level pipeline snapshot every [`PROGRESS_LOG_INTERVAL`] so
/// cold-cache hangs are debuggable from default-level logs.
pub async fn wait_for_mn_list_synced(spv: &SpvRuntime, timeout: Duration) -> FrameworkResult<()> {
    let effective_timeout = timeout.max(COLD_CACHE_TIMEOUT_FLOOR);
    if effective_timeout != timeout {
        tracing::info!(
            target: "platform_wallet::e2e::spv",
            requested = ?timeout,
            effective = ?effective_timeout,
            "raising mn-list-sync timeout to cold-cache floor"
        );
    }

    let start = Instant::now();
    let deadline = start + effective_timeout;
    let mut last_height: Option<u32> = None;
    let mut last_state: Option<SyncState> = None;
    let mut next_progress_log = start + PROGRESS_LOG_INTERVAL;

    loop {
        let progress = spv.sync_progress().await;
        let mn_snapshot = progress
            .as_ref()
            .and_then(|p| p.masternodes().ok().cloned());

        if let Some(mn) = mn_snapshot.as_ref() {
            let height = mn.current_height();
            let state = mn.state();
            if Some(height) != last_height || Some(state) != last_state {
                tracing::debug!(
                    target: "platform_wallet::e2e::spv",
                    state = ?state,
                    current_height = height,
                    target_height = mn.target_height(),
                    elapsed = ?start.elapsed(),
                    "mn-list sync progress"
                );
                last_height = Some(height);
                last_state = Some(state);
            }
            if matches!(state, SyncState::Synced) {
                tracing::info!(
                    target: "platform_wallet::e2e::spv",
                    current_height = height,
                    elapsed = ?start.elapsed(),
                    "mn-list synced"
                );
                return Ok(());
            }
            if matches!(state, SyncState::Error) {
                tracing::error!(
                    target: "platform_wallet::e2e::spv",
                    "mn-list sync entered Error state"
                );
                return Err(FrameworkError::NotImplemented(
                    "spv::wait_for_mn_list_synced — mn-list entered Error state (see logs)",
                ));
            }
        }

        // Periodic "still waiting" snapshot at info level so
        // cold-cache runs show where the time is going.
        let now = Instant::now();
        if now >= next_progress_log {
            log_pipeline_snapshot(progress.as_ref(), start.elapsed(), effective_timeout);
            next_progress_log = now + PROGRESS_LOG_INTERVAL;
        }

        if now >= deadline {
            log_pipeline_snapshot(progress.as_ref(), start.elapsed(), effective_timeout);
            tracing::error!(
                target: "platform_wallet::e2e::spv",
                "timed out after {effective_timeout:?} waiting for mn-list sync"
            );
            return Err(FrameworkError::NotImplemented(
                "spv::wait_for_mn_list_synced — timed out (see logs)",
            ));
        }

        tokio::time::sleep(READINESS_POLL_INTERVAL).await;
    }
}

/// One-line info-level pipeline-snapshot log used by
/// [`wait_for_mn_list_synced`].
fn log_pipeline_snapshot(
    progress: Option<&dash_spv::sync::SyncProgress>,
    elapsed: Duration,
    timeout: Duration,
) {
    let Some(p) = progress else {
        tracing::info!(
            target: "platform_wallet::e2e::spv",
            ?elapsed,
            ?timeout,
            "still waiting for mn-list sync (no SPV progress yet)"
        );
        return;
    };

    let headers = p
        .headers()
        .ok()
        .map(|h| (h.state(), h.current_height(), h.target_height()));
    let filter_headers = p
        .filter_headers()
        .ok()
        .map(|f| (f.state(), f.current_height(), f.target_height()));
    let filters = p
        .filters()
        .ok()
        .map(|f| (f.state(), f.current_height(), f.target_height()));
    let mn = p
        .masternodes()
        .ok()
        .map(|m| (m.state(), m.current_height(), m.target_height()));

    tracing::info!(
        target: "platform_wallet::e2e::spv",
        ?elapsed,
        ?timeout,
        ?headers,
        ?filter_headers,
        ?filters,
        ?mn,
        "still waiting for mn-list sync"
    );
}

/// Build the SPV [`ClientConfig`] for `config.network`. Storage
/// under `<workdir>/spv-data`, full validation, bloom-filter
/// mempool tracking, and DAPI peers (extracted from `address_list`)
/// seeded with the effective P2P port — sticks to the SDK's live
/// endpoints to skip DNS-discovered peers that lack compact-block-filter
/// support.
fn build_client_config(
    config: &Config,
    address_list: &AddressList,
) -> FrameworkResult<ClientConfig> {
    let network = config.network;

    let storage_path = config.workdir_base.join("spv-data");
    std::fs::create_dir_all(&storage_path).map_err(|e| {
        tracing::error!(
            target: "platform_wallet::e2e::spv",
            "failed to create SPV storage dir {}: {e}",
            storage_path.display()
        );
        FrameworkError::NotImplemented(
            "spv::build_client_config — failed to create SPV storage dir (see logs)",
        )
    })?;

    let mut client_config = ClientConfig::new(network)
        .with_storage_path(storage_path)
        .with_validation_mode(ValidationMode::Full)
        .with_start_height(0)
        .with_mempool_tracking(MempoolStrategy::BloomFilter);

    seed_p2p_peers(&mut client_config, config, address_list);

    client_config.validate().map_err(|e| {
        tracing::error!(
            target: "platform_wallet::e2e::spv",
            "invalid SPV ClientConfig: {e}"
        );
        FrameworkError::NotImplemented(
            "spv::build_client_config — invalid SPV ClientConfig (see logs)",
        )
    })?;

    Ok(client_config)
}

/// Seed the SPV `ClientConfig` with P2P peers derived from the SDK's
/// live `AddressList`. Each address contributes its host IP paired
/// with [`Config::p2p_port`] (already resolved to override-or-default
/// at config construction time). Non-IP hostnames (which
/// `address.uri().host()` can return for DNS targets) fall through to
/// the SPV's own DNS discovery rather than being added as numeric
/// peers.
///
/// If `Config::p2p_port` is `None` (regtest / devnet without an
/// explicit override) no peers are seeded — the operator must supply
/// [`vars::P2P_PORT`](super::config::vars::P2P_PORT) for those.
fn seed_p2p_peers(client_config: &mut ClientConfig, config: &Config, address_list: &AddressList) {
    let Some(port) = config.p2p_port else {
        tracing::debug!(
            target: "platform_wallet::e2e::spv",
            network = ?config.network,
            "no SPV P2P port configured (neither {} nor a known network default); \
             skipping peer seeding — SPV will fall back to DNS discovery",
            super::config::vars::P2P_PORT,
        );
        return;
    };

    for address in address_list.get_live_addresses() {
        let Some(host) = address.uri().host() else {
            continue;
        };
        // SPV's `add_peer` takes a numeric `SocketAddr`; non-IP hosts
        // (DNS names) are left for the SPV client's discovery loop.
        if let Ok(ip) = host.parse::<IpAddr>() {
            client_config.add_peer(SocketAddr::new(ip, port));
        }
    }
}
