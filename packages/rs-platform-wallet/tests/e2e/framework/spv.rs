//! SPV runtime startup and readiness wait.
//!
//! **NOTE: currently disabled in favor of
//! `rs_sdk_trusted_context_provider::TrustedHttpContextProvider`
//! — see `harness.rs` for the commented-out wiring. Re-enable
//! when SPV cold-start is stable (Task #15). The module remains
//! compilable so re-enablement is a single-block uncomment.**
//!
//! [`start_spv`] kicks off the SPV client via
//! [`platform_wallet::SpvRuntime::spawn_in_background`] using a
//! [`ClientConfig`] derived from the e2e [`Config`]. Storage is
//! anchored under the harness workdir slot (the manager / runtime
//! itself is constructed elsewhere — Wave 4 wires it together).
//!
//! [`wait_for_mn_list_synced`] polls
//! [`SpvRuntime::sync_progress`] until the masternode-list manager
//! reports `SyncState::Synced` (i.e. it has caught up to the block
//! header tip). That's the readiness signal the
//! [`super::context_provider::SpvContextProvider`] needs before it
//! can answer quorum public-key lookups for proof verification.
//!
//! The harness passes a 180s deadline that's only sufficient on a
//! warm SPV cache; for cold-cache runs we lift the effective timeout
//! to a [`COLD_CACHE_TIMEOUT_FLOOR`] (600s) so the live e2e doesn't
//! flake while still surfacing a real hang inside that envelope.
//! Periodic info-level progress logs emitted every
//! [`PROGRESS_LOG_INTERVAL`] make the wait debuggable without having
//! to re-run with `RUST_LOG=debug`.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dash_spv::client::config::MempoolStrategy;
use dash_spv::sync::{ProgressPercentage, SyncState};
use dash_spv::types::ValidationMode;
use dash_spv::ClientConfig;
use dashcore::Network;
use platform_wallet::{changeset::PlatformWalletPersistence, PlatformWalletManager, SpvRuntime};

use super::config::Config;
use super::sdk::TESTNET_DAPI_ADDRESSES;
use super::{FrameworkError, FrameworkResult};

/// P2P port for testnet seed peers (matches `tests/spv_sync.rs`).
const TESTNET_P2P_PORT: u16 = 19999;

/// Polling interval used by [`wait_for_mn_list_synced`].
const READINESS_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Wall-clock floor for [`wait_for_mn_list_synced`] timeouts. The
/// harness's caller-supplied `SPV_READY_TIMEOUT` (180s) is fine on a
/// warm SPV cache but provably too short on a cold cache against live
/// testnet (~1.4M+ blocks of headers, ~3.6M filters, then a full
/// QRInfo + non-rotating quorum verification). `tests/spv_sync.rs`
/// uses a 600s timeout for the same cold-cache scenario, so we lift
/// the effective timeout to that floor here. If callers pass a larger
/// timeout (e.g. for explicitly cold runs) we honor it as-is.
const COLD_CACHE_TIMEOUT_FLOOR: Duration = Duration::from_secs(600);

/// Period for "still waiting" progress logs while
/// [`wait_for_mn_list_synced`] polls. Picked to be short enough that
/// CI tail logs surface meaningful state every ~30s, long enough to
/// keep the noise level reasonable on a successful run.
const PROGRESS_LOG_INTERVAL: Duration = Duration::from_secs(30);

/// Start the SPV client backing the harness's
/// [`PlatformWalletManager`].
///
/// Builds a [`ClientConfig`] for the configured network, anchors the
/// SPV storage under `config.workdir_base.join("spv-data")`, and
/// hands the config off to
/// [`SpvRuntime::spawn_in_background`]. The runtime stores its own
/// cancellation token internally; the caller can shut it down later
/// via [`SpvRuntime::stop`].
///
/// The returned `Arc<SpvRuntime>` is the same handle exposed by
/// [`PlatformWalletManager::spv_arc`] — returning it explicitly here
/// keeps the call-site of [`super::context_provider::SpvContextProvider`]
/// independent of the manager's full type signature.
pub async fn start_spv<P>(
    manager: &Arc<PlatformWalletManager<P>>,
    config: &Config,
) -> FrameworkResult<Arc<SpvRuntime>>
where
    P: PlatformWalletPersistence + 'static,
{
    let spv = manager.spv_arc();
    let client_config = build_client_config(config)?;

    spv.spawn_in_background(client_config);
    tracing::info!(
        target: "platform_wallet::e2e::spv",
        network = %config.network,
        "SPV runtime spawned in background"
    );

    Ok(spv)
}

