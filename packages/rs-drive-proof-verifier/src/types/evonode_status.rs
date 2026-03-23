//! Status details of EvoNode, like version, current height, etc.

use crate::Error;
use dapi_grpc::platform::v0::{
    get_status_response::{self},
    GetStatusResponse,
};

#[cfg(feature = "mocks")]
use {
    bincode::{Decode, Encode},
    dpp::{version as platform_version, ProtocolError},
    platform_serialization_derive::{PlatformDeserialize, PlatformSerialize},
};

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// The status of an EvoNode.
pub struct EvoNodeStatus {
    /// Information about protocol and software components versions.
    pub version: Version,
    /// Information about the node.
    pub node: Node,
    /// Layer 2 blockchain information
    pub chain: Chain,
    /// Node networking information.
    pub network: Network,
    /// Information about state synchronization progress.
    pub state_sync: StateSync,
    /// Information about current time used by the node.
    pub time: Time,
}
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// Information about protocol and software components versions.
pub struct Version {
    /// Information about software components versions.
    pub software: Option<Software>,
    /// Information about protocol version.
    pub protocol: Option<Protocol>,
}
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// Information about software components versions.
pub struct Software {
    /// DAPI version, semver-compatible string.
    pub dapi: String,
    /// Drive version, semver-compatible string.
    pub drive: Option<String>,
    /// Tenderdash version, semver-compatible string.
    pub tenderdash: Option<String>,
}
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// Information about protocol-level versions used by the node
pub struct Protocol {
    /// Tenderdash protocols version.
    pub tenderdash: Option<TenderdashProtocol>,
    /// Drive protocols version.
    pub drive: Option<DriveProtocol>,
}
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// Tenderdash protocol versions.
pub struct TenderdashProtocol {
    /// Tenderdash P2P protocol version.
    pub p2p: u32,
    /// Tenderdash block protocol version.
    pub block: u32,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// Drive protocol versions.
pub struct DriveProtocol {
    /// Latest version supported by the node.
    pub latest: u32,
    /// Current version used by the node.
    pub current: u32,
    /// Protocol version scheduled for the next epoch.
    pub next_epoch: u32,
}
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// Information about current time used by the node.
pub struct Time {
    /// Local time of the node. Unix timestamp since epoch.
    pub local: u64,
    /// Time of the last block. Unix timestamp since epoch.
    pub block: Option<u64>,
    /// Genesis time. Unix timestamp since epoch.
    pub genesis: Option<u64>,
    /// Epoch number
    pub epoch: Option<u32>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// Evo node identification information.
pub struct Node {
    /// Node ID
    pub id: Vec<u8>,
    /// ProTxHash of masternode; None for full nodes
    pub pro_tx_hash: Option<Vec<u8>>,
}
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// Layer 2 blockchain information
pub struct Chain {
    /// Whether the node is catching up with the network.
    pub catching_up: bool,
    /// Block hash of the latest block on the node.
    pub latest_block_hash: Vec<u8>,
    /// Latest app hash of the node, as visible in the latest block.
    pub latest_app_hash: Vec<u8>,
    /// Block hash of the earliest block available on the node.
    pub earliest_block_hash: Vec<u8>,
    /// Earliest app hash of the node, as visible in the earliest block.
    pub earliest_app_hash: Vec<u8>,
    /// Height of the latest block available on the node.
    pub latest_block_height: u64,
    /// Height of the earliest block available on the node.
    pub earliest_block_height: u64,
    /// Maximum block height of the peers connected to the node.
    pub max_peer_block_height: u64,
    /// Current core chain locked height.
    pub core_chain_locked_height: Option<u32>,
}
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// Node networking information.
pub struct Network {
    /// Identifier of chain the node is member of.
    pub chain_id: String,
    /// Number of peers in the address book.
    pub peers_count: u32,
    /// Whether the node is listening for incoming connections.
    pub listening: bool,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "mocks",
    derive(Encode, Decode, PlatformSerialize, PlatformDeserialize),
    platform_serialize(unversioned)
)]
/// Information about state synchronization progress.
pub struct StateSync {
    /// Total time spent on state synchronization.
    pub total_synced_time: u64,
    /// Estimated remaining time to finish state synchronization.
    pub remaining_time: u64,
    /// Total number of snapshots available.
    pub total_snapshots: u32,
    /// Average time spent on processing a chunk of snapshot data.
    pub chunk_process_avg_time: u64,
    /// Height of the latest snapshot.
    pub snapshot_height: u64,
    /// Number of chunks in the latest snapshot.
    pub snapshot_chunks_count: u64,
    /// Number of backfilled blocks.
    pub backfilled_blocks: u64,
    /// Total number of blocks to backfill.
    pub backfill_blocks_total: u64,
}

impl TryFrom<GetStatusResponse> for EvoNodeStatus {
    type Error = Error;

