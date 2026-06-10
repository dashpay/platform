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
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dash_sdk::dapi_client::AddressList;
use dash_spv::client::config::MempoolStrategy;
use dash_spv::network::NetworkEvent;
use dash_spv::sync::{ManagerIdentifier, ProgressPercentage, SyncEvent, SyncState};
use dash_spv::types::ValidationMode;
use dash_spv::{ClientConfig, DevnetConfig};
use dashcore::sml::llmq_type::LlmqDevnetParams;
use dashcore::Network;
use platform_wallet::events::{EventHandler, PlatformEventHandler, WalletEvent};
use platform_wallet::{changeset::PlatformWalletPersistence, PlatformWalletManager, SpvRuntime};
use tokio::sync::broadcast;

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

/// Mn-list-stall heuristic: if the mn-list snapshot does not change
/// (state + current_height + target_height all identical) for this
/// long while we're still waiting, dash-spv has almost certainly
/// given up internally — fail fast instead of burning the cold-cache
/// floor. Backstop for the event-driven `ManagerError` path: if
/// dash-spv ever stops emitting that event for the same root cause,
/// we still bail in well under the 600s floor. 120s ≈ 2 min ≈
/// roughly the testnet block interval, so a single missed block tick
/// won't trip it.
const MN_LIST_STALL_THRESHOLD: Duration = Duration::from_secs(120);

/// Spawn the SPV client backing the harness's
/// [`PlatformWalletManager`]. Storage is anchored under
/// `<workdir>/spv-data` where `workdir` is the slot the harness
/// already locked via [`super::workdir::pick_available_workdir`] —
/// concurrent processes get distinct slots and therefore distinct
/// SPV stores, so RocksDB never sees cross-process contention.
/// Returns the same handle as [`PlatformWalletManager::spv_arc`];
/// shut it down via [`SpvRuntime::stop`].
///
/// `address_list` is the SDK's live DAPI address list (typically
/// `sdk.address_list()`). P2P peers are seeded from those same
/// IPs with the effective P2P port — keeping a single source of
/// truth instead of forking from `dash_network_seeds` and risking
/// drift between SDK-tracked and SPV-tracked endpoints.
pub async fn start_spv<P>(
    manager: &Arc<PlatformWalletManager<P>>,
    config: &Config,
    workdir: &Path,
    address_list: &AddressList,
) -> FrameworkResult<Arc<SpvRuntime>>
where
    P: PlatformWalletPersistence + 'static,
{
    let spv = manager.spv_arc();
    let client_config = build_client_config(config, workdir, address_list)?;

    // Apply the devnet genesis override before spawn so the runtime's
    // pre-seed (which sidesteps dash-spv's missing devnet genesis) uses
    // it. Empty override = the `dashcore` built-in; the runtime ignores
    // it entirely on non-devnet networks.
    if config.network == Network::Devnet && !config.devnet_genesis.is_empty() {
        spv.set_devnet_genesis_override(config.devnet_genesis.clone());
    }

    spv.spawn_in_background(client_config);
    tracing::info!(
        target: "platform_wallet::e2e::spv",
        network = ?config.network,
        "SPV runtime spawned in background"
    );

    Ok(spv)
}

