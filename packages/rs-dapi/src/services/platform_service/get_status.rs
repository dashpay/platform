use dapi_grpc::platform::v0::{
    GetStatusRequest, GetStatusResponse,
    get_status_response::get_status_response_v0,
    get_status_response::{self, GetStatusResponseV0},
};
use dapi_grpc::tonic::{Request, Response, Status};
use tracing::debug;

use crate::clients::{
    drive_client::DriveStatusResponse,
    tenderdash_client::{NetInfoResponse, TenderdashStatusResponse},
};

// The struct is defined in the parent platform_service.rs module
use crate::services::platform_service::PlatformServiceImpl;

/// Captures upstream health information when building the Platform status response.
#[derive(Debug, Clone, Default)]
pub struct PlatformStatusHealth {
    pub drive_error: Option<String>,
    pub tenderdash_status_error: Option<String>,
    pub tenderdash_netinfo_error: Option<String>,
}

impl PlatformStatusHealth {
    #[inline]
    pub fn is_drive_healthy(&self) -> bool {
        self.drive_error.is_none()
    }

    #[inline]
    pub fn is_tenderdash_healthy(&self) -> bool {
        self.tenderdash_status_error.is_none()
    }

    #[inline]
    pub fn is_netinfo_healthy(&self) -> bool {
        self.tenderdash_netinfo_error.is_none()
    }

    #[inline]
    pub fn is_healthy(&self) -> bool {
        self.is_drive_healthy() && self.is_tenderdash_healthy() && self.is_netinfo_healthy()
    }
}

impl PlatformServiceImpl {
    /// Handle the Platform `getStatus` request with caching and cache warming logic.
    pub async fn get_status_impl(
        &self,
        request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        use crate::cache::make_cache_key;
        use std::time::Duration;

        // Build cache key and try TTL cache first (3 minutes)
        let key = make_cache_key("get_status", request.get_ref());
        if let Some(mut cached) = self
            .platform_cache
            .get_with_ttl::<GetStatusResponse>(&key, Duration::from_secs(180))
        {
            // Refresh local time to current instant like JS implementation
            if let Some(get_status_response::Version::V0(ref mut v0)) = cached.version
                && let Some(ref mut time) = v0.time
            {
                time.local = chrono::Utc::now().timestamp() as u64;
            }
            return Ok(Response::new(cached));
        }

        // Build fresh response and cache it
        match self.build_status_response_with_health().await {
            Ok((response, _health)) => {
                self.platform_cache.put(key, &response);
                Ok(Response::new(response))
            }
            Err(status) => Err(status),
        }
    }

    /// Gather Drive and Tenderdash status information and compose the unified response.
    pub(crate) async fn build_status_response_with_health(
        &self,
    ) -> Result<(GetStatusResponse, PlatformStatusHealth), Status> {
        let mut health = PlatformStatusHealth::default();

        // Prepare request for Drive
        let drive_request = GetStatusRequest {
            version: Some(dapi_grpc::platform::v0::get_status_request::Version::V0(
                dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0 {},
            )),
        };

        // Fetch data from Drive and Tenderdash concurrently
        let (drive_result, tenderdash_status_result, tenderdash_netinfo_result) = tokio::join!(
            self.drive_client.get_drive_status(&drive_request),
            self.tenderdash_client.status(),
            self.tenderdash_client.net_info()
        );

        // Handle potential errors with proper logging
        let drive_status = match drive_result {
            Ok(status) => status,
            Err(e) => {
                debug!(error = ?e, "Failed to fetch Drive status - technical failure, using defaults");
                health.drive_error = Some(e.to_string());
                DriveStatusResponse::default()
            }
        };

        let tenderdash_status = match tenderdash_status_result {
            Ok(status) => status,
            Err(e) => {
                debug!(error = ?e, "Failed to fetch Tenderdash status - technical failure, using defaults");
                health.tenderdash_status_error = Some(e.to_string());
                TenderdashStatusResponse::default()
            }
        };

        let tenderdash_netinfo = match tenderdash_netinfo_result {
            Ok(netinfo) => netinfo,
            Err(e) => {
                debug!(error = ?e, "Failed to fetch Tenderdash netinfo - technical failure, using defaults");
                health.tenderdash_netinfo_error = Some(e.to_string());
                NetInfoResponse::default()
            }
        };

        // Use standalone functions to create the response
        let response = build_status_response(drive_status, tenderdash_status, tenderdash_netinfo)?;
        Ok((response, health))
    }
}

