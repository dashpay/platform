use dpp::bincode::{Decode, Encode};
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeAddresses, MasternodeType};
use dpp::dashcore_rpc::json::MasternodeListItem;
use std::fmt::{Debug, Formatter};

use dpp::dashcore::{ProTxHash, Txid};

use std::net::SocketAddr;

/// `MasternodeV1` represents a masternode on the network. It differs from
/// [`super::v0::MasternodeV0`] only in its `state` ([`MasternodeStateV1`]), which
/// persists the Core 23 platform host so a split platform/core host survives a restart.
#[derive(Clone, PartialEq, Encode, Decode)]
pub struct MasternodeV1 {
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
    pub state: MasternodeStateV1,
}

impl Debug for MasternodeV1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MasternodeV1")
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

impl From<MasternodeListItem> for MasternodeV1 {
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

impl From<MasternodeV1> for MasternodeListItem {
    fn from(value: MasternodeV1) -> Self {
        let MasternodeV1 {
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

/// A `MasternodeStateV1` contains information about a masternode's state. It extends
/// [`super::v0::MasternodeStateV0`] with `platform_host`: Core 23 (`DEPLOYMENT_V24`)
/// decouples the platform endpoints from the core service address, so the resolved
/// platform host can differ from `service.ip()`. V0 stored only the ports and so lost
/// the host across a restart.
#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct MasternodeStateV1 {
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

    /// The host the platform endpoints resolve to (Core 23 nested `addresses`). For a
    /// Core 22 / legacy node it equals the core service IP; `None` when no platform p2p
    /// address resolves (non-HPMN), in which case the reverse conversion falls back to
    /// `service.ip()`.
    pub platform_host: Option<String>,
}

impl From<DMNState> for MasternodeStateV1 {
    // Resolve the platform ports via DMNState's accessors (Core 23 nested `addresses`
    // preferred, legacy flat fields as fallback) and capture the platform p2p host.
    // Unlike V0, the host is retained so a Core 23 node whose platform host differs from
    // its core service IP keeps advertising the correct endpoint after a restart.
    fn from(value: DMNState) -> Self {
        let platform_p2p = value.platform_p2p_address();
        let platform_http = value.platform_http_address();
        let platform_http_port = platform_http.as_ref().map(|(_host, port)| *port);
        // Prefer the p2p host (what the validator advertises); fall back to the http host
        // so an http-only entry still round-trips its host instead of collapsing to
        // service.ip() on the reverse conversion.
        let platform_host = platform_p2p
            .as_ref()
            .map(|(host, _port)| host.clone())
            .or_else(|| platform_http.as_ref().map(|(host, _port)| host.clone()));
        let platform_p2p_port = platform_p2p.map(|(_host, port)| port);
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
            platform_host,
        }
    }
}

impl From<MasternodeStateV1> for DMNState {
    // Reverse of the conversion above (used by the persisted-state round-trip).
    // Reconstruct the Core 23 nested `addresses`, pairing each stored port with the
    // retained `platform_host` (falling back to `service.ip()` for a Core 22 / legacy
    // node that has no distinct host — identical to the V0 behavior and to the host the
    // live validator path would advertise for such a node). Build `addresses` only when
    // a platform port is present, and leave `legacy_*` ports `None` so a later
    // `addresses: Some(None)` clear diff actually drops the platform endpoint instead of
    // being masked by a stale legacy port.
    #[allow(deprecated)]
    fn from(value: MasternodeStateV1) -> Self {
        let MasternodeStateV1 {
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
            platform_host,
        } = value;

        let host = platform_host.unwrap_or_else(|| service.ip().to_string());
        let addresses = (platform_p2p_port.is_some() || platform_http_port.is_some()).then(|| {
            MasternodeAddresses {
                core_p2p: vec![],
                platform_p2p: platform_p2p_port
                    .map(|port| format!("{host}:{port}"))
                    .into_iter()
                    .collect(),
                platform_https: platform_http_port
                    .map(|port| format!("{host}:{port}"))
                    .into_iter()
                    .collect(),
            }
        });

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
            legacy_platform_p2p_port: None,
            legacy_platform_http_port: None,
            addresses,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    // A Core 23 masternode whose platform host differs from its core service IP must
    // survive the persisted round-trip with the platform host intact. V0 collapses it to
    // the service IP (the bug this version fixes); V1 preserves it. The contrast is the
    // red→green proof: the same input loses the host through V0 and keeps it through V1.
    #[test]
    #[allow(deprecated)]
    fn v1_round_trip_preserves_distinct_platform_host() {
        use crate::platform_types::masternode::v0::MasternodeStateV0;

        let dmn = DMNState {
            legacy_platform_p2p_port: Some(0),
            legacy_platform_http_port: Some(0),
            addresses: Some(MasternodeAddresses {
                core_p2p: vec![],
                platform_p2p: vec!["203.0.113.7:36656".to_string()],
                platform_https: vec!["203.0.113.7:443".to_string()],
            }),
            ..base_dmn_state() // service IP is 192.0.2.2 — deliberately different
        };

        // V0 loses the platform host: the reverse conversion pairs the port with the
        // core service IP.
        let v0_back: DMNState = MasternodeStateV0::from(dmn.clone()).into();
        assert_eq!(v0_back.platform_p2p_address().expect("port resolves").0, "192.0.2.2");

        // V1 captures and restores the platform host.
        let state = MasternodeStateV1::from(dmn.clone());
        assert_eq!(state.platform_host.as_deref(), Some("203.0.113.7"));
        assert_eq!(state.platform_p2p_port, Some(36656));
        assert_eq!(state.platform_http_port, Some(443));

        let v1_back: DMNState = state.into();
        let (host, port) = v1_back.platform_p2p_address().expect("port resolves");
        assert_eq!(host, "203.0.113.7");
        assert_eq!(port, 36656);
        assert_eq!(v1_back.platform_http_address().expect("http resolves").0, "203.0.113.7");
    }

    // A Core 22 / legacy node has no distinct platform host: the conversion captures
    // `platform_host = None` and the reverse falls back to the core service IP, matching
    // V0 behavior exactly.
    #[test]
    #[allow(deprecated)]
    fn v1_legacy_node_falls_back_to_service_ip() {
        let dmn = DMNState {
            legacy_platform_p2p_port: Some(26656),
            legacy_platform_http_port: Some(8443),
            ..base_dmn_state()
        };

        let state = MasternodeStateV1::from(dmn);
        // The legacy accessor pairs the port with the service IP, so the captured host is
        // the service IP — not `None`.
        assert_eq!(state.platform_host.as_deref(), Some("192.0.2.2"));
        assert_eq!(state.platform_p2p_port, Some(26656));

        let back: DMNState = state.into();
        let (host, port) = back.platform_p2p_address().expect("port resolves");
        assert_eq!(host, "192.0.2.2");
        assert_eq!(port, 26656);
    }

    // An http-only entry (empty platform_p2p) still round-trips its host: `platform_host`
    // falls back to the http host instead of collapsing to service.ip() on reverse.
    #[test]
    #[allow(deprecated)]
    fn v1_http_only_node_preserves_host() {
        let dmn = DMNState {
            addresses: Some(MasternodeAddresses {
                core_p2p: vec![],
                platform_p2p: vec![],
                platform_https: vec!["203.0.113.7:443".to_string()],
            }),
            ..base_dmn_state() // service IP is 192.0.2.2
        };

        let state = MasternodeStateV1::from(dmn);
        assert_eq!(state.platform_host.as_deref(), Some("203.0.113.7"));
        assert_eq!(state.platform_p2p_port, None);
        assert_eq!(state.platform_http_port, Some(443));

        let back: DMNState = state.into();
        // Host preserved (would be 192.0.2.2 without the http fallback).
        assert_eq!(back.platform_http_address().expect("http resolves").0, "203.0.113.7");
    }

    // The full save/load path goes MasternodeListItem -> MasternodeV1 -> MasternodeListItem;
    // the outer conversions are pure field moves, so the platform host survives end-to-end.
    #[test]
    #[allow(deprecated)]
    fn v1_masternode_list_item_round_trip_preserves_host() {
        let dmn = DMNState {
            addresses: Some(MasternodeAddresses {
                core_p2p: vec![],
                platform_p2p: vec!["203.0.113.7:36656".to_string()],
                platform_https: vec!["203.0.113.7:443".to_string()],
            }),
            ..base_dmn_state()
        };
        let item = MasternodeListItem {
            node_type: MasternodeType::Evo,
            pro_tx_hash: ProTxHash::from([0u8; 32]),
            collateral_hash: Txid::from([0u8; 32]),
            collateral_index: 0,
            collateral_address: [0u8; 20],
            operator_reward: 0.0,
            state: dmn,
        };

        let persisted: MasternodeV1 = item.into();
        assert_eq!(persisted.state.platform_host.as_deref(), Some("203.0.113.7"));

        let restored: MasternodeListItem = persisted.into();
        assert_eq!(
            restored.state.platform_p2p_address().expect("port resolves").0,
            "203.0.113.7"
        );
    }
}
