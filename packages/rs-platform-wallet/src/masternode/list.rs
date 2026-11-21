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

use std::net::{IpAddr, SocketAddr, SocketAddrV4, SocketAddrV6};

use dashcore::sml::masternode_list::MasternodeList;
use dashcore::sml::masternode_list_entry::net_info::{Bip155Network, NetInfoEntry};
use dashcore::sml::masternode_list_entry::{
    EntryMasternodeType, MasternodeListEntry, MasternodeNetInfo,
};

use super::locator::MasternodeKeyReference;

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
    /// EVERY advertised endpoint with a socket form — the primary plus, for
    /// a v3 extended entry, the rest of its endpoint map across all
    /// purposes. Core registers each of them as a unique property of the
    /// masternode (`bad-protx-dup-netinfo-entry`), so uniqueness preflights
    /// must compare against all of these, never just `service_address`.
    /// Defaults to the primary alone on pre-field snapshots.
    pub service_addresses: Vec<SocketAddr>,
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
    /// The entry advertises v3 extended network info (an `ExtNetInfo`
    /// endpoint map) rather than a single legacy address —
    /// `service_address` is then only its primary endpoint, and the map
    /// itself is only on the raw list entry. Snapshots persisted before
    /// this field existed default to `false`.
    pub has_extended_net_info: bool,
    /// The entry is version 1 (pre-v19), so `operator_public_key` uses the
    /// LEGACY BLS serialization. A kept operator key re-entering a
    /// version-2 payload must then be reserialized to the basic scheme —
    /// the two serializations of one point differ only in flag bits, so
    /// this cannot be inferred from the bytes. Defaults to `false` on
    /// pre-field snapshots; the registrar path reads only live summaries.
    pub operator_key_is_legacy: bool,
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
            service_addresses: all_socket_addresses(&entry.service_address),
            platform_http_port,
            operator_public_key,
            voting_key_id,
            platform_node_id,
            is_valid: entry.is_valid,
            is_evonode,
            has_extended_net_info: matches!(entry.service_address, MasternodeNetInfo::Extended(_)),
            operator_key_is_legacy: entry.use_legacy_bls_keys(),
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

/// The first of `summaries` whose operator key is `key`, given in the basic
/// serialization and, when known, the legacy one. A version-1 entry holds
/// the legacy bytes of the same key, so a comparison against the basic bytes
/// alone would miss it.
///
/// The match is on the serialized bytes, whichever scheme the entry uses.
/// The registrar update refuses a key on this match, so the key picker
/// reports as taken exactly the keys that update would refuse.
pub(crate) fn operator_key_holder<'a>(
    summaries: impl IntoIterator<Item = &'a MasternodeListSummary>,
    key: &[u8; 48],
    key_legacy: Option<&[u8; 48]>,
) -> Option<&'a MasternodeListSummary> {
    summaries.into_iter().find(|entry| {
        entry.operator_public_key == *key
            || key_legacy.is_some_and(|legacy| entry.operator_public_key == *legacy)
    })
}

/// The masternode list while it follows the chain: one snapshot's height
/// and entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentMasternodeList {
    /// Core block height the list describes.
    pub height: u32,
    /// Every entry of the list, as [`MasternodeListSummary::all_from_list`].
    pub summaries: Vec<MasternodeListSummary>,
}

impl CurrentMasternodeList {
    /// Height and summaries of `list`. Crate-private: only a gated read
    /// knows the list is current.
    pub(crate) fn from_list(list: &MasternodeList) -> Self {
        Self {
            height: list.known_height,
            summaries: MasternodeListSummary::all_from_list(list),
        }
    }

    /// The keys the list knows for `pro_tx_hash` (wire order): voting,
    /// operator, platform node. `None` when the node is not on this list.
    pub fn key_reference(&self, pro_tx_hash: &[u8; 32]) -> Option<MasternodeKeyReference> {
        self.summaries
            .iter()
            .find(|summary| &summary.pro_tx_hash == pro_tx_hash)
            .map(MasternodeKeyReference::from_summary)
    }
}