// Status building functions

/// Assemble the full gRPC response from Drive and Tenderdash status snapshots.
fn build_status_response(
    drive_status: DriveStatusResponse,
    tenderdash_status: TenderdashStatusResponse,
    tenderdash_netinfo: NetInfoResponse,
) -> Result<GetStatusResponse, Status> {
    let v0 = GetStatusResponseV0 {
        version: Some(build_version_info(&drive_status, &tenderdash_status)),
        node: build_node_info(&tenderdash_status),
        chain: build_chain_info(&drive_status, &tenderdash_status),
        state_sync: build_state_sync_info(&tenderdash_status),
        network: build_network_info(&tenderdash_status, &tenderdash_netinfo),
        time: Some(build_time_info(&drive_status)),
    };

    let response = GetStatusResponse {
        version: Some(get_status_response::Version::V0(v0)),
    };

    Ok(response)
}

/// Populate version metadata including protocol and software versions.
fn build_version_info(
    drive_status: &DriveStatusResponse,
    tenderdash_status: &TenderdashStatusResponse,
) -> get_status_response_v0::Version {
    let mut version = get_status_response_v0::Version::default();

    // Protocol version
    let mut protocol = get_status_response_v0::version::Protocol::default();

    // Tenderdash protocol version
    if let Some(node_info) = &tenderdash_status.node_info
        && let Some(protocol_version) = &node_info.protocol_version
    {
        let mut tenderdash_protocol =
            get_status_response_v0::version::protocol::Tenderdash::default();

        if let Some(block) = &protocol_version.block {
            tenderdash_protocol.block = block.parse().unwrap_or(0);
        }
        if let Some(p2p) = &protocol_version.p2p {
            tenderdash_protocol.p2p = p2p.parse().unwrap_or(0);
        }

        protocol.tenderdash = Some(tenderdash_protocol);
    }

    // Drive protocol version
    if let Some(version_info) = &drive_status.version
        && let Some(protocol_info) = &version_info.protocol
        && let Some(drive_protocol) = &protocol_info.drive
    {
        let drive_protocol_version = get_status_response_v0::version::protocol::Drive {
            current: drive_protocol.current.unwrap_or(0) as u32,
            latest: drive_protocol.latest.unwrap_or(0) as u32,
        };

        protocol.drive = Some(drive_protocol_version);
    }

    version.protocol = Some(protocol);

    // Software version
    let drive_version = drive_status
        .version
        .as_ref()
        .and_then(|v| v.software.as_ref())
        .and_then(|s| s.drive.as_ref())
        .cloned();

    let tenderdash_version = tenderdash_status
        .node_info
        .as_ref()
        .and_then(|n| n.version.as_ref())
        .cloned();

    let software = get_status_response_v0::version::Software {
        dapi: env!("CARGO_PKG_VERSION").to_string(),
        drive: drive_version,
        tenderdash: tenderdash_version,
    };

    version.software = Some(software);
    version
}

/// Build node identification data from Tenderdash status, decoding hex identifiers.
fn build_node_info(
    tenderdash_status: &TenderdashStatusResponse,
) -> Option<get_status_response_v0::Node> {
    if let Some(node_info) = &tenderdash_status.node_info {
        let mut node = get_status_response_v0::Node::default();

        if let Some(id) = &node_info.id
            && let Ok(id_bytes) = hex::decode(id)
        {
            node.id = id_bytes;
        }

        if let Some(pro_tx_hash) = &node_info.pro_tx_hash
            && let Ok(pro_tx_hash_bytes) = hex::decode(pro_tx_hash)
        {
            node.pro_tx_hash = Some(pro_tx_hash_bytes);
        }

        Some(node)
    } else {
        None
    }
}

