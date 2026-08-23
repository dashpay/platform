//! Typed access to the deterministic masternode list (DML) for lookups.
//!
//! The SML entry dash-spv holds for each masternode carries its proTxHash,
//! service address, operator BLS key, voting key id, validity and (for
//! evonodes) the platform node id and HTTP port — but **not** the owner key
//! hash, payout script, collateral or registration height; those live only
//! in the provider transactions and on Platform's masternode identities.
//! [`MasternodeListSummary`] is exactly what the list knows, typed
//! (`SocketAddr`, not `"ip:port"`), and [`MasternodeListQuery`] is every way
//! a host can ask the list for a masternode from a user-supplied locator.
//!
//! Lookups are pure over a snapshot (`Vec<MasternodeListSummary>`, ~4 k
//! entries on mainnet) so they are unit-testable without a live engine and
//! never hold the engine lock while a host iterates results.

use std::net::{IpAddr, SocketAddr};

use dashcore::sml::masternode_list::MasternodeList;
use dashcore::sml::masternode_list_entry::{EntryMasternodeType, MasternodeListEntry};

/// What the deterministic masternode list knows about one masternode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasternodeListSummary {
    /// proTxHash in **wire order** — the same orientation as
    /// `MasternodeRecord::pro_tx_hash` and `MasternodeEntryFFI.pro_tx_hash`
    /// (a `Txid`'s bytes; explorers, Tenderdash and Platform identity ids
    /// show the reversal).
    pub pro_tx_hash: [u8; 32],
    /// Primary routable Core P2P endpoint. `None` for Tor / I2P / CJDNS /
    /// domain-only entries, which have no `SocketAddr` form.
    pub service_address: Option<SocketAddr>,
    /// Platform HTTP (DAPI gRPC) port — evonodes only.
    pub platform_http_port: Option<u16>,
    /// Operator BLS public key (48 bytes, as serialized in the list — the
    /// basic scheme for v2+ entries, legacy for v1).
    pub operator_public_key: [u8; 48],
    /// Voting key id (hash160 of the voting public key).
    pub voting_key_id: [u8; 20],
    /// Tenderdash node id (`SHA256(ed25519 pk)[..20]`, canonical order) —
    /// evonodes only.
    pub platform_node_id: Option<[u8; 20]>,
    /// `false` when the entry is PoSe-banned.
    pub is_valid: bool,
    /// High-performance (evonode) entry.
    pub is_evonode: bool,
}

impl MasternodeListSummary {
    /// Lift the list entry into the typed summary.
    pub fn from_entry(entry: &MasternodeListEntry) -> Self {
        let mut pro_tx_hash = [0u8; 32];
        // `pro_reg_tx_hash` on a consensus-decoded entry is the wire
        // orientation (the DML map keys by the reversed/display form, so
        // read it off the entry, never the map key).
        pro_tx_hash.copy_from_slice(entry.pro_reg_tx_hash.as_ref());
        let mut operator_public_key = [0u8; 48];
        operator_public_key.copy_from_slice(entry.operator_public_key.as_ref());
        let mut voting_key_id = [0u8; 20];
        voting_key_id.copy_from_slice(entry.key_id_voting.as_ref());
        let (platform_http_port, platform_node_id, is_evonode) = match &entry.mn_type {
            EntryMasternodeType::Regular => (None, None, false),
            EntryMasternodeType::HighPerformance {
                platform_http_port,
                platform_node_id,
            } => (
                Some(*platform_http_port),
                Some(platform_node_id.to_byte_array()),
                true,
            ),
        };
        Self {
            pro_tx_hash,
            service_address: entry.service_address.primary_service_address(),
            platform_http_port,
            operator_public_key,
            voting_key_id,
            platform_node_id,
            is_valid: entry.is_valid,
            is_evonode,
        }
    }

    /// Every entry of `list` as a summary, in the list's (proTxHash map)
    /// order.
    pub fn all_from_list(list: &MasternodeList) -> Vec<Self> {
        list.masternodes
            .values()
            .map(|qualified| Self::from_entry(&qualified.masternode_list_entry))
            .collect()
    }

    /// proTxHash in display (explorer / Tenderdash / Platform identity id)
    /// orientation.
    pub fn pro_tx_hash_display(&self) -> [u8; 32] {
        let mut out = self.pro_tx_hash;
        out.reverse();
        out
    }
}

