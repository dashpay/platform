use dapi_grpc::platform::v0::{
    GetStatusRequest, GetStatusResponse,
    get_status_response::get_status_response_v0,
    get_status_response::{self, GetStatusResponseV0},
};
use dapi_grpc::tonic::{Request, Response, Status};
use tracing::{debug, trace};

use crate::clients::{
    drive_client::DriveStatusResponse,
    tenderdash_client::{NetInfoResponse, TenderdashStatusResponse},
};
use crate::error::DapiError;

// The struct is defined in the parent platform_service.rs module
use crate::services::platform_service::PlatformServiceImpl;

/// Captures upstream health information when building the Platform status response.
#[derive(Debug, Default)]
pub struct PlatformStatusHealth {
    pub drive_error: Option<DapiError>,
    pub tenderdash_status_error: Option<DapiError>,
    pub tenderdash_netinfo_error: Option<DapiError>,
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

        // Cache status response, just to avoid hammering Drive and Tenderdash
        let key = make_cache_key("get_status", request.get_ref());
        trace!(?key, "get_status cache lookup");
        if let Some(mut cached) = self
            .platform_cache
            .get_with_ttl::<GetStatusResponse>(&key, Duration::from_secs(10))
        {
            trace!(?key, "get_status cache hit");
            // Refresh local time to current instant like JS implementation
            if let Some(get_status_response::Version::V0(ref mut v0)) = cached.version
                && let Some(ref mut time) = v0.time
            {
                time.local = chrono::Utc::now().timestamp().max(0) as u64;
            }
            return Ok(Response::new(cached));
        }

        trace!(?key, "get_status cache miss; building response");
        // Build fresh response and cache it
        match self.build_status_response_with_health().await {
            Ok((response, health)) => {
                trace!(
                    drive_error = ?health.drive_error,
                    tenderdash_status_error = ?health.tenderdash_status_error,
                    tenderdash_netinfo_error = ?health.tenderdash_netinfo_error,
                    "get_status upstream fetch completed"
                );
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
        trace!("fetching Drive status, Tenderdash status, and netinfo");
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
                health.drive_error = Some(e.into());
                DriveStatusResponse::default()
            }
        };

        let tenderdash_status = match tenderdash_status_result {
            Ok(status) => status,
            Err(e) => {
                debug!(error = ?e, "Failed to fetch Tenderdash status - technical failure, using defaults");
                health.tenderdash_status_error = Some(e);
                TenderdashStatusResponse::default()
            }
        };

