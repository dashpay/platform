use dapi_grpc::platform::v0::{
    get_status_response::get_status_response_v0,
    get_status_response::{self, GetStatusResponseV0},
    GetStatusRequest, GetStatusResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use std::time::Duration;
use tokio::time::Instant;

use crate::clients::{
    drive_client::DriveStatusResponse,
    tenderdash_client::{NetInfoResponse, TenderdashStatusResponse},
};

// The struct is defined in the parent platform_service.rs module
use crate::services::platform_service::PlatformServiceImpl;

impl PlatformServiceImpl {
    pub async fn get_status_impl(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        // Check cache first
        let cache_key = "get_status".to_string();

        if let Some((cached_response, cached_time)) = self.cache.get(&cache_key).await {
            // If cache is still fresh (less than 10 seconds old), return it with updated local time
            if cached_time.elapsed() < Duration::from_secs(10) {
                let mut response = cached_response;
                // Update local time to current time
                if let Some(dapi_grpc::platform::v0::get_status_response::Version::V0(ref mut v0)) =
                    response.version
                {
                    if let Some(ref mut time) = v0.time {
                        time.local = chrono::Utc::now().timestamp() as u64;
                    }
                }
                return Ok(Response::new(response));
            }
        }

        // Build fresh response
        match self.build_status_response().await {
            Ok(response) => {
                // Cache the response
                let cache_entry = (response.clone(), Instant::now());
                self.cache.insert(cache_key, cache_entry).await;

                Ok(Response::new(response))
            }
            Err(status) => Err(status),
        }
    }

    async fn build_status_response(&self) -> Result<GetStatusResponse, Status> {
        // Prepare request for Drive
        let drive_request = GetStatusRequest {
            version: Some(dapi_grpc::platform::v0::get_status_request::Version::V0(
                dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0 {},
            )),
        };

        // Fetch data from Drive and Tenderdash concurrently
        let (drive_result, tenderdash_status_result, tenderdash_netinfo_result) = tokio::join!(
            self.drive_client.get_status(&drive_request),
            self.tenderdash_client.status(),
            self.tenderdash_client.net_info()
        );

        // Handle potential errors by using empty data if calls fail
        let drive_status = drive_result.unwrap_or_default();
        let tenderdash_status = tenderdash_status_result.unwrap_or_default();
        let tenderdash_netinfo = tenderdash_netinfo_result.unwrap_or_default();

        // Use standalone functions to create the response
        build_status_response(drive_status, tenderdash_status, tenderdash_netinfo)
    }
}

// Status building functions

fn build_status_response(
    drive_status: DriveStatusResponse,
    tenderdash_status: TenderdashStatusResponse,
    tenderdash_netinfo: NetInfoResponse,
) -> Result<GetStatusResponse, Status> {
    let mut v0 = GetStatusResponseV0::default();

    // Build each section using separate functions
    v0.version = Some(build_version_info(&drive_status, &tenderdash_status));
    v0.node = build_node_info(&tenderdash_status);
    v0.chain = build_chain_info(&drive_status, &tenderdash_status);
    v0.state_sync = build_state_sync_info(&tenderdash_status);
    v0.network = build_network_info(&tenderdash_status, &tenderdash_netinfo);
    v0.time = Some(build_time_info(&drive_status));

    let response = GetStatusResponse {
        version: Some(get_status_response::Version::V0(v0)),
    };

    Ok(response)
}

fn build_version_info(
    drive_status: &DriveStatusResponse,
    tenderdash_status: &TenderdashStatusResponse,
) -> get_status_response_v0::Version {
    let mut version = get_status_response_v0::Version::default();

    // Protocol version
    let mut protocol = get_status_response_v0::version::Protocol::default();

    // Tenderdash protocol version
    if let Some(node_info) = &tenderdash_status.node_info {
        if let Some(protocol_version) = &node_info.protocol_version {
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
    }

    // Drive protocol version
    if let Some(version_info) = &drive_status.version {
        if let Some(protocol_info) = &version_info.protocol {
            if let Some(drive_protocol) = &protocol_info.drive {
                let mut drive_protocol_version =
                    get_status_response_v0::version::protocol::Drive::default();

                drive_protocol_version.current = drive_protocol.current.unwrap_or(0) as u32;
                drive_protocol_version.latest = drive_protocol.latest.unwrap_or(0) as u32;

                protocol.drive = Some(drive_protocol_version);
            }
        }
    }

    version.protocol = Some(protocol);

    // Software version
    let mut software = get_status_response_v0::version::Software::default();

    software.dapi = env!("CARGO_PKG_VERSION").to_string();

    if let Some(version_info) = &drive_status.version {
        if let Some(software_info) = &version_info.software {
            if let Some(drive_version) = &software_info.drive {
                software.drive = Some(drive_version.clone());
            }
        }
    }

    if let Some(node_info) = &tenderdash_status.node_info {
        if let Some(tenderdash_version) = &node_info.version {
            software.tenderdash = Some(tenderdash_version.clone());
        }
    }

    version.software = Some(software);
    version
}

fn build_node_info(
    tenderdash_status: &TenderdashStatusResponse,
) -> Option<get_status_response_v0::Node> {
    if let Some(node_info) = &tenderdash_status.node_info {
        let mut node = get_status_response_v0::Node::default();

        if let Some(id) = &node_info.id {
            if let Ok(id_bytes) = hex::decode(id) {
                node.id = id_bytes;
            }
        }

        if let Some(pro_tx_hash) = &node_info.pro_tx_hash {
            if let Ok(pro_tx_hash_bytes) = hex::decode(pro_tx_hash) {
                node.pro_tx_hash = Some(pro_tx_hash_bytes);
            }
        }

        Some(node)
    } else {
        None
    }
}

fn build_chain_info(
    drive_status: &DriveStatusResponse,
    tenderdash_status: &TenderdashStatusResponse,
) -> Option<get_status_response_v0::Chain> {
    if let Some(sync_info) = &tenderdash_status.sync_info {
        let mut chain = get_status_response_v0::Chain::default();

        chain.catching_up = sync_info.catching_up.unwrap_or(false);

        if let Some(latest_block_hash) = &sync_info.latest_block_hash {
            if let Ok(hash_bytes) = hex::decode(latest_block_hash) {
                chain.latest_block_hash = hash_bytes;
            }
        }

        if let Some(latest_app_hash) = &sync_info.latest_app_hash {
            if let Ok(hash_bytes) = hex::decode(latest_app_hash) {
                chain.latest_app_hash = hash_bytes;
            }
        }

        if let Some(latest_block_height) = &sync_info.latest_block_height {
            chain.latest_block_height = latest_block_height.parse().unwrap_or(0);
        }

        if let Some(earliest_block_hash) = &sync_info.earliest_block_hash {
            if let Ok(hash_bytes) = hex::decode(earliest_block_hash) {
                chain.earliest_block_hash = hash_bytes;
            }
        }

        if let Some(earliest_app_hash) = &sync_info.earliest_app_hash {
            if let Ok(hash_bytes) = hex::decode(earliest_app_hash) {
                chain.earliest_app_hash = hash_bytes;
            }
        }

        if let Some(earliest_block_height) = &sync_info.earliest_block_height {
            chain.earliest_block_height = earliest_block_height.parse().unwrap_or(0);
        }

        if let Some(max_peer_block_height) = &sync_info.max_peer_block_height {
            chain.max_peer_block_height = max_peer_block_height.parse().unwrap_or(0);
        }

        if let Some(drive_chain) = &drive_status.chain {
            if let Some(core_chain_locked_height) = drive_chain.core_chain_locked_height {
                chain.core_chain_locked_height = Some(core_chain_locked_height as u32);
            }
        }

        Some(chain)
    } else {
        None
    }
}

fn build_state_sync_info(
    tenderdash_status: &TenderdashStatusResponse,
) -> Option<get_status_response_v0::StateSync> {
    if let Some(sync_info) = &tenderdash_status.sync_info {
        let mut state_sync = get_status_response_v0::StateSync::default();

        state_sync.total_synced_time = sync_info
            .total_synced_time
            .as_ref()
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap_or(0);
        state_sync.remaining_time = sync_info
            .remaining_time
            .as_ref()
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap_or(0);
        state_sync.total_snapshots = sync_info
            .total_snapshots
            .as_ref()
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap_or(0);
        state_sync.chunk_process_avg_time = sync_info
            .chunk_process_avg_time
            .as_ref()
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap_or(0);
        state_sync.snapshot_height = sync_info
            .snapshot_height
            .as_ref()
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap_or(0);
        state_sync.snapshot_chunks_count = sync_info
            .snapshot_chunks_count
            .as_ref()
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap_or(0);
        state_sync.backfilled_blocks = sync_info
            .backfilled_blocks
            .as_ref()
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap_or(0);
        state_sync.backfill_blocks_total = sync_info
            .backfill_blocks_total
            .as_ref()
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap_or(0);

        Some(state_sync)
    } else {
        None
    }
}

fn build_network_info(
    tenderdash_status: &TenderdashStatusResponse,
    tenderdash_netinfo: &NetInfoResponse,
) -> Option<get_status_response_v0::Network> {
    if tenderdash_netinfo.listening.is_some() {
        let mut network = get_status_response_v0::Network::default();

        network.listening = tenderdash_netinfo.listening.unwrap_or(false);
        network.peers_count = tenderdash_netinfo
            .n_peers
            .as_ref()
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap_or(0);

        if let Some(node_info) = &tenderdash_status.node_info {
            if let Some(network_name) = &node_info.network {
                network.chain_id = network_name.clone();
            }
        }

        Some(network)
    } else {
        None
    }
}

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