/// Construct chain synchronization information combining Drive and Tenderdash fields.
fn build_chain_info(
    drive_status: &DriveStatusResponse,
    tenderdash_status: &TenderdashStatusResponse,
) -> Option<get_status_response_v0::Chain> {
    if let Some(sync_info) = &tenderdash_status.sync_info {
        let catching_up = sync_info.catching_up.unwrap_or(false);

        let latest_block_hash = sync_info
            .latest_block_hash
            .as_ref()
            .and_then(|hash| hex::decode(hash).ok())
            .unwrap_or_default();

        let latest_app_hash = sync_info
            .latest_app_hash
            .as_ref()
            .and_then(|hash| hex::decode(hash).ok())
            .unwrap_or_default();

        let latest_block_height = sync_info
            .latest_block_height
            .as_ref()
            .and_then(|h| h.parse().ok())
            .unwrap_or(0);

        let earliest_block_hash = sync_info
            .earliest_block_hash
            .as_ref()
            .and_then(|hash| hex::decode(hash).ok())
            .unwrap_or_default();

        let earliest_app_hash = sync_info
            .earliest_app_hash
            .as_ref()
            .and_then(|hash| hex::decode(hash).ok())
            .unwrap_or_default();

        let earliest_block_height = sync_info
            .earliest_block_height
            .as_ref()
            .and_then(|h| h.parse().ok())
            .unwrap_or(0);

        let max_peer_block_height = sync_info
            .max_peer_block_height
            .as_ref()
            .and_then(|h| h.parse().ok())
            .unwrap_or(0);

        let core_chain_locked_height = drive_status
            .chain
            .as_ref()
            .and_then(|c| c.core_chain_locked_height)
            .map(|h| h as u32);

        let chain = get_status_response_v0::Chain {
            catching_up,
            latest_block_hash,
            latest_app_hash,
            latest_block_height,
            earliest_block_hash,
            earliest_app_hash,
            earliest_block_height,
            max_peer_block_height,
            core_chain_locked_height,
        };

        Some(chain)
    } else {
        None
    }
}

/// Produce state sync metrics derived from Tenderdash status response.
fn build_state_sync_info(
    tenderdash_status: &TenderdashStatusResponse,
) -> Option<get_status_response_v0::StateSync> {
    if let Some(sync_info) = &tenderdash_status.sync_info {
        let parse_or_default = |opt_str: Option<&String>| -> u64 {
            opt_str.unwrap_or(&"0".to_string()).parse().unwrap_or(0)
        };

        let state_sync = get_status_response_v0::StateSync {
            total_synced_time: parse_or_default(sync_info.total_synced_time.as_ref()),
            remaining_time: parse_or_default(sync_info.remaining_time.as_ref()),
            total_snapshots: parse_or_default(sync_info.total_snapshots.as_ref()) as u32,
            chunk_process_avg_time: parse_or_default(sync_info.chunk_process_avg_time.as_ref()),
            snapshot_height: parse_or_default(sync_info.snapshot_height.as_ref()),
            snapshot_chunks_count: parse_or_default(sync_info.snapshot_chunks_count.as_ref()),
            backfilled_blocks: parse_or_default(sync_info.backfilled_blocks.as_ref()),
            backfill_blocks_total: parse_or_default(sync_info.backfill_blocks_total.as_ref()),
        };

        Some(state_sync)
    } else {
        None
    }
}

/// Build network-related stats such as peers and listening state.
fn build_network_info(
    tenderdash_status: &TenderdashStatusResponse,
    tenderdash_netinfo: &NetInfoResponse,
) -> Option<get_status_response_v0::Network> {
    if tenderdash_netinfo.listening.is_some() {
        let listening = tenderdash_netinfo.listening.unwrap_or(false);
        let peers_count = tenderdash_netinfo
            .n_peers
            .as_ref()
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap_or(0);

        let chain_id = tenderdash_status
            .node_info
            .as_ref()
            .and_then(|n| n.network.as_ref())
            .cloned()
            .unwrap_or_default();

        let network = get_status_response_v0::Network {
            listening,
            peers_count,
            chain_id,
        };

        Some(network)
    } else {
        None
    }
}

/// Compose the time section using Drive status timestamps.
fn build_time_info(drive_status: &DriveStatusResponse) -> get_status_response_v0::Time {
    let mut time = get_status_response_v0::Time::default();

    if let Some(drive_time) = &drive_status.time {
        time.block = drive_time.block;
        time.genesis = drive_time.genesis;
        time.epoch = drive_time.epoch.map(|e| e as u32);
    }

    time.local = chrono::Utc::now().timestamp() as u64;

    time
}