        let tenderdash_netinfo = match tenderdash_netinfo_result {
            Ok(netinfo) => netinfo,
            Err(e) => {
                debug!(error = ?e, "Failed to fetch Tenderdash netinfo - technical failure, using defaults");
                health.tenderdash_netinfo_error = Some(e);
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

    let response: GetStatusResponse = GetStatusResponse {
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
    let node_info = &tenderdash_status.node_info;
    let protocol_version = &node_info.protocol_version;

    if !protocol_version.block.is_empty() || !protocol_version.p2p.is_empty() {
        let mut tenderdash_protocol =
            get_status_response_v0::version::protocol::Tenderdash::default();

        if !protocol_version.block.is_empty() {
            tenderdash_protocol.block = protocol_version.block.parse().unwrap_or(0);
        }

        if !protocol_version.p2p.is_empty() {
            tenderdash_protocol.p2p = protocol_version.p2p.parse().unwrap_or(0);
        }

        protocol.tenderdash = Some(tenderdash_protocol);
    }

    // Drive protocol version
    if let Some(version_info) = &drive_status.version
        && let Some(protocol_info) = &version_info.protocol
        && let Some(drive_protocol) = &protocol_info.drive
    {
        protocol.drive = Some(drive_protocol.clone());
    }

    version.protocol = Some(protocol);

    // Software version
    let drive_version = drive_status
        .version
        .as_ref()
        .and_then(|v| v.software.as_ref())
        .and_then(|s| s.drive.as_ref())
        .cloned();

    let tenderdash_version = if tenderdash_status.node_info.version.is_empty() {
        None
    } else {
        Some(tenderdash_status.node_info.version.clone())
    };

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
    let node_info = &tenderdash_status.node_info;

    if node_info.id.is_empty() && node_info.pro_tx_hash.is_empty() {
        None
    } else {
        let mut node = get_status_response_v0::Node::default();

        if let Ok(id_bytes) = hex::decode(&node_info.id) {
            node.id = id_bytes;
        }

        if !node_info.pro_tx_hash.is_empty()
            && let Ok(pro_tx_hash_bytes) = hex::decode(&node_info.pro_tx_hash)
        {
            node.pro_tx_hash = Some(pro_tx_hash_bytes);
        }

        Some(node)
    }
}

/// Construct chain synchronization information combining Drive and Tenderdash fields.
fn build_chain_info(
    drive_status: &DriveStatusResponse,
    tenderdash_status: &TenderdashStatusResponse,
) -> Option<get_status_response_v0::Chain> {
    let sync_info = &tenderdash_status.sync_info;

    let has_sync_data = sync_info.latest_block_height != 0
        || !sync_info.latest_block_hash.is_empty()
        || !sync_info.latest_app_hash.is_empty();

    if !has_sync_data {
        None
    } else {
        let catching_up = sync_info.catching_up;

        let latest_block_hash = if sync_info.latest_block_hash.is_empty() {
            Vec::new()
        } else {
            hex::decode(&sync_info.latest_block_hash).unwrap_or_default()
        };

        let latest_app_hash = if sync_info.latest_app_hash.is_empty() {
            Vec::new()
        } else {
            hex::decode(&sync_info.latest_app_hash).unwrap_or_default()
        };

        let latest_block_height = sync_info.latest_block_height.max(0) as u64;

        let earliest_block_hash = if sync_info.earliest_block_hash.is_empty() {
            Vec::new()
        } else {
            hex::decode(&sync_info.earliest_block_hash).unwrap_or_default()
        };

        let earliest_app_hash = if sync_info.earliest_app_hash.is_empty() {
            Vec::new()
        } else {
            hex::decode(&sync_info.earliest_app_hash).unwrap_or_default()
        };

        let earliest_block_height = sync_info.earliest_block_height.max(0) as u64;
        let max_peer_block_height = sync_info.max_peer_block_height.max(0) as u64;

        let core_chain_locked_height = drive_status
            .chain
            .as_ref()
            .and_then(|c| c.core_chain_locked_height)
            .map(|h| {
                h.try_into()
                    .inspect_err(|error| {
                        tracing::warn!(
                            core_chain_locked_height = h,
                            ?error,
                            "Failed to convert core_chain_locked_height"
                        )
                    })
                    .unwrap_or(u32::MIN)
            });

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
    }
}

/// Produce state sync metrics derived from Tenderdash status response.
fn build_state_sync_info(
    tenderdash_status: &TenderdashStatusResponse,
) -> Option<get_status_response_v0::StateSync> {
    let sync_info = &tenderdash_status.sync_info;

    let has_state_sync_data = !sync_info.total_synced_time.is_empty()
        || !sync_info.remaining_time.is_empty()
        || !sync_info.total_snapshots.is_empty()
        || !sync_info.snapshot_height.is_empty();

    if !has_state_sync_data {
        None
    } else {
        let parse_or_default = |value: &str| -> u64 {
            if value.is_empty() {
                0
            } else {
                value.parse::<i64>().map(|v| v.max(0) as u64).unwrap_or(0)
            }
        };

        let state_sync = get_status_response_v0::StateSync {
            total_synced_time: parse_or_default(&sync_info.total_synced_time),
            remaining_time: parse_or_default(&sync_info.remaining_time),
            total_snapshots: parse_or_default(&sync_info.total_snapshots).min(u32::MAX as u64)
                as u32,
            chunk_process_avg_time: parse_or_default(&sync_info.chunk_process_avg_time),
            snapshot_height: parse_or_default(&sync_info.snapshot_height),
            snapshot_chunks_count: parse_or_default(&sync_info.snapshot_chunks_count),
            backfilled_blocks: parse_or_default(&sync_info.backfilled_blocks),
            backfill_blocks_total: parse_or_default(&sync_info.backfill_blocks_total),
        };

        Some(state_sync)
    }
}

/// Build network-related stats such as peers and listening state.
fn build_network_info(
    tenderdash_status: &TenderdashStatusResponse,
    tenderdash_netinfo: &NetInfoResponse,
) -> Option<get_status_response_v0::Network> {
    let has_network_data = tenderdash_netinfo.listening
        || tenderdash_netinfo.n_peers > 0
        || !tenderdash_status.node_info.network.is_empty();

    if !has_network_data {
        None
    } else {
        let network = get_status_response_v0::Network {
            listening: tenderdash_netinfo.listening,
            peers_count: tenderdash_netinfo.n_peers,
            chain_id: tenderdash_status.node_info.network.clone(),
        };

        Some(network)
    }
}

/// Compose the time section using Drive status timestamps.
fn build_time_info(drive_status: &DriveStatusResponse) -> get_status_response_v0::Time {
    let mut time = get_status_response_v0::Time::default();

    if let Some(drive_time) = &drive_status.time {
        time.block = drive_time.block;
        time.genesis = drive_time.genesis;
        time.epoch = drive_time.epoch;
    }

    time.local = chrono::Utc::now().timestamp().max(0) as u64;

    time
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::drive_client::DriveStatusResponse;
    use crate::clients::tenderdash_client::{NetInfoResponse, TenderdashStatusResponse};

    #[test]
    fn build_status_response_populates_fields_from_tenderdash_status() {
        let tenderdash_status: TenderdashStatusResponse =
            serde_json::from_str(TENDERMASH_STATUS_JSON).expect("parse tenderdash status");
        let drive_status = DriveStatusResponse::default();
        let net_info = NetInfoResponse::default();

        let response =
            build_status_response(drive_status, tenderdash_status, net_info).expect("build ok");

        let get_status_response::Version::V0(inner) = response.version.expect("version present");

        let version = inner.version.expect("version struct");
        let software = version.software.expect("software present");
        assert_eq!(software.tenderdash.as_deref(), Some("1.5.0-dev.3"));

        let protocol = version.protocol.expect("protocol present");
        let tenderdash_protocol = protocol.tenderdash.expect("tenderdash protocol");
        assert_eq!(tenderdash_protocol.block, 14);
        assert_eq!(tenderdash_protocol.p2p, 10);

        let node = inner.node.expect("node present");
        assert_eq!(
            node.id,
            hex::decode("972a33056d57359de8acfa4fb8b29dc1c14f76b8").expect("decode node id")
        );

        let chain = inner.chain.expect("chain present");
        assert_eq!(chain.latest_block_height, 198748);
        assert_eq!(
            chain.latest_block_hash,
            hex::decode("B15CB7BD25D5334587B591D46FADEDA3AFCE2C57B7BC99E512F79422AB710343")
                .expect("decode latest block hash")
        );
        assert_eq!(
            chain.earliest_block_hash,
            hex::decode("08FA02C27EC0390BA301E4FC7E3D7EADB350C8193E3E62A093689706E3A20BFA")
                .expect("decode earliest block hash")
        );

        let network = inner.network.expect("network present");
        assert_eq!(network.chain_id, "dash-testnet-51");

        let state_sync = inner.state_sync.expect("state sync present");
        assert_eq!(state_sync.total_synced_time, 0);

        let time = inner.time.expect("time present");
        assert!(time.local > 0);
    }

    const TENDERMASH_STATUS_JSON: &str = r#"
    {
      "node_info": {
        "protocol_version": {
          "p2p": "10",
          "block": "14",
          "app": "9"
        },
        "id": "972a33056d57359de8acfa4fb8b29dc1c14f76b8",
        "listen_addr": "44.239.39.153:36656",
        "ProTxHash": "5C6542766615387183715D958A925552472F93335FA1612880423E4BBDAEF436",
        "network": "dash-testnet-51",
        "version": "1.5.0-dev.3",
        "channels": [
          64,
          32,
          33,
          34,
          35,
          48,
          56,
          96,
          97,
          98,
          99,
          0
        ],
        "moniker": "hp-masternode-16",
        "other": {
          "tx_index": "on",
          "rpc_address": "tcp://0.0.0.0:36657"
        }
      },
      "application_info": {
        "version": "10"
      },
      "sync_info": {
        "latest_block_hash": "B15CB7BD25D5334587B591D46FADEDA3AFCE2C57B7BC99E512F79422AB710343",
        "latest_app_hash": "FB90D667EB6CAE5DD5293EED7ECCE8B8B492EC0FF310BB0CB0C49C7DC1FFF9CD",
        "latest_block_height": "198748",
        "latest_block_time": "2025-10-14T13:10:48.765Z",
        "earliest_block_hash": "08FA02C27EC0390BA301E4FC7E3D7EADB350C8193E3E62A093689706E3A20BFA",
        "earliest_app_hash": "BF0CCB9CA071BA01AE6E67A0C090F97803D26D56D675DCD5131781CBCAC8EC8F",
        "earliest_block_height": "1",
        "earliest_block_time": "2024-07-19T01:40:09Z",
        "max_peer_block_height": "198748",
        "catching_up": false,
        "total_synced_time": "0",
        "remaining_time": "0",
        "total_snapshots": "0",
        "chunk_process_avg_time": "0",
        "snapshot_height": "0",
        "snapshot_chunks_count": "0",
        "backfilled_blocks": "0",
        "backfill_blocks_total": "0"
      },
      "validator_info": {
        "pro_tx_hash": "5C6542766615387183715D958A925552472F93335FA1612880423E4BBDAEF436",
        "voting_power": 100
      },
      "light_client_info": {
        "primaryID": "",
        "witnessesID": null,
        "number_of_peers": "0",
        "last_trusted_height": "0",
        "last_trusted_hash": "",
        "latest_block_time": "0001-01-01T00:00:00Z",
        "trusting_period": "",
        "trusted_block_expired": false
      }
    }
    "#;
}
