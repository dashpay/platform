/// Accessors for Masternode
pub mod accessors;

use dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeType};
use dashcore_rpc::json::MasternodeListItem;
use dpp::bincode::{Decode, Encode};
use std::fmt::{Debug, Formatter};

use dpp::dashcore::{ProTxHash, Txid};

use std::net::SocketAddr;

/// `Masternode` represents a masternode on the network.
#[derive(Clone, PartialEq, Encode, Decode)]
pub struct MasternodeV0 {
    /// The type of masternode (e.g., full, partial).
    pub node_type: MasternodeType,
    /// A unique hash representing the masternode's registration transaction.
    #[bincode(with_serde)]
    pub pro_tx_hash: ProTxHash,
    /// A unique hash representing the collateral transaction.
    #[bincode(with_serde)]
    pub collateral_hash: Txid,
    /// The index of the collateral transaction output.
    pub collateral_index: u32,
    /// The address where the collateral is stored.
    pub collateral_address: [u8; 20],
    /// The amount of the operator's reward for running the masternode.
    pub operator_reward: f32,
    /// The current state of the masternode (e.g., enabled, pre-enabled, banned).
    pub state: MasternodeStateV0,
}

impl Debug for MasternodeV0 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MasternodeV0")
            .field("node_type", &self.node_type)
            .field("pro_tx_hash", &self.pro_tx_hash.to_string())
            .field("collateral_hash", &self.collateral_hash)
            .field("collateral_index", &self.collateral_index)
            .field("collateral_address", &self.collateral_address)
            .field("operator_reward", &self.operator_reward)
            .field("state", &self.state)
            .finish()
    }
}

impl From<MasternodeListItem> for MasternodeV0 {
    fn from(value: MasternodeListItem) -> Self {
        let MasternodeListItem {
            node_type,
            pro_tx_hash,
            collateral_hash,
            collateral_index,
            collateral_address,
            operator_reward,
            state,
        } = value;

        Self {
            node_type,
            pro_tx_hash,
            collateral_hash,
            collateral_index,
            collateral_address,
            operator_reward,
            state: state.into(),
        }
    }
}

impl From<MasternodeV0> for MasternodeListItem {
    fn from(value: MasternodeV0) -> Self {
        let MasternodeV0 {
            node_type,
            pro_tx_hash,
            collateral_hash,
            collateral_index,
            collateral_address,
            operator_reward,
            state,
        } = value;

        Self {
            node_type,
            pro_tx_hash,
            collateral_hash,
            collateral_index,
            collateral_address,
            operator_reward,
            state: state.into(),
        }
    }
}

/// A `MasternodeState` contains information about a masternode's state.
#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct MasternodeStateV0 {
    /// Masternode's network service address.
    #[bincode(with_serde)]
    pub service: SocketAddr,

    /// Block height when the masternode was registered.
    pub registered_height: u32,

    /// Block height when the masternode last revived from a Proof-of-Service ban.
    pub pose_revived_height: Option<u32>,

    /// Block height when the masternode was banned due to a failed Proof-of-Service.
    pub pose_ban_height: Option<u32>,

    /// Reason for the masternode's revocation (encoded as an integer).
    pub revocation_reason: u32,

    /// The masternode owner's public address.
    pub owner_address: [u8; 20],

    /// The masternode voting public address.
    pub voting_address: [u8; 20],

    /// The masternode payout public address.
    pub payout_address: [u8; 20],

    /// The masternode operator's public key.
    pub pub_key_operator: Vec<u8>,

    /// Optional masternode operator's payout public address.
    pub operator_payout_address: Option<[u8; 20]>,

    /// Platform-specific node ID for the masternode.
    pub platform_node_id: Option<[u8; 20]>,

    /// Optional platform-specific P2P port for the masternode.
    pub platform_p2p_port: Option<u32>,

    /// Optional platform-specific HTTP port for the masternode.
    pub platform_http_port: Option<u32>,
}

impl From<DMNState> for MasternodeStateV0 {
    fn from(value: DMNState) -> Self {
        let DMNState {
            service,
            registered_height,
            pose_revived_height,
            pose_ban_height,
            revocation_reason,
            owner_address,
            voting_address,
            payout_address,
            pub_key_operator,
            operator_payout_address,
            platform_node_id,
            platform_p2p_port,
            platform_http_port,
        } = value;

        Self {
            service,
            registered_height,
            pose_revived_height,
            pose_ban_height,
            revocation_reason,
            owner_address,
            voting_address,
            payout_address,
            pub_key_operator,
            operator_payout_address,
            platform_node_id,
            platform_p2p_port,
            platform_http_port,
        }
    }
}

impl From<MasternodeStateV0> for DMNState {
    fn from(value: MasternodeStateV0) -> Self {
        let MasternodeStateV0 {
            service,
            registered_height,
            pose_revived_height,
            pose_ban_height,
            revocation_reason,
            owner_address,
            voting_address,
            payout_address,
            pub_key_operator,
            operator_payout_address,
            platform_node_id,
            platform_p2p_port,
            platform_http_port,
        } = value;

        Self {
            service,
            registered_height,
            pose_revived_height,
            pose_ban_height,
            revocation_reason,
            owner_address,
            voting_address,
            payout_address,
            pub_key_operator,
            operator_payout_address,
            platform_node_id,
            platform_p2p_port,
            platform_http_port,
        }
    }
}
