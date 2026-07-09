//! Connected-peer tracking and masternode-list classification.
//!
//! dash-spv does not expose the addresses of currently connected peers as
//! a query; it pushes them through `NetworkEvent::PeersUpdated` after every
//! connect/disconnect. [`PeerTracker`] mirrors the latest snapshot so
//! [`SpvRuntime`](super::SpvRuntime) can answer "who are we connected to?"
//! at any time, and classification against the masternode list turns each
//! address into Evonode / Masternode / Normal for display.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;

use dash_spv::network::NetworkEvent;
use dash_spv::EventHandler;
use dashcore::sml::masternode_list::MasternodeList;
use dashcore::sml::masternode_list_entry::EntryMasternodeType;

/// Node type of a connected SPV peer, resolved against the masternode list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpvPeerNodeType {
    /// The masternode list is not available yet, so the peer can't be
    /// classified. Distinct from [`SpvPeerNodeType::Normal`] so callers
    /// don't render "regular node" while the masternode phase is still
    /// syncing.
    Unknown,
    /// Not present in the masternode list.
    Normal,
    /// A regular masternode.
    Masternode,
    /// A high-performance (evo) masternode.
    Evonode,
}

/// A connected peer and its classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpvPeerInfo {
    pub address: SocketAddr,
    pub node_type: SpvPeerNodeType,
}

/// [`EventHandler`] that mirrors the most recent `PeersUpdated` snapshot.
///
/// Registered alongside the platform event manager on the SPV client's
/// handler list; every other event uses the trait's no-op default.
#[derive(Debug, Default)]
pub(crate) struct PeerTracker {
    connected: Mutex<Vec<SocketAddr>>,
}

impl PeerTracker {
    /// Addresses from the latest `PeersUpdated` event.
    pub(crate) fn snapshot(&self) -> Vec<SocketAddr> {
        self.connected
            .lock()
            .expect("peer tracker mutex poisoned")
            .clone()
    }

    /// Forget all peers. Called when the SPV client stops so a stale
    /// snapshot can't outlive the connections it describes.
    pub(crate) fn clear(&self) {
        self.connected
            .lock()
            .expect("peer tracker mutex poisoned")
            .clear();
    }
}

impl EventHandler for PeerTracker {
    fn on_network_event(&self, event: &NetworkEvent) {
        if let NetworkEvent::PeersUpdated { addresses, .. } = event {
            *self.connected.lock().expect("peer tracker mutex poisoned") = addresses.clone();
        }
    }
}