/// One way of asking the list for a masternode. Every variant is matched
/// against the list's own fields — nothing here needs the wallet or the
/// network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasternodeListQuery {
    /// proTxHash in wire order.
    ProTxHash([u8; 32]),
    /// Service IP, optionally pinned to a port. Without a port any port
    /// matches (a host that pastes a bare IP rarely knows the P2P port).
    ServiceAddress { ip: IpAddr, port: Option<u16> },
    /// hash160 of a voting public key.
    VotingKeyId([u8; 20]),
    /// Operator BLS public key, in whichever serialization the caller
    /// has — callers that derive from a secret should query both the basic
    /// and the legacy form.
    OperatorPublicKey([u8; 48]),
    /// Tenderdash node id (`SHA256(ed25519 pk)[..20]`).
    PlatformNodeId([u8; 20]),
}

impl MasternodeListQuery {
    /// Whether `summary` satisfies this query.
    pub fn matches(&self, summary: &MasternodeListSummary) -> bool {
        match self {
            Self::ProTxHash(hash) => &summary.pro_tx_hash == hash,
            Self::ServiceAddress { ip, port } => match summary.service_address {
                Some(addr) => addr.ip() == *ip && port.map(|p| p == addr.port()).unwrap_or(true),
                None => false,
            },
            Self::VotingKeyId(id) => &summary.voting_key_id == id,
            Self::OperatorPublicKey(key) => &summary.operator_public_key == key,
            Self::PlatformNodeId(id) => summary.platform_node_id.as_ref() == Some(id),
        }
    }
}

