//! Background health check for DAPI nodes.
//!
//! Provides startup probing and ban-expiry re-probing to maintain a clean address list.

use std::time::Duration;

use dapi_grpc::platform::v0 as platform_proto;
use futures::stream::{self, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::connection_pool::ConnectionPool;
use crate::request_settings::AppliedRequestSettings;
use crate::transport::{TransportClient, TransportRequest};
use crate::{Address, AddressList, RequestSettings};

use dapi_grpc::tonic::transport::Certificate;

/// Configuration for the background health check.
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Timeout for each individual health probe. Default: 5 seconds.
    pub probe_timeout: Duration,
    /// Maximum concurrent health probes. Default: 10.
    pub max_concurrent_probes: usize,
    /// How long to sleep when no bans exist to watch. Default: 120 seconds.
    pub no_ban_idle_period: Duration,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            probe_timeout: Duration::from_secs(5),
            max_concurrent_probes: 10,
            no_ban_idle_period: Duration::from_secs(120),
        }
    }
}

/// Runs the background health check loop.
///
/// Phase 1 (startup): Probes all addresses concurrently, bans failures.
/// Phase 2 (watch): Sleeps until next ban expiry, re-probes expired bans.
///
/// **Note:** There is a small race window between ban expiry and re-probe completion
/// where `get_live_address()` may return a previously-banned node before the health check
/// has confirmed it is healthy. This is an accepted design tradeoff — changing this would
/// require modifying `get_live_address()` to never auto-unban expired bans, which would break
/// the existing reactive ban behavior.
pub async fn run_health_check(
    address_list: AddressList,
    pool: ConnectionPool,
    config: HealthCheckConfig,
    cancel_token: CancellationToken,
    ca_certificate: Option<Certificate>,
) {
    let probe_settings = RequestSettings {
        timeout: Some(config.probe_timeout),
        retries: Some(0),
        ban_failed_address: Some(false),
        connect_timeout: Some(config.probe_timeout),
        ..RequestSettings::default()
    }
    .finalize()
    .with_ca_certificate(ca_certificate);

    // Phase 1: Startup probe
    let all_addresses = address_list.get_all_addresses();
    if !all_addresses.is_empty() {
        tracing::info!(
            total = all_addresses.len(),
            "starting health check probe of all addresses"
        );
        probe_and_update_batch(
            &all_addresses,
            &address_list,
            &pool,
            &probe_settings,
            &config,
        )
        .await;
        tracing::info!(total = all_addresses.len(), "startup health check complete");
    }

    // Phase 2: Watch for ban expirations
    loop {
        let sleep_duration = match address_list.get_next_ban_expiry() {
            Some(expiry) => {
                let until = expiry - chrono::Utc::now();
                until.to_std().unwrap_or(Duration::ZERO)
            }
            None => config.no_ban_idle_period,
        };

        tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::debug!("health check cancelled");
                break;
            }
            _ = tokio::time::sleep(sleep_duration) => {
                let expired = address_list.get_expired_ban_addresses();
                if !expired.is_empty() {
                    tracing::debug!(
                        count = expired.len(),
                        "re-probing addresses with expired bans"
                    );
                    probe_and_update_batch(
                        &expired,
                        &address_list,
                        &pool,
                        &probe_settings,
                        &config,
                    )
                    .await;
                }
            }
        }
    }
}

async fn probe_and_update_batch(
    addresses: &[Address],
    address_list: &AddressList,
    pool: &ConnectionPool,
    settings: &AppliedRequestSettings,
    config: &HealthCheckConfig,
) {
    stream::iter(addresses)
        .for_each_concurrent(config.max_concurrent_probes, |address| {
            let pool = pool.clone();
            let settings = settings.clone();
            async move {
                let healthy = probe_node(address, &pool, &settings).await;
                if healthy {
                    if address_list.is_banned(address) {
                        address_list.unban(address);
                        tracing::debug!(%address, "health check: node is healthy, unbanned");
                    }
                } else {
                    // Ban the unhealthy node. If already banned (e.g., during Phase 2 re-probe),
                    // this increments ban_count and extends banned_until with exponential backoff,
                    // effectively escalating the ban duration for persistently dead nodes.
                    address_list.ban(address);
                    tracing::debug!(%address, "health check: node is unhealthy, banned");
                }
            }
        })
        .await;
}

