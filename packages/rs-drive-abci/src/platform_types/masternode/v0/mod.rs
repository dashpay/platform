/// Accessors for Masternode
pub mod accessors;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::bincode::{Decode, Encode};
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeType};
use dpp::dashcore_rpc::json::MasternodeListItem;
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

impl TryFrom<MasternodeListItem> for MasternodeV0 {
    type Error = Error;

    fn try_from(value: MasternodeListItem) -> Result<Self, Self::Error> {
        let MasternodeListItem {
            node_type,
            pro_tx_hash,
            collateral_hash,
            collateral_index,
            collateral_address,
            operator_reward,
            state,
        } = value;

        Ok(Self {
            node_type,
            pro_tx_hash,
            collateral_hash,
            collateral_index,
            collateral_address: required_legacy_address(collateral_address, "collateralAddress")?,
            operator_reward,
            state: state.try_into()?,
        })
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
            collateral_address: Some(collateral_address),
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

#[allow(deprecated)] // Persist the same flat ports as the shipped v0 format.
impl TryFrom<DMNState> for MasternodeStateV0 {
    type Error = Error;

    fn try_from(value: DMNState) -> Result<Self, Self::Error> {
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
            legacy_platform_p2p_port: platform_p2p_port,
            legacy_platform_http_port: platform_http_port,
            ..
        } = value;

        Ok(Self {
            service,
            registered_height,
            pose_revived_height,
            pose_ban_height,
            revocation_reason,
            owner_address: required_legacy_address(owner_address, "ownerAddress")?,
            voting_address,
            payout_address: required_legacy_address(payout_address, "payoutAddress")?,
            pub_key_operator,
            operator_payout_address,
            platform_node_id,
            platform_p2p_port,
            platform_http_port,
        })
    }
}

#[allow(deprecated)] // Restore the shipped flat-port representation.
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
            owner_address: Some(owner_address),
            voting_address,
            payout_address: Some(payout_address),
            pub_key_operator,
            operator_payout_address,
            platform_node_id,
            legacy_platform_p2p_port: platform_p2p_port,
            legacy_platform_http_port: platform_http_port,
            payouts: None,
            addresses: None,
        }
    }
}

/// Require the addresses that the pre-upgrade RPC parser required. This keeps
/// shipped identity and storage behavior unchanged for all legacy records.
pub(crate) fn required_legacy_address(
    address: Option<[u8; 20]>,
    field: &str,
) -> Result<[u8; 20], Error> {
    address.ok_or_else(|| {
        ExecutionError::DashCoreBadResponseError(format!(
            "masternode is missing required legacy {field}"
        ))
        .into()
    })
}

/// Validate before inserting a newly parsed Core record into Platform state.
pub(crate) fn validate_legacy_masternode(item: &MasternodeListItem) -> Result<(), Error> {
    required_legacy_address(item.collateral_address, "collateralAddress")?;
    required_legacy_address(item.state.owner_address, "ownerAddress")?;
    required_legacy_address(item.state.payout_address, "payoutAddress")?;
    Ok(())
}