/// Every summary in `summaries` matching `query`, in input order.
pub fn find_in_summaries<'a>(
    summaries: &'a [MasternodeListSummary],
    query: &MasternodeListQuery,
) -> Vec<&'a MasternodeListSummary> {
    summaries.iter().filter(|s| query.matches(s)).collect()
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Synthetic list summaries for locator tests. Kept `pub(crate)` so the
    //! locator tests build lists the same way.
    use super::MasternodeListSummary;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

    /// A regular masternode at `10.0.0.<seed>:9999` whose proTxHash,
    /// operator key and voting key id are all derived from `seed` so every
    /// entry is distinct and recognizable.
    pub(crate) fn masternode(seed: u8) -> MasternodeListSummary {
        MasternodeListSummary {
            pro_tx_hash: [seed; 32],
            service_address: Some(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(10, 0, 0, seed),
                9999,
            ))),
            platform_http_port: None,
            operator_public_key: [seed; 48],
            voting_key_id: [seed; 20],
            platform_node_id: None,
            is_valid: true,
            is_evonode: false,
        }
    }

    /// An evonode variant of [`masternode`] with a platform node id and
    /// HTTP port.
    pub(crate) fn evonode(seed: u8) -> MasternodeListSummary {
        MasternodeListSummary {
            platform_http_port: Some(443),
            platform_node_id: Some([seed ^ 0xFF; 20]),
            is_evonode: true,
            ..masternode(seed)
        }
    }

    pub(crate) fn ip(seed: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, seed))
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{evonode, ip, masternode};
    use super::*;
    use dashcore::bls_sig_utils::BLSPublicKey;
    use dashcore::hashes::Hash;
    use dashcore::sml::masternode_list_entry::MasternodeNetInfo;
    use dashcore::{BlockHash, PlatformNodeId, ProTxHash, PubkeyHash};
    use std::net::{Ipv4Addr, SocketAddrV4};

    #[test]
    fn summary_lifts_every_field_from_a_list_entry() {
        let pro_tx = ProTxHash::from_byte_array([7u8; 32]);
        let entry = MasternodeListEntry {
            version: 2,
            pro_reg_tx_hash: pro_tx,
            confirmed_hash: None,
            service_address: MasternodeNetInfo::Legacy(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(1, 2, 3, 4),
                19999,
            ))),
            operator_public_key: BLSPublicKey::from([9u8; 48]),
            key_id_voting: PubkeyHash::from_byte_array([5u8; 20]),
            is_valid: false,
            mn_type: EntryMasternodeType::HighPerformance {
                platform_http_port: 1443,
                platform_node_id: PlatformNodeId::from_byte_array([3u8; 20]),
            },
        };
        let list = MasternodeList::build(
            [(pro_tx, entry.into())].into_iter().collect(),
            Default::default(),
            BlockHash::from_byte_array([0u8; 32]),
            0,
        )
        .build();

        let summaries = MasternodeListSummary::all_from_list(&list);
        assert_eq!(summaries.len(), 1);
        let s = &summaries[0];
        assert_eq!(s.pro_tx_hash, [7u8; 32]);
        assert_eq!(
            s.service_address,
            Some("1.2.3.4:19999".parse::<SocketAddr>().unwrap())
        );
        assert_eq!(s.platform_http_port, Some(1443));
        assert_eq!(s.operator_public_key, [9u8; 48]);
        assert_eq!(s.voting_key_id, [5u8; 20]);
        assert_eq!(s.platform_node_id, Some([3u8; 20]));
        assert!(!s.is_valid, "PoSe-banned entry stays invalid");
        assert!(s.is_evonode);
        let mut display = [7u8; 32];
        display.reverse();
        assert_eq!(s.pro_tx_hash_display(), display);
    }

    #[test]
    fn regular_entry_has_no_platform_fields() {
        let s = masternode(1);
        assert!(!s.is_evonode);
        assert_eq!(s.platform_node_id, None);
        assert_eq!(s.platform_http_port, None);
    }

    #[test]
    fn finds_by_pro_tx_hash() {
        let list = vec![masternode(1), masternode(2), evonode(3)];
        let hits = find_in_summaries(&list, &MasternodeListQuery::ProTxHash([2u8; 32]));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].pro_tx_hash, [2u8; 32]);
        assert!(find_in_summaries(&list, &MasternodeListQuery::ProTxHash([9u8; 32])).is_empty());
    }

    #[test]
    fn finds_by_ip_with_or_without_port() {
        let list = vec![masternode(1), masternode(2)];
        let any_port = MasternodeListQuery::ServiceAddress {
            ip: ip(2),
            port: None,
        };
        assert_eq!(find_in_summaries(&list, &any_port).len(), 1);
        let right_port = MasternodeListQuery::ServiceAddress {
            ip: ip(2),
            port: Some(9999),
        };
        assert_eq!(find_in_summaries(&list, &right_port).len(), 1);
        let wrong_port = MasternodeListQuery::ServiceAddress {
            ip: ip(2),
            port: Some(19999),
        };
        assert!(
            find_in_summaries(&list, &wrong_port).is_empty(),
            "a pinned port must match exactly"
        );
        let unknown_ip = MasternodeListQuery::ServiceAddress {
            ip: ip(200),
            port: None,
        };
        assert!(find_in_summaries(&list, &unknown_ip).is_empty());
    }

    #[test]
    fn ip_query_skips_entries_without_a_socket_address() {
        let mut tor_only = masternode(4);
        tor_only.service_address = None;
        let list = vec![tor_only];
        let q = MasternodeListQuery::ServiceAddress {
            ip: ip(4),
            port: None,
        };
        assert!(find_in_summaries(&list, &q).is_empty());
    }

    #[test]
    fn finds_every_masternode_sharing_a_voting_key() {
        let mut shared_a = masternode(1);
        shared_a.voting_key_id = [0xAA; 20];
        let mut shared_b = masternode(2);
        shared_b.voting_key_id = [0xAA; 20];
        let list = vec![shared_a, shared_b, masternode(3)];
        let hits = find_in_summaries(&list, &MasternodeListQuery::VotingKeyId([0xAA; 20]));
        assert_eq!(hits.len(), 2, "shared voting keys return every node");
    }

    #[test]
    fn finds_by_operator_key_and_platform_node_id() {
        let list = vec![masternode(1), evonode(2), evonode(3)];
        let by_op = find_in_summaries(&list, &MasternodeListQuery::OperatorPublicKey([2u8; 48]));
        assert_eq!(by_op.len(), 1);
        assert!(by_op[0].is_evonode);
        let by_node = find_in_summaries(
            &list,
            &MasternodeListQuery::PlatformNodeId([3u8 ^ 0xFF; 20]),
        );
        assert_eq!(by_node.len(), 1);
        assert_eq!(by_node[0].pro_tx_hash, [3u8; 32]);
        // A regular masternode never matches a node-id query.
        assert!(
            find_in_summaries(&list, &MasternodeListQuery::PlatformNodeId([1u8; 20])).is_empty()
        );
    }
}