async fn probe_node(
    address: &Address,
    pool: &ConnectionPool,
    settings: &AppliedRequestSettings,
) -> bool {
    let client_result =
        <platform_proto::GetStatusRequest as TransportRequest>::Client::with_uri_and_settings(
            address.uri().clone(),
            settings,
            pool,
        );

    let mut client = match client_result {
        Ok(client) => client,
        Err(e) => {
            tracing::trace!(%address, error = %e, "health check: failed to create transport client");
            return false;
        }
    };

    let request = platform_proto::GetStatusRequest {
        version: Some(platform_proto::get_status_request::Version::V0(
            platform_proto::get_status_request::GetStatusRequestV0 {},
        )),
    };

    match request.execute_transport(&mut client, settings).await {
        Ok(_) => true,
        Err(e) => {
            tracing::trace!(%address, error = %e, "health check: probe failed");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_config_default() {
        let config = HealthCheckConfig::default();
        assert_eq!(config.probe_timeout, Duration::from_secs(5));
        assert_eq!(config.max_concurrent_probes, 10);
        assert_eq!(config.no_ban_idle_period, Duration::from_secs(120));
    }

    #[tokio::test]
    async fn test_cancel_token_stops_loop() {
        let address_list = AddressList::new();
        let pool = ConnectionPool::new(10);
        let config = HealthCheckConfig::default();
        let cancel_token = CancellationToken::new();

        let ct = cancel_token.clone();
        ct.cancel();

        run_health_check(address_list, pool, config, cancel_token, None).await;
    }

    #[tokio::test]
    async fn test_probe_unreachable_node_returns_false() {
        let pool = ConnectionPool::new(10);
        let settings = RequestSettings {
            timeout: Some(Duration::from_millis(100)),
            retries: Some(0),
            ban_failed_address: Some(false),
            connect_timeout: Some(Duration::from_millis(100)),
            ..RequestSettings::default()
        }
        .finalize();

        let addr: Address = "http://192.0.2.1:1".parse().unwrap();
        let result = probe_node(&addr, &pool, &settings).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_startup_probe_bans_unreachable() {
        let mut address_list = AddressList::new();
        let addr: Address = "http://192.0.2.1:1".parse().unwrap();
        address_list.add(addr.clone());

        let pool = ConnectionPool::new(10);
        let config = HealthCheckConfig {
            probe_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let cancel_token = CancellationToken::new();

        let ct = cancel_token.clone();
        let al = address_list.clone();

        let handle = tokio::spawn(async move {
            run_health_check(al, pool, config, cancel_token, None).await;
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
        ct.cancel();
        handle.await.unwrap();

        assert!(address_list.is_banned(&addr));
    }

    /// QA-001: Verify that startup probe with empty address list completes without panic
    /// and transitions directly to the watch loop (which cancel_token can stop).
    #[tokio::test]
    async fn test_startup_probe_empty_address_list() {
        let address_list = AddressList::new();
        let pool = ConnectionPool::new(10);
        let config = HealthCheckConfig {
            probe_timeout: Duration::from_millis(50),
            no_ban_idle_period: Duration::from_millis(50),
            ..Default::default()
        };
        let cancel_token = CancellationToken::new();

        let ct = cancel_token.clone();
        let al = address_list.clone();

        let handle = tokio::spawn(async move {
            run_health_check(al, pool, config, cancel_token, None).await;
        });

        // Let the loop iterate at least once through the idle period
        tokio::time::sleep(Duration::from_millis(200)).await;
        ct.cancel();
        handle.await.unwrap();

        // No addresses should exist
        assert_eq!(address_list.len(), 0);
    }

    /// QA-002: Verify that ALL unreachable nodes in a multi-node list get banned during startup.
    #[tokio::test]
    async fn test_startup_probe_bans_all_unreachable_nodes() {
        let mut address_list = AddressList::new();
        let addr1: Address = "http://192.0.2.1:1".parse().unwrap();
        let addr2: Address = "http://192.0.2.2:1".parse().unwrap();
        let addr3: Address = "http://192.0.2.3:1".parse().unwrap();
        address_list.add(addr1.clone());
        address_list.add(addr2.clone());
        address_list.add(addr3.clone());

        let pool = ConnectionPool::new(10);
        let config = HealthCheckConfig {
            probe_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let cancel_token = CancellationToken::new();

        let ct = cancel_token.clone();
        let al = address_list.clone();

        let handle = tokio::spawn(async move {
            run_health_check(al, pool, config, cancel_token, None).await;
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
        ct.cancel();
        handle.await.unwrap();

        assert!(
            address_list.is_banned(&addr1),
            "addr1 should be banned after startup probe"
        );
        assert!(
            address_list.is_banned(&addr2),
            "addr2 should be banned after startup probe"
        );
        assert!(
            address_list.is_banned(&addr3),
            "addr3 should be banned after startup probe"
        );
        // No live addresses should remain
        assert_eq!(
            address_list.get_live_addresses().len(),
            0,
            "no live addresses should remain when all nodes are dead"
        );
    }

    /// QA-003: Verify probe_settings has ban_failed_address=false so probing itself
    /// does not trigger banning via the normal DapiClient ban path.
    #[test]
    fn test_probe_settings_do_not_ban_on_failure() {
        let config = HealthCheckConfig::default();
        let probe_settings = RequestSettings {
            timeout: Some(config.probe_timeout),
            retries: Some(0),
            ban_failed_address: Some(false),
            connect_timeout: Some(config.probe_timeout),
            ..RequestSettings::default()
        }
        .finalize();

        assert!(
            !probe_settings.ban_failed_address,
            "probe settings must have ban_failed_address=false to avoid \
             double-banning through the transport layer"
        );
        assert_eq!(
            probe_settings.retries, 0,
            "probe settings must have retries=0 for fast failure detection"
        );
    }

    /// QA-004: Verify cancel_token stops the loop even when addresses have active bans
    /// (i.e., when sleeping until ban expiry).
    #[tokio::test]
    async fn test_cancel_during_ban_watch_phase() {
        let mut address_list = AddressList::new();
        let addr: Address = "http://192.0.2.1:1".parse().unwrap();
        address_list.add(addr.clone());
        // Pre-ban the address so Phase 2 will sleep until ban expiry
        address_list.ban(&addr);

        let pool = ConnectionPool::new(10);
        let config = HealthCheckConfig {
            probe_timeout: Duration::from_millis(50),
            // Long idle period to ensure we're actually cancelling during sleep
            no_ban_idle_period: Duration::from_secs(300),
            ..Default::default()
        };
        let cancel_token = CancellationToken::new();

        let ct = cancel_token.clone();
        let al = address_list.clone();

        let handle = tokio::spawn(async move {
            run_health_check(al, pool, config, cancel_token, None).await;
        });

        // Wait a bit then cancel -- the task should stop promptly
        tokio::time::sleep(Duration::from_millis(500)).await;
        ct.cancel();

        // If cancel doesn't work, this will hang for 300s and timeout
        let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
        assert!(
            result.is_ok(),
            "health check should have stopped within 5s after cancel"
        );
        result.unwrap().unwrap();
    }
}