    fn try_from(response: GetStatusResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            version: Version::try_from(&response)?,
            node: Node::try_from(&response)?,
            chain: Chain::try_from(&response)?,
            network: Network::try_from(&response)?,
            state_sync: StateSync::try_from(&response)?,
            time: Time::try_from(&response)?,
        })
    }
}

impl TryFrom<&GetStatusResponse> for Version {
    type Error = Error;

    fn try_from(response: &GetStatusResponse) -> Result<Self, Self::Error> {
        match &response.version {
            Some(get_status_response::Version::V0(v0)) => {
                let software = v0
                    .version
                    .as_ref()
                    .and_then(|v| v.software.clone())
                    .map(|s| Software {
                        dapi: s.dapi,
                        drive: s.drive,
                        tenderdash: s.tenderdash,
                    });

                let protocol = v0
                    .version
                    .as_ref()
                    .and_then(|v| v.protocol)
                    .map(|p| Protocol {
                        tenderdash: p.tenderdash.map(|t| TenderdashProtocol {
                            p2p: t.p2p,
                            block: t.block,
                        }),
                        drive: p.drive.map(|d| DriveProtocol {
                            latest: d.latest,
                            current: d.current,
                            next_epoch: d.next_epoch,
                        }),
                    });

                Ok(Self { software, protocol })
            }
            _ => Err(Error::EmptyVersion),
        }
    }
}

impl TryFrom<&GetStatusResponse> for Node {
    type Error = Error;

    fn try_from(response: &GetStatusResponse) -> Result<Self, Self::Error> {
        match &response.version {
            Some(get_status_response::Version::V0(v0)) => {
                let node = v0
                    .node
                    .as_ref()
                    .map(|n| Self {
                        id: n.id.clone(),
                        pro_tx_hash: n.pro_tx_hash.clone(),
                    })
                    .unwrap_or_default();
                Ok(node)
            }
            _ => Err(Error::EmptyVersion),
        }
    }
}

impl TryFrom<&GetStatusResponse> for Chain {
    type Error = Error;

    fn try_from(response: &GetStatusResponse) -> Result<Self, Self::Error> {
        match &response.version {
            Some(get_status_response::Version::V0(v0)) => {
                let chain = v0
                    .chain
                    .as_ref()
                    .map(|c| Self {
                        catching_up: c.catching_up,
                        latest_block_hash: c.latest_block_hash.clone(),
                        latest_app_hash: c.latest_app_hash.clone(),
                        earliest_block_hash: c.earliest_block_hash.clone(),
                        earliest_app_hash: c.earliest_app_hash.clone(),
                        latest_block_height: c.latest_block_height,
                        earliest_block_height: c.earliest_block_height,
                        max_peer_block_height: c.max_peer_block_height,
                        core_chain_locked_height: c.core_chain_locked_height,
                    })
                    .unwrap_or_default();
                Ok(chain)
            }
            _ => Err(Error::EmptyVersion),
        }
    }
}

impl TryFrom<&GetStatusResponse> for Network {
    type Error = Error;

    fn try_from(response: &GetStatusResponse) -> Result<Self, Self::Error> {
        match &response.version {
            Some(get_status_response::Version::V0(v0)) => {
                let network = v0
                    .network
                    .as_ref()
                    .map(|n| Self {
                        chain_id: n.chain_id.clone(),
                        peers_count: n.peers_count,
                        listening: n.listening,
                    })
                    .unwrap_or_default();
                Ok(network)
            }
            _ => Err(Error::EmptyVersion),
        }
    }
}

impl TryFrom<&GetStatusResponse> for StateSync {
    type Error = Error;

