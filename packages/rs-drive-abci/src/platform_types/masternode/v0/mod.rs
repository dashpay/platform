/// Accessors for Masternode
pub mod accessors;

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
    // Core 23+ moved the platform ports into a nested `addresses` object and marked
    // the flat ports `legacy_*`. Resolve each port via DMNState's accessor, which
    // prefers the nested address and falls back to the legacy flat field: a Core 22
    // entry maps byte-identically, while a Core 23 entry (legacy = None) still
    // yields its port instead of being dropped and excluded from validator sets.
    // Platform state stores only the port; the host pairing is delegated to the
    // validator path.
    fn from(value: DMNState) -> Self {
        let platform_p2p_port = value.platform_p2p_address().map(|(_host, port)| port);
        let platform_http_port = value.platform_http_address().map(|(_host, port)| port);
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
            ..
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
    // Reverse of the conversion above: platform only holds the legacy port pair,
    // so the Core 23+ nested `addresses` are reconstructed as `None`.
    #[allow(deprecated)]
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
            legacy_platform_p2p_port: platform_p2p_port,
            legacy_platform_http_port: platform_http_port,
            addresses: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeAddresses};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[allow(deprecated)]
    fn base_dmn_state() -> DMNState {
        DMNState {
            service: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2)), 19999),
            registered_height: 1,
            pose_revived_height: None,
            pose_ban_height: None,
            revocation_reason: 0,
            owner_address: [0u8; 20],
            voting_address: [0u8; 20],
            payout_address: [0u8; 20],
            pub_key_operator: vec![1u8; 48],
            operator_payout_address: None,
            platform_node_id: Some([7u8; 20]),
            legacy_platform_p2p_port: None,
            legacy_platform_http_port: None,
            addresses: None,
        }
    }

    // A Core 23 masternode reports its platform ports in the nested `addresses`
    // object with the legacy flat fields absent. The DMNState -> MasternodeStateV0
    // conversion must resolve the port from `addresses`; reading only the empty
    // legacy field would drop the port and exclude the node from validator sets.
    #[test]
    fn from_dmn_state_resolves_core23_nested_platform_ports() {
        let mut dmn = base_dmn_state();
        dmn.addresses = Some(MasternodeAddresses {
            core_p2p: vec!["192.0.2.2:9999".to_string()],
            platform_p2p: vec!["192.0.2.2:36656".to_string()],
            platform_https: vec!["192.0.2.2:443".to_string()],
        });

        let state = MasternodeStateV0::from(dmn);
        assert_eq!(state.platform_p2p_port, Some(36656));
        assert_eq!(state.platform_http_port, Some(443));
    }

    // A Core 22 masternode reports the deprecated flat ports; the conversion stays
    // byte-identical by falling back to the legacy field.
    #[test]
    #[allow(deprecated)]
    fn from_dmn_state_falls_back_to_legacy_ports() {
        let dmn = DMNState {
            legacy_platform_p2p_port: Some(26656),
            legacy_platform_http_port: Some(8443),
            ..base_dmn_state()
        };

        let state = MasternodeStateV0::from(dmn);
        assert_eq!(state.platform_p2p_port, Some(26656));
        assert_eq!(state.platform_http_port, Some(8443));
    }
}
