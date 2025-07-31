// Native gRPC protocol handler - direct pass-through

use crate::errors::DapiResult;
use dapi_grpc::platform::v0::{GetStatusRequest, GetStatusResponse};

#[derive(Debug)]
pub struct GrpcNativeHandler;

impl GrpcNativeHandler {
    pub fn new() -> Self {
        Self
    }

    // For native gRPC, we just pass through the requests directly
    pub async fn handle_get_status(&self, request: GetStatusRequest) -> DapiResult<GetStatusResponse> {
        // This would normally call the actual service implementation
        // For now, we'll create a dummy implementation
        let response = create_dummy_status_response();
        Ok(response)
    }
}

fn create_dummy_status_response() -> GetStatusResponse {
    use dapi_grpc::platform::v0::get_status_response::GetStatusResponseV0;
    use dapi_grpc::platform::v0::get_status_response::get_status_response_v0::{
        Version, Time, Node, Chain, Network, StateSync
    };
    use dapi_grpc::platform::v0::get_status_response::get_status_response_v0::version::{
        Software, Protocol
    };
    use dapi_grpc::platform::v0::get_status_response::get_status_response_v0::version::protocol::{
        Tenderdash, Drive
    };

    let software = Software {
        dapi: "rs-dapi-0.1.0".to_string(),
        drive: Some("drive-0.1.0".to_string()),
        tenderdash: Some("tenderdash-0.1.0".to_string()),
    };

    let protocol = Protocol {
        tenderdash: Some(Tenderdash {
            p2p: 8,
            block: 11,
        }),
        drive: Some(Drive {
            latest: 1,
            current: 1,
        }),
    };

    let version = Version {
        software: Some(software),
        protocol: Some(protocol),
    };

    let time = Time {
        local: chrono::Utc::now().timestamp_millis() as u64,
        block: Some(chrono::Utc::now().timestamp_millis() as u64),
        genesis: Some(1640995200000), // Example genesis time
        epoch: Some(1),
    };

    let node = Node {
        id: b"test-node-id".to_vec(),
        pro_tx_hash: Some(b"test-pro-tx-hash".to_vec()),
    };

    let chain = Chain {
        catching_up: false,
        latest_block_hash: b"latest-block-hash".to_vec(),
        latest_app_hash: b"latest-app-hash".to_vec(),
        latest_block_height: 1000,
        earliest_block_hash: b"earliest-block-hash".to_vec(),
        earliest_app_hash: b"earliest-app-hash".to_vec(),
        earliest_block_height: 1,
        max_peer_block_height: 1000,
        core_chain_locked_height: Some(999),
    };

    let network = Network {
        chain_id: "dash-testnet".to_string(),
        peers_count: 5,
        listening: true,
    };

    let state_sync = StateSync {
        total_synced_time: 0,
        remaining_time: 0,
        total_snapshots: 0,
        chunk_process_avg_time: 0,
        snapshot_height: 0,
        snapshot_chunks_count: 0,
        backfilled_blocks: 0,
        backfill_blocks_total: 0,
    };

    let response_v0 = GetStatusResponseV0 {
        version: Some(version),
        node: Some(node),
        chain: Some(chain),
        network: Some(network),
        state_sync: Some(state_sync),
        time: Some(time),
    };

    GetStatusResponse {
        version: Some(dapi_grpc::platform::v0::get_status_response::Version::V0(response_v0)),
    }
}