    fn try_from(response: &GetStatusResponse) -> Result<Self, Self::Error> {
        match &response.version {
            Some(get_status_response::Version::V0(v0)) => {
                let state_sync = v0
                    .state_sync
                    .as_ref()
                    .map(|s| Self {
                        total_synced_time: s.total_synced_time,
                        remaining_time: s.remaining_time,
                        total_snapshots: s.total_snapshots,
                        chunk_process_avg_time: s.chunk_process_avg_time,
                        snapshot_height: s.snapshot_height,
                        snapshot_chunks_count: s.snapshot_chunks_count,
                        backfilled_blocks: s.backfilled_blocks,
                        backfill_blocks_total: s.backfill_blocks_total,
                    })
                    .unwrap_or_default();
                Ok(state_sync)
            }
            _ => Err(Error::EmptyVersion),
        }
    }
}

impl TryFrom<&GetStatusResponse> for Time {
    type Error = Error;

    fn try_from(response: &GetStatusResponse) -> Result<Self, Self::Error> {
        match &response.version {
            Some(get_status_response::Version::V0(v0)) => {
                let time = v0
                    .time
                    .as_ref()
                    .map(|t| Self {
                        local: t.local,
                        block: t.block,
                        genesis: t.genesis,
                        epoch: t.epoch,
                    })
                    .unwrap_or_default();
                Ok(time)
            }
            _ => Err(Error::EmptyVersion),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_status_response::get_status_response_v0;

    /// Build a fully populated GetStatusResponse V0 for test reuse.
    fn build_full_status_response() -> GetStatusResponse {
        let software = get_status_response_v0::version::Software {
            dapi: "1.2.3".to_string(),
            drive: Some("4.5.6".to_string()),
            tenderdash: Some("0.14.0-dev.1".to_string()),
        };

        let tenderdash_protocol =
            get_status_response_v0::version::protocol::Tenderdash { p2p: 9, block: 12 };

        let drive_protocol = get_status_response_v0::version::protocol::Drive {
            latest: 7,
            current: 6,
            next_epoch: 7,
        };

        let protocol = get_status_response_v0::version::Protocol {
            tenderdash: Some(tenderdash_protocol),
            drive: Some(drive_protocol),
        };

        let version = get_status_response_v0::Version {
            software: Some(software),
            protocol: Some(protocol),
        };

        let time = get_status_response_v0::Time {
            local: 1700000000,
            block: Some(1699999900),
            genesis: Some(1690000000),
            epoch: Some(42),
        };

        let node = get_status_response_v0::Node {
            id: vec![0xAA; 20],
            pro_tx_hash: Some(vec![0xBB; 32]),
        };

        let chain = get_status_response_v0::Chain {
            catching_up: true,
            latest_block_hash: vec![0x11; 32],
            latest_app_hash: vec![0x22; 32],
            latest_block_height: 5000,
            earliest_block_hash: vec![0x33; 32],
            earliest_app_hash: vec![0x44; 32],
            earliest_block_height: 10,
            max_peer_block_height: 5001,
            core_chain_locked_height: Some(750),
        };

        let network = get_status_response_v0::Network {
            chain_id: "dash-mainnet".to_string(),
            peers_count: 50,
            listening: true,
        };

        let state_sync = get_status_response_v0::StateSync {
            total_synced_time: 7200,
            remaining_time: 60,
            total_snapshots: 3,
            chunk_process_avg_time: 25,
            snapshot_height: 4500,
            snapshot_chunks_count: 200,
            backfilled_blocks: 1000,
            backfill_blocks_total: 2000,
        };

        let v0 = get_status_response::GetStatusResponseV0 {
            version: Some(version),
            node: Some(node),
            chain: Some(chain),
            network: Some(network),
            state_sync: Some(state_sync),
            time: Some(time),
        };

        GetStatusResponse {
            version: Some(get_status_response::Version::V0(v0)),
        }
    }

    #[test]
    fn test_try_from_status_response_all_fields() {
        let response = build_full_status_response();
        let status = EvoNodeStatus::try_from(response).expect("should convert valid response");

        // Version / Software
        let sw = status.version.software.as_ref().unwrap();
        assert_eq!(sw.dapi, "1.2.3");
        assert_eq!(sw.drive.as_deref(), Some("4.5.6"));
        assert_eq!(sw.tenderdash.as_deref(), Some("0.14.0-dev.1"));

        // Version / Protocol
        let proto = status.version.protocol.as_ref().unwrap();
        let td_proto = proto.tenderdash.as_ref().unwrap();
        assert_eq!(td_proto.p2p, 9);
        assert_eq!(td_proto.block, 12);
        let drv_proto = proto.drive.as_ref().unwrap();
        assert_eq!(drv_proto.latest, 7);
        assert_eq!(drv_proto.current, 6);
        assert_eq!(drv_proto.next_epoch, 7);

        // Node
        assert_eq!(status.node.id, vec![0xAA; 20]);
        assert_eq!(status.node.pro_tx_hash, Some(vec![0xBB; 32]));

        // Chain
        assert!(status.chain.catching_up);
        assert_eq!(status.chain.latest_block_hash, vec![0x11; 32]);
        assert_eq!(status.chain.latest_app_hash, vec![0x22; 32]);
        assert_eq!(status.chain.latest_block_height, 5000);
        assert_eq!(status.chain.earliest_block_hash, vec![0x33; 32]);
        assert_eq!(status.chain.earliest_app_hash, vec![0x44; 32]);
        assert_eq!(status.chain.earliest_block_height, 10);
        assert_eq!(status.chain.max_peer_block_height, 5001);
        assert_eq!(status.chain.core_chain_locked_height, Some(750));

        // Network
        assert_eq!(status.network.chain_id, "dash-mainnet");
        assert_eq!(status.network.peers_count, 50);
        assert!(status.network.listening);

        // StateSync
        assert_eq!(status.state_sync.total_synced_time, 7200);
        assert_eq!(status.state_sync.remaining_time, 60);
        assert_eq!(status.state_sync.total_snapshots, 3);
        assert_eq!(status.state_sync.chunk_process_avg_time, 25);
        assert_eq!(status.state_sync.snapshot_height, 4500);
        assert_eq!(status.state_sync.snapshot_chunks_count, 200);
        assert_eq!(status.state_sync.backfilled_blocks, 1000);
        assert_eq!(status.state_sync.backfill_blocks_total, 2000);

        // Time
        assert_eq!(status.time.local, 1700000000);
        assert_eq!(status.time.block, Some(1699999900));
        assert_eq!(status.time.genesis, Some(1690000000));
        assert_eq!(status.time.epoch, Some(42));
    }

    #[test]
    fn test_try_from_status_response_empty_version() {
        let response = GetStatusResponse { version: None };
        let err = EvoNodeStatus::try_from(response).expect_err("should fail when version is None");
        let err_string = err.to_string();
        assert!(
            err_string.contains("empty version"),
            "unexpected error: {err_string}"
        );
    }

    #[test]
    fn test_version_conversion() {
        let response = build_full_status_response();
        let version = Version::try_from(&response).expect("should convert Version");

        // Software components
        let sw = version.software.as_ref().unwrap();
        assert_eq!(sw.dapi, "1.2.3");
        assert_eq!(sw.drive.as_deref(), Some("4.5.6"));
        assert_eq!(sw.tenderdash.as_deref(), Some("0.14.0-dev.1"));

        // Protocol versions
        let proto = version.protocol.as_ref().unwrap();

        let td = proto.tenderdash.as_ref().unwrap();
        assert_eq!(td.p2p, 9);
        assert_eq!(td.block, 12);

        let drv = proto.drive.as_ref().unwrap();
        assert_eq!(drv.latest, 7);
        assert_eq!(drv.current, 6);
        assert_eq!(drv.next_epoch, 7);
    }

    #[test]
    fn test_chain_conversion() {
        let response = build_full_status_response();
        let chain = Chain::try_from(&response).expect("should convert Chain");

        assert!(chain.catching_up);
        assert_eq!(chain.latest_block_hash, vec![0x11; 32]);
        assert_eq!(chain.latest_app_hash, vec![0x22; 32]);
        assert_eq!(chain.latest_block_height, 5000);
        assert_eq!(chain.earliest_block_hash, vec![0x33; 32]);
        assert_eq!(chain.earliest_app_hash, vec![0x44; 32]);
        assert_eq!(chain.earliest_block_height, 10);
        assert_eq!(chain.max_peer_block_height, 5001);
        assert_eq!(chain.core_chain_locked_height, Some(750));
    }
}
