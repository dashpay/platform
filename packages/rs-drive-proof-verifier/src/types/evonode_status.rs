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