/// Classify peer addresses against a masternode list.
///
/// A peer is matched by full `ip:port` first, then by bare IP — the list
/// advertises the P2P endpoint we dialed, but an IP match alone is already
/// conclusive since masternodes don't share hosts with unrelated peers.
/// `None` (masternode list not yet synced) classifies every peer
/// [`SpvPeerNodeType::Unknown`].
pub(crate) fn classify_peers(
    addresses: &[SocketAddr],
    list: Option<&MasternodeList>,
) -> Vec<SpvPeerInfo> {
    let Some(list) = list else {
        return addresses
            .iter()
            .map(|&address| SpvPeerInfo {
                address,
                node_type: SpvPeerNodeType::Unknown,
            })
            .collect();
    };

    let mut by_socket: HashMap<SocketAddr, SpvPeerNodeType> = HashMap::new();
    let mut by_ip: HashMap<IpAddr, SpvPeerNodeType> = HashMap::new();
    for qualified in list.masternodes.values() {
        let entry = &qualified.masternode_list_entry;
        let Some(service) = entry.service_address.primary_service_address() else {
            continue;
        };
        let node_type = match entry.mn_type {
            EntryMasternodeType::Regular => SpvPeerNodeType::Masternode,
            EntryMasternodeType::HighPerformance { .. } => SpvPeerNodeType::Evonode,
        };
        by_socket.insert(service, node_type);
        by_ip.insert(service.ip(), node_type);
    }

    addresses
        .iter()
        .map(|&address| SpvPeerInfo {
            address,
            node_type: by_socket
                .get(&address)
                .or_else(|| by_ip.get(&address.ip()))
                .copied()
                .unwrap_or(SpvPeerNodeType::Normal),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};

    use dashcore::bls_sig_utils::BLSPublicKey;
    use dashcore::hashes::Hash;
    use dashcore::sml::masternode_list_entry::{MasternodeListEntry, MasternodeNetInfo};
    use dashcore::{BlockHash, ProTxHash, PubkeyHash};

    use super::*;

    fn socket(ip: [u8; 4], port: u16) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(ip), port))
    }

    fn masternode_list(entries: Vec<(SocketAddr, EntryMasternodeType)>) -> MasternodeList {
        let masternodes = entries
            .into_iter()
            .enumerate()
            .map(|(i, (service, mn_type))| {
                let mut hash_bytes = [0u8; 32];
                hash_bytes[0] = i as u8 + 1;
                let pro_tx_hash = ProTxHash::from_byte_array(hash_bytes);
                let entry = MasternodeListEntry {
                    version: 1,
                    pro_reg_tx_hash: pro_tx_hash,
                    confirmed_hash: None,
                    service_address: MasternodeNetInfo::Legacy(service),
                    operator_public_key: BLSPublicKey::from([0u8; 48]),
                    key_id_voting: PubkeyHash::from_byte_array([0u8; 20]),
                    is_valid: true,
                    mn_type,
                };
                (pro_tx_hash, entry.into())
            })
            .collect();
        MasternodeList::build(
            masternodes,
            Default::default(),
            BlockHash::from_byte_array([0u8; 32]),
            0,
        )
        .build()
    }

    #[test]
    fn no_masternode_list_classifies_unknown() {
        let peers = [socket([1, 2, 3, 4], 9999)];
        let infos = classify_peers(&peers, None);
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].node_type, SpvPeerNodeType::Unknown);
    }

    #[test]
    fn classifies_masternode_evonode_and_normal() {
        let regular = socket([10, 0, 0, 1], 9999);
        let evo = socket([10, 0, 0, 2], 9999);
        let normal = socket([10, 0, 0, 3], 9999);
        let list = masternode_list(vec![
            (regular, EntryMasternodeType::Regular),
            (
                evo,
                EntryMasternodeType::HighPerformance {
                    platform_http_port: 443,
                    platform_node_id: PubkeyHash::from_byte_array([0u8; 20]),
                },
            ),
        ]);

        let infos = classify_peers(&[regular, evo, normal], Some(&list));
        assert_eq!(infos[0].node_type, SpvPeerNodeType::Masternode);
        assert_eq!(infos[1].node_type, SpvPeerNodeType::Evonode);
        assert_eq!(infos[2].node_type, SpvPeerNodeType::Normal);
    }

    #[test]
    fn ip_match_with_different_port_still_classifies() {
        let listed = socket([10, 0, 0, 1], 9999);
        let dialed = socket([10, 0, 0, 1], 19999);
        let list = masternode_list(vec![(listed, EntryMasternodeType::Regular)]);

        let infos = classify_peers(&[dialed], Some(&list));
        assert_eq!(infos[0].node_type, SpvPeerNodeType::Masternode);
    }

    #[test]
    fn peers_updated_replaces_snapshot_and_clear_empties_it() {
        let tracker = PeerTracker::default();
        tracker.on_network_event(&NetworkEvent::PeersUpdated {
            connected_count: 1,
            addresses: vec![socket([1, 1, 1, 1], 9999)],
            best_height: Some(100),
        });
        assert_eq!(tracker.snapshot(), vec![socket([1, 1, 1, 1], 9999)]);

        tracker.on_network_event(&NetworkEvent::PeersUpdated {
            connected_count: 1,
            addresses: vec![socket([2, 2, 2, 2], 9999)],
            best_height: Some(100),
        });
        assert_eq!(tracker.snapshot(), vec![socket([2, 2, 2, 2], 9999)]);

        tracker.clear();
        assert!(tracker.snapshot().is_empty());
    }
}