/// Block until the SPV mn-list manager reports `Synced`, or one of
/// three failure conditions trips:
///
/// 1. **Engine event** — dash-spv emits a
///    [`SyncEvent::ManagerError`] for the masternode manager. The
///    classic example is the QRInfo retry loop hard-capping at 3
///    attempts (`Required rotated chain lock sig at h - 0 not
///    present`); the engine then stops trying to advance mn-list. We
///    bail with a sharply-targeted error message rather than burn
///    the full cold-cache floor.
/// 2. **Stall heuristic** — the mn-list snapshot has not advanced
///    (same state + current_height + target_height) for
///    [`MN_LIST_STALL_THRESHOLD`]. Backstop for cases where the
///    engine never emits a `ManagerError` (e.g. silent retry loop).
/// 3. **Hard timeout** — the effective timeout
///    (`timeout.max(COLD_CACHE_TIMEOUT_FLOOR)`) elapses.
///
/// Polls every [`READINESS_POLL_INTERVAL`] and emits an info-level
/// pipeline snapshot every [`PROGRESS_LOG_INTERVAL`] so cold-cache
/// hangs are debuggable from default-level logs.
pub async fn wait_for_mn_list_synced(
    spv: &SpvRuntime,
    mn_list_observer: &MnListErrorObserver,
    timeout: Duration,
) -> FrameworkResult<()> {
    let effective_timeout = timeout.max(COLD_CACHE_TIMEOUT_FLOOR);
    if effective_timeout != timeout {
        tracing::info!(
            target: "platform_wallet::e2e::spv",
            requested = ?timeout,
            effective = ?effective_timeout,
            "raising mn-list-sync timeout to cold-cache floor"
        );
    }

    // Subscribe a fresh receiver to dash-spv's
    // `SyncEvent::ManagerError` stream via the constructor-injected
    // `MnListErrorObserver`. dash-spv emits one Masternode
    // `ManagerError` per failed inbound message, so a persistent
    // stall bursts many errors fast — both a single error and a
    // ring-overflow (`Lagged`) are treated as stall signals so the
    // wait fast-fails in O(ms) instead of burning the cold-cache
    // floor. The receiver is dropped when this call returns — no leak.
    let mut err_rx = mn_list_observer.subscribe();

    let start = Instant::now();
    let deadline = start + effective_timeout;
    let mut last_height: Option<u32> = None;
    let mut last_state: Option<SyncState> = None;
    let mut last_target: Option<u32> = None;
    let mut last_progress_at = start;
    let mut next_progress_log = start + PROGRESS_LOG_INTERVAL;

    loop {
        // Race the engine error stream against the next poll tick.
        // `biased` so a queued error wins over a coincident sleep
        // expiry — surfaces the engine signal at the earliest tick.
        tokio::select! {
            biased;
            maybe_err = err_rx.recv() => {
                match maybe_err {
                    Ok(err) => {
                        tracing::error!(
                            target: "platform_wallet::e2e::spv",
                            error = %err,
                            elapsed = ?start.elapsed(),
                            "dash-spv reported ManagerError before mn-list synced"
                        );
                        return Err(FrameworkError::Spv(format!(
                            "dash-spv reported ManagerError before mn-list synced: {err}. \
                             Likely a stale workdir / testnet ChainLock cycle issue. \
                             Try wiping spv-data/ and retry, or wait 10-20 min for the \
                             next testnet ChainLock cycle."
                        )));
                    }
                    // Ring overflow: dash-spv emits one error per
                    // failed inbound message, so a lagged burst is
                    // itself definitive proof of a stall — fail fast.
                    Err(broadcast::error::RecvError::Lagged(dropped)) => {
                        tracing::error!(
                            target: "platform_wallet::e2e::spv",
                            dropped,
                            elapsed = ?start.elapsed(),
                            "dash-spv mn-list ManagerError burst overflowed the \
                             observer ring before mn-list synced"
                        );
                        return Err(FrameworkError::Spv(format!(
                            "dash-spv reported a burst of mn-list ManagerErrors \
                             (>{dropped} dropped) before mn-list synced. \
                             Likely a stale workdir / testnet ChainLock cycle issue. \
                             Try wiping spv-data/ and retry, or wait 10-20 min for the \
                             next testnet ChainLock cycle."
                        )));
                    }
                    // Sender gone (observer outlives every wait via the
                    // manager, so not expected) — not a stall; poll on
                    // so the heuristic / hard timeout still applies.
                    Err(broadcast::error::RecvError::Closed) => {}
                }
            }
            _ = tokio::time::sleep(READINESS_POLL_INTERVAL) => {}
        }

        let progress = spv.sync_progress().await;
        let mn_snapshot = progress
            .as_ref()
            .and_then(|p| p.masternodes().ok().cloned());

        if let Some(mn) = mn_snapshot.as_ref() {
            let height = mn.current_height();
            let state = mn.state();
            let target = mn.target_height();
            let advanced = Some(height) != last_height
                || Some(state) != last_state
                || Some(target) != last_target;
            if advanced {
                tracing::debug!(
                    target: "platform_wallet::e2e::spv",
                    state = ?state,
                    current_height = height,
                    target_height = target,
                    elapsed = ?start.elapsed(),
                    "mn-list sync progress"
                );
                last_height = Some(height);
                last_state = Some(state);
                last_target = Some(target);
                last_progress_at = Instant::now();
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
                return Err(FrameworkError::Spv(
                    "wait_for_mn_list_synced: mn-list entered Error state".to_string(),
                ));
            }

            // Heuristic: no forward progress for
            // `MN_LIST_STALL_THRESHOLD` while still in a non-terminal
            // state ⇒ engine is stuck. Bail with the same operator
            // hint as the event path so the user sees one consistent
            // remediation.
            let stalled_for = last_progress_at.elapsed();
            if stalled_for >= MN_LIST_STALL_THRESHOLD {
                log_pipeline_snapshot(progress.as_ref(), start.elapsed(), effective_timeout);
                tracing::error!(
                    target: "platform_wallet::e2e::spv",
                    state = ?state,
                    current_height = height,
                    target_height = target,
                    stalled_for = ?stalled_for,
                    "mn-list sync made no forward progress for stall threshold; \
                     engine has likely given up internally"
                );
                return Err(FrameworkError::Spv(format!(
                    "wait_for_mn_list_synced: mn-list made no forward progress for \
                     {stalled_for:?} (state={state:?}, current_height={height}, \
                     target_height={target}). dash-spv has likely given up \
                     internally without surfacing a ManagerError. \
                     Try wiping spv-data/ and retry, or wait 10-20 min for the \
                     next testnet ChainLock cycle."
                )));
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
            return Err(FrameworkError::Spv(format!(
                "wait_for_mn_list_synced: timed out after {effective_timeout:?}"
            )));
        }
    }
}

/// Broadcast capacity for [`MnListErrorObserver`]. dash-spv bursts one
/// Masternode `ManagerError` per failed inbound message, so the ring
/// can overflow during a stall; that `Lagged` is itself treated as a
/// stall signal in [`wait_for_mn_list_synced`], so a modest size only
/// needs to absorb non-stall transients.
const MN_LIST_ERROR_CHANNEL_CAP: usize = 16;

/// Single-purpose [`PlatformEventHandler`] that forwards
/// [`SyncEvent::ManagerError`] events scoped to
/// [`ManagerIdentifier::Masternode`] onto a broadcast channel. Built
/// once during harness init and threaded into
/// [`PlatformWalletManager::new`] as one of its `app_handlers`; each
/// [`wait_for_mn_list_synced`] call [`subscribe`](Self::subscribe)s a
/// fresh receiver so it escapes the cold-cache floor as soon as
/// dash-spv signals a fatal manager error during that wait.
///
/// All other event variants are ignored — this is *not* a substitute
/// for [`super::wait_hub::WaitEventHub`].
pub struct MnListErrorObserver {
    tx: broadcast::Sender<String>,
}

impl MnListErrorObserver {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(MN_LIST_ERROR_CHANNEL_CAP);
        Self { tx }
    }

    /// A fresh receiver scoped to the caller's wait window. Only
    /// observes errors emitted after this call returns, matching the
    /// per-wait semantics the loop relies on.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }
}