/// Every endpoint of `net_info` with a socket form, across all purposes —
/// the shape Core's unique-property index holds them in (each entry of an
/// extended map is registered individually). Tor / I2P / CJDNS / domain
/// entries have no `SocketAddr` form and are skipped; a caller-supplied
/// `ip:port` value can never collide with them anyway.
pub(crate) fn all_socket_addresses(net_info: &MasternodeNetInfo) -> Vec<SocketAddr> {
    match net_info {
        MasternodeNetInfo::Legacy(addr) => vec![*addr],
        MasternodeNetInfo::Extended(info) => info
            .purposes
            .iter()
            .flat_map(|(_, entries)| entries.iter())
            .filter_map(socket_address)
            .collect(),
    }
}

/// The socket form of one address-map entry: an IPv4 or IPv6 service entry
/// whose address has its network's length. Tor / I2P / CJDNS, domain and
/// invalid entries have none.
///
/// The one place this crate reads an entry's IP; rust-dashcore decodes the
/// same two arms privately inside `primary_service_address`.
pub(crate) fn socket_address(entry: &NetInfoEntry) -> Option<SocketAddr> {
    match entry {
        NetInfoEntry::Service {
            network: Bip155Network::Ipv4,
            addr,
            port,
        } => {
            let octets: [u8; 4] = addr.as_slice().try_into().ok()?;
            Some(SocketAddr::V4(SocketAddrV4::new(octets.into(), *port)))
        }
        NetInfoEntry::Service {
            network: Bip155Network::Ipv6,
            addr,
            port,
        } => {
            let octets: [u8; 16] = addr.as_slice().try_into().ok()?;
            Some(SocketAddr::V6(SocketAddrV6::new(
                octets.into(),
                *port,
                0,
                0,
            )))
        }
        _ => None,
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
    //! Synthetic list summaries for locator tests, and the raw list entries
    //! behind them for the paths that read entries. Kept `pub(crate)` so the
    //! locator and update-service tests build lists the same way.
    use super::{EntryMasternodeType, MasternodeListEntry, MasternodeListSummary};
    use dashcore::bls_sig_utils::BLSPublicKey;
    use dashcore::sml::masternode_list_entry::MasternodeNetInfo;
    use dashcore::{PlatformNodeId, ProTxHash, PubkeyHash};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

    /// A regular masternode at `10.0.0.<seed>:9999` whose proTxHash,
    /// operator key and voting key id are all derived from `seed` so every
    /// entry is distinct and recognizable.
    pub(crate) fn masternode(seed: u8) -> MasternodeListSummary {
        let primary = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, seed), 9999));
        MasternodeListSummary {
            pro_tx_hash: [seed; 32],
            service_address: Some(primary),
            service_addresses: vec![primary],
            platform_http_port: None,
            operator_public_key: [seed; 48],
            voting_key_id: [seed; 20],
            platform_node_id: None,
            is_valid: true,
            is_evonode: false,
            has_extended_net_info: false,
            operator_key_is_legacy: false,
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

    /// The raw list entry [`masternode`] summarizes: a version-2 regular
    /// masternode with a legacy address.
    pub(crate) fn masternode_entry(seed: u8) -> MasternodeListEntry {
        MasternodeListEntry {
            version: 2,
            pro_reg_tx_hash: ProTxHash::from_byte_array([seed; 32]),
            confirmed_hash: None,
            service_address: MasternodeNetInfo::Legacy(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(10, 0, 0, seed),
                9999,
            ))),
            operator_public_key: BLSPublicKey::from([seed; 48]),
            key_id_voting: PubkeyHash::from_byte_array([seed; 20]),
            is_valid: true,
            mn_type: EntryMasternodeType::Regular,
        }
    }

    /// The raw list entry [`evonode`] summarizes.
    pub(crate) fn evonode_entry(seed: u8) -> MasternodeListEntry {
        MasternodeListEntry {
            mn_type: EntryMasternodeType::HighPerformance {
                platform_http_port: 443,
                platform_node_id: PlatformNodeId::from_byte_array([seed ^ 0xFF; 20]),
            },
            ..masternode_entry(seed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{evonode, evonode_entry, ip, masternode, masternode_entry};
    use super::*;
    use dashcore::bls_sig_utils::BLSPublicKey;
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
            [(pro_tx, std::sync::Arc::new(entry.into()))]
                .into_iter()
                .collect::<dashcore::sml::masternode_list::MasternodeMap>(),
            std::collections::BTreeMap::new(),
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
        assert_eq!(
            s.service_addresses,
            vec!["1.2.3.4:19999".parse::<SocketAddr>().unwrap()],
            "a legacy entry's endpoint list is its single address"
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

    /// The extended-map conversion is what feeds the service-uniqueness
    /// preflight its secondary endpoints: every socket entry under every
    /// purpose must be lifted (Core's unique-property index registers each
    /// one individually), while domain / invalid entries — which have no
    /// socket form — are skipped. The lifted secondary endpoint must then
    /// actually collide in the values validator.
    #[test]
    fn summary_lifts_every_extended_endpoint() {
        use super::super::update_service::validate_update_service_values;
        use dashcore::sml::masternode_list_entry::net_info::{
            Bip155Network, ExtNetInfo, NetInfoEntry, NetInfoPurpose,
        };
        use dashcore::Network;

        let mut ipv6 = [0u8; 16];
        ipv6[0] = 0x20;
        ipv6[1] = 0x01;
        ipv6[15] = 0x01;
        let entry = MasternodeListEntry {
            version: 2,
            pro_reg_tx_hash: ProTxHash::from_byte_array([8u8; 32]),
            confirmed_hash: None,
            service_address: MasternodeNetInfo::Extended(ExtNetInfo {
                version: 1,
                purposes: vec![
                    (
                        NetInfoPurpose::CoreP2P,
                        vec![
                            NetInfoEntry::Service {
                                network: Bip155Network::Ipv4,
                                addr: vec![34, 214, 48, 68],
                                port: 19999,
                            },
                            NetInfoEntry::Service {
                                network: Bip155Network::Ipv4,
                                addr: vec![34, 214, 48, 69],
                                port: 29999,
                            },
                            NetInfoEntry::Domain {
                                host: "node.example".to_string(),
                                port: 19999,
                            },
                        ],
                    ),
                    (
                        NetInfoPurpose::PlatformHttps,
                        vec![
                            NetInfoEntry::Service {
                                network: Bip155Network::Ipv6,
                                addr: ipv6.to_vec(),
                                port: 443,
                            },
                            NetInfoEntry::Invalid,
                        ],
                    ),
                ],
            }),
            operator_public_key: BLSPublicKey::from([9u8; 48]),
            key_id_voting: PubkeyHash::from_byte_array([5u8; 20]),
            is_valid: true,
            mn_type: EntryMasternodeType::Regular,
        };

        let summary = MasternodeListSummary::from_entry(&entry);
        assert!(summary.has_extended_net_info);
        assert_eq!(
            summary.service_addresses,
            vec![
                "34.214.48.68:19999".parse::<SocketAddr>().unwrap(),
                "34.214.48.69:29999".parse::<SocketAddr>().unwrap(),
                "[2001::1]:443".parse::<SocketAddr>().unwrap(),
            ],
            "every socket endpoint under every purpose is lifted; \
             domain and invalid entries are skipped"
        );

        // A caller-supplied value equal to the extended entry's SECONDARY
        // endpoint is refused by the uniqueness preflight — the join this
        // converter exists to feed.
        let target = masternode(0x60);
        let summaries = vec![target.clone(), summary];
        let values = super::super::update_service::UpdateServiceValues {
            service_address: "34.214.48.69:29999".to_string(),
            platform_node_id: None,
            platform_p2p_port: None,
            platform_http_port: None,
        };
        validate_update_service_values(Network::Testnet, &target, &values, &summaries)
            .expect_err("another entry's secondary extended endpoint must collide");
    }

    /// An entry has a socket form only as an IPv4 or IPv6 service whose
    /// address has that network's length; rust-dashcore's decoder holds wire
    /// entries to it, but an entry built in code is not checked.
    #[test]
    fn should_read_a_socket_address_only_from_a_well_formed_ip_service_entry() {
        use dashcore::sml::masternode_list_entry::net_info::{Bip155Network, NetInfoEntry};

        let service = |network, addr: Vec<u8>| NetInfoEntry::Service {
            network,
            addr,
            port: 19999,
        };
        assert_eq!(
            socket_address(&service(Bip155Network::Ipv4, vec![34, 214, 48, 68])),
            Some("34.214.48.68:19999".parse().unwrap())
        );
        let mut ipv6 = vec![0u8; 16];
        ipv6[0] = 0x20;
        ipv6[1] = 0x01;
        ipv6[15] = 0x01;
        assert_eq!(
            socket_address(&service(Bip155Network::Ipv6, ipv6)),
            Some("[2001::1]:19999".parse().unwrap())
        );

        for entry in [
            service(Bip155Network::Ipv4, vec![34, 214, 48]),
            service(Bip155Network::Ipv6, vec![34, 214, 48, 68]),
            service(Bip155Network::TorV3, vec![7; 32]),
            service(Bip155Network::Cjdns, vec![0xfc; 16]),
            NetInfoEntry::Domain {
                host: "node.example".to_string(),
                port: 19999,
            },
            NetInfoEntry::Invalid,
        ] {
            assert_eq!(socket_address(&entry), None, "{entry:?}");
        }
    }

    /// A list entry holds one serialization of its operator key, so a wallet
    /// key is looked up under both.
    #[test]
    fn should_find_the_holder_of_an_operator_key_under_either_serialization() {
        let mut basic = masternode(1);
        basic.operator_public_key = [0xAA; 48];
        let mut legacy = masternode(2);
        legacy.operator_public_key = [0xBB; 48];
        let list = vec![basic, legacy];

        let holder = |key: [u8; 48], key_legacy: Option<[u8; 48]>| {
            operator_key_holder(&list, &key, key_legacy.as_ref()).map(|entry| entry.pro_tx_hash)
        };
        assert_eq!(holder([0xAA; 48], None), Some([1; 32]));
        assert_eq!(holder([0xCC; 48], Some([0xBB; 48])), Some([2; 32]));
        assert_eq!(holder([0xBB; 48], None), Some([2; 32]));
        assert_eq!(holder([0xCC; 48], Some([0xDD; 48])), None);
        assert_eq!(
            operator_key_holder(list.iter().skip(1), &[0xAA; 48], None),
            None,
            "only the entries given are searched"
        );
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

    /// Height and entries come from the same list.
    #[test]
    fn should_carry_the_list_height_with_its_summaries() {
        let empty = MasternodeList::empty(BlockHash::from_byte_array([1u8; 32]), 2_345_678);
        assert_eq!(
            CurrentMasternodeList::from_list(&empty),
            CurrentMasternodeList {
                height: 2_345_678,
                summaries: Vec::new(),
            }
        );

        let list = MasternodeList::build(
            [masternode_entry(1), evonode_entry(2)]
                .into_iter()
                .map(|entry| (entry.pro_reg_tx_hash, std::sync::Arc::new(entry.into())))
                .collect::<dashcore::sml::masternode_list::MasternodeMap>(),
            std::collections::BTreeMap::new(),
            BlockHash::from_byte_array([2u8; 32]),
            1_000,
        )
        .build();
        assert_eq!(
            CurrentMasternodeList::from_list(&list),
            CurrentMasternodeList {
                height: 1_000,
                summaries: vec![masternode(1), evonode(2)],
            }
        );
    }

    /// A listed node gets the keys the list knows; any other node, nothing.
    #[test]
    fn should_give_a_key_reference_only_for_a_listed_node() {
        let current = CurrentMasternodeList {
            height: 1,
            summaries: vec![masternode(1), evonode(2)],
        };

        assert_eq!(
            current.key_reference(&[2u8; 32]),
            Some(MasternodeKeyReference {
                voting_key_id: Some([2u8; 20]),
                operator_public_key: Some([2u8; 48]),
                platform_node_id: Some([2u8 ^ 0xFF; 20]),
                ..Default::default()
            })
        );
        assert_eq!(current.key_reference(&[3u8; 32]), None);
    }
}