/// Block until the SPV masternode-list manager reports `Synced`, or
/// the effective timeout elapses.
///
/// Polls [`SpvRuntime::sync_progress`] every
/// [`READINESS_POLL_INTERVAL`]. While the masternodes manager is
/// still in `WaitForEvents` / `WaitingForConnections` (i.e.
/// `sync_progress.masternodes()` is either `None` or has no progress
/// entry) we keep waiting — the SPV client only attaches the
/// progress entry once the masternode sub-system has bootstrapped.
///
/// Effective timeout is `timeout.max(COLD_CACHE_TIMEOUT_FLOOR)`: the
/// harness passes a 180s deadline that's only sufficient on a warm
/// cache; against a cold testnet cache the full pipeline (headers →
/// filters → QRInfo → quorum verification) consistently runs longer
/// (`tests/spv_sync.rs` uses 600s for the same scenario), so we lift
/// the floor here rather than make every cold run flake. Larger
/// caller-supplied timeouts pass through unchanged.
///
/// While polling, every [`PROGRESS_LOG_INTERVAL`] we emit an `info`
/// log summarising the current masternode-list state so timeouts are
/// debuggable without re-running with `RUST_LOG=debug`.
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

        // Periodic "still waiting" log. Snapshots whatever stage we're
        // currently at — including the headers / filters managers —
        // so a cold-cache run shows where the time is going even at
        // info level.
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

/// Log a one-line summary of the SPV pipeline snapshot at info level.
///
/// Invoked by [`wait_for_mn_list_synced`] every
/// [`PROGRESS_LOG_INTERVAL`] (and once on timeout) to make cold-cache
/// runs debuggable from default-level logs.
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

/// Build the SPV [`ClientConfig`] for the configured network.
///
/// Uses [`ClientConfig::testnet`] / [`ClientConfig::regtest`] /
/// [`ClientConfig::new`] depending on selector, then layers on:
/// per-process storage path (under the workdir slot), full
/// validation, mempool tracking via bloom filters, and — for testnet
/// — the well-known DAPI peers as P2P seeds (matches the precedent
/// from `tests/spv_sync.rs`, which avoids slow DNS-discovered peers
/// without compact block filter support).
fn build_client_config(config: &Config) -> FrameworkResult<ClientConfig> {
    let network = match config.network.trim().to_ascii_lowercase().as_str() {
        "" | "testnet" => Network::Testnet,
        "mainnet" => Network::Mainnet,
        "devnet" => Network::Devnet,
        "regtest" | "local" => Network::Regtest,
        other => {
            tracing::error!(
                target: "platform_wallet::e2e::spv",
                "unknown network selector {other:?} (expected testnet/mainnet/devnet/regtest/local)"
            );
            return Err(FrameworkError::NotImplemented(
                "spv::build_client_config — unknown network selector (see logs)",
            ));
        }
    };

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

    seed_p2p_peers(&mut client_config, config, network);

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

/// Seed the SPV config with hard-coded P2P peers when running on
/// testnet without explicit overrides.
///
/// Mirrors `tests/spv_sync.rs`: extract the hostnames from the
/// configured (or default) DAPI URLs, parse them as IP addresses,
/// and add them on the testnet P2P port. Hostnames that don't parse
/// as IPs are skipped — DNS-based DAPI URLs are best left to the
/// SPV's own DNS seed discovery for header sync.
fn seed_p2p_peers(client_config: &mut ClientConfig, config: &Config, network: Network) {
    if !matches!(network, Network::Testnet) {
        return;
    }

    let addresses: Vec<&str> = if config.dapi_addresses.is_empty() {
        TESTNET_DAPI_ADDRESSES.to_vec()
    } else {
        config.dapi_addresses.iter().map(String::as_str).collect()
    };

    for addr in addresses {
        let host = addr
            .strip_prefix("https://")
            .or_else(|| addr.strip_prefix("http://"))
            .unwrap_or(addr);
        let host_only = host.split(':').next().unwrap_or(host);
        if let Ok(ip) = host_only.parse::<IpAddr>() {
            client_config.add_peer(std::net::SocketAddr::new(ip, TESTNET_P2P_PORT));
        }
    }
}