impl Default for MnListErrorObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for MnListErrorObserver {
    fn on_sync_event(&self, event: &SyncEvent) {
        if let SyncEvent::ManagerError { manager, error } = event {
            if matches!(manager, ManagerIdentifier::Masternode) {
                // Best-effort: no live subscriber (between waits) is
                // fine, the next wait subscribes its own receiver.
                let _ = self.tx.send(format!("Masternode manager error: {error}"));
            }
        }
    }

    fn on_network_event(&self, _event: &NetworkEvent) {}
    fn on_progress(&self, _progress: &dash_spv::sync::SyncProgress) {}
    fn on_wallet_event(&self, _event: &WalletEvent) {}
    fn on_error(&self, _error: &str) {}
}

impl PlatformEventHandler for MnListErrorObserver {}

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
/// under `<workdir>/spv-data` (the slot-locked dir, NOT
/// `workdir_base`), full validation, bloom-filter mempool tracking,
/// and DAPI peers (extracted from `address_list`) seeded with the
/// effective P2P port — sticks to the SDK's live endpoints to skip
/// DNS-discovered peers that lack compact-block-filter support.
fn build_client_config(
    config: &Config,
    workdir: &Path,
    address_list: &AddressList,
) -> FrameworkResult<ClientConfig> {
    let network = config.network;

    let storage_path = workdir.join("spv-data");
    std::fs::create_dir_all(&storage_path).map_err(|e| {
        tracing::error!(
            target: "platform_wallet::e2e::spv",
            "failed to create SPV storage dir {}: {e}",
            storage_path.display()
        );
        FrameworkError::Spv(format!(
            "failed to create SPV storage dir {}: {e}",
            storage_path.display()
        ))
    })?;

    let mut client_config = ClientConfig::new(network)
        .with_storage_path(storage_path)
        .with_validation_mode(ValidationMode::Full)
        .with_start_height(0)
        .with_mempool_tracking(MempoolStrategy::BloomFilter);

    seed_p2p_peers(&mut client_config, config, address_list);

    if network == Network::Devnet {
        // Mirrors packages/rs-platform-wallet-ffi/src/spv.rs:306-358 (devnet handshake + LLMQ).
        // Dash Core devnet peers drop any inbound connection whose user agent
        // lacks `devnet.devnet-<name>`; rebuild it in the FFI's exact
        // `/<base>(devnet.devnet-<name>)/` shape.
        let base = client_config
            .user_agent
            .clone()
            .unwrap_or_else(|| format!("platform-wallet-e2e:{}", env!("CARGO_PKG_VERSION")));
        client_config.user_agent = Some(format!("/{base}(devnet.devnet-{})/", config.devnet_name));

        let mut devnet = DevnetConfig::new(config.devnet_name.clone());
        if config.devnet_llmq_size > 0 {
            devnet.llmq_params = Some(LlmqDevnetParams {
                size: config.devnet_llmq_size,
                threshold: config.devnet_llmq_threshold,
            });
        }
        client_config.devnet = Some(devnet);
    }

    client_config.validate().map_err(|e| {
        tracing::error!(
            target: "platform_wallet::e2e::spv",
            "invalid SPV ClientConfig: {e}"
        );
        FrameworkError::Spv(format!("invalid SPV ClientConfig: {e}"))
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
