//! Masternode records: the wallet-side model of a masternode / evonode.
//!
//! [`aggregate_masternodes`] folds a wallet's retained provider special
//! transactions (ProRegTx / ProUpServTx / ProUpRegTx / ProUpRevTx) into one
//! [`MasternodeRecord`] per proTxHash, resolving the displayed
//! [`MasternodeStatus`] against the deterministic masternode list through an
//! injected [`ListMembership`] lookup. Everything here is pure and
//! host-agnostic; the FFI crates only marshal the results.

use std::net::IpAddr;

use dashcore::blockdata::transaction::special_transaction::provider_registration::MasternodePayoutShare;
use dashcore::sml::masternode_list_entry::net_info::{Bip155Network, NetInfoEntry, NetInfoPurpose};
use dashcore::sml::masternode_list_entry::MasternodeNetInfo;

/// Fixed-size hash copies. `Txid` / `PubkeyHash` are exactly 32 / 20
/// bytes, so `copy_from_slice` on `as_ref()` is length-exact and cannot
/// panic — the same pattern `tx_record_to_ffi`'s txid copy relies on.
pub(crate) fn provider_hash_to_32(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(bytes);
    out
}

pub(crate) fn provider_hash_to_20(bytes: &[u8]) -> [u8; 20] {
    let mut out = [0u8; 20];
    out.copy_from_slice(bytes);
    out
}

/// Render a ProUpServTx service address as `"ip:port"`, collapsing an
/// IPv4-mapped IPv6 address to V4 so a normal masternode renders as
/// `"1.2.3.4:port"`. A native IPv6 address is bracketed,
/// `"[2001:db8::1]:port"`, as a ProRegTx's address renders.
pub fn provider_ip_port(address: std::net::SocketAddr) -> String {
    std::net::SocketAddr::new(address.ip().to_canonical(), address.port()).to_string()
}

/// The script a ProRegTx / ProUpRegTx pays the owner reward to. Before
/// version 3 it is `script_payout`. From version 3 `script_payout` is empty
/// and a payout list replaces it: a list with one entry names the script,
/// while a list that splits the reward names no single one, as for the
/// TRANSFER key of Platform's masternode owner identity. `None` when the
/// payload names no single, non-empty script.
pub(crate) fn owner_payout_script<'a>(
    script_payout: &'a dashcore::ScriptBuf,
    payouts: Option<&'a [MasternodePayoutShare]>,
) -> Option<&'a dashcore::ScriptBuf> {
    if !script_payout.is_empty() {
        return Some(script_payout);
    }
    match payouts {
        Some([share]) if !share.script_payout.is_empty() => Some(&share.script_payout),
        _ => None,
    }
}

/// The platform HTTP port of a ProRegTx / ProUpServTx: the payload's own
/// field before version 3, and from version 3 the port of a `PlatformHttps`
/// entry of its extended address map. Hosts reach the node's DAPI at the
/// service address's IP with this port, so only an entry at that IP counts:
/// one at another IP or a domain gives `None` rather than a wrong endpoint.
/// An IPv4-mapped IPv6 address is the same IP as the IPv4 address it maps.
pub(crate) fn platform_http_port(
    service_address: &MasternodeNetInfo,
    platform_http_port: Option<u16>,
) -> Option<u16> {
    let MasternodeNetInfo::Extended(net_info) = service_address else {
        return platform_http_port;
    };
    let service_ip = service_address
        .primary_service_address()?
        .ip()
        .to_canonical();
    let (_, https) = net_info
        .purposes
        .iter()
        .find(|(purpose, _)| *purpose == NetInfoPurpose::PlatformHttps)?;
    https.iter().find_map(|entry| match entry {
        NetInfoEntry::Service {
            network,
            addr,
            port,
        } => {
            let ip = match network {
                Bip155Network::Ipv4 => IpAddr::from(<[u8; 4]>::try_from(addr.as_slice()).ok()?),
                Bip155Network::Ipv6 => IpAddr::from(<[u8; 16]>::try_from(addr.as_slice()).ok()?),
                _ => return None,
            };
            (ip.to_canonical() == service_ip).then_some(*port)
        }
        _ => None,
    })
}

/// Provider (masternode) special-transaction payload fields lifted for
/// host UIs. All optional / gated — only a ProRegTx or ProUpServTx
/// populates them. The single seam where the DIP-3 payload is decoded;
/// the FFI layers only marshal the flat results.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ProviderPayloadFields {
    /// Service endpoint as `"ip:port"`; `None` for a version-3 payload
    /// that advertises no IPv4/IPv6 address.
    pub service_address: Option<String>,
    /// ProUpServTx registration linkage. `None` for ProRegTx (its own
    /// txid is the proTxHash).
    pub pro_tx_hash: Option<[u8; 32]>,
    /// ProRegTx collateral outpoint (`txid` wire bytes, `vout`).
    pub collateral: Option<([u8; 32], u32)>,
    /// ProRegTx owner / voting key hashes (hash160, 20 bytes).
    pub owner_key_hash: Option<[u8; 20]>,
    pub voting_key_hash: Option<[u8; 20]>,
}

/// Extract provider-registration (ProRegTx) / provider-update-service
/// (ProUpServTx) payload fields from a transaction for display. Returns
/// all-`None` for any other transaction. Pure; the only allocation is
/// the returned service-address `String`. A version-3 payload's service
/// address is the primary routable IPv4/IPv6 entry of its extended address
/// map, `None` when it advertises none.
pub fn provider_payload_fields(tx: &dashcore::Transaction) -> ProviderPayloadFields {
    use dashcore::transaction::TransactionPayload;

    match &tx.special_transaction_payload {
        Some(TransactionPayload::ProviderRegistrationPayloadType(p)) => ProviderPayloadFields {
            service_address: p
                .service_address
                .primary_service_address()
                .map(|address| address.to_string()),
            pro_tx_hash: None,
            collateral: Some((
                provider_hash_to_32(p.collateral_outpoint.txid.as_ref()),
                p.collateral_outpoint.vout,
            )),
            owner_key_hash: Some(provider_hash_to_20(p.owner_key_hash.as_ref())),
            voting_key_hash: Some(provider_hash_to_20(p.voting_key_hash.as_ref())),
        },
        Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) => ProviderPayloadFields {
            service_address: p
                .service_address
                .primary_service_address()
                .map(provider_ip_port),
            pro_tx_hash: Some(provider_hash_to_32(p.pro_tx_hash.as_ref())),
            ..Default::default()
        },
        _ => ProviderPayloadFields::default(),
    }
}

/// Membership of a proTxHash in the current deterministic masternode
/// list (DML), the authoritative status source. Injected into
/// [`aggregate_masternodes`] as a closure so the aggregation stays
/// source-agnostic and unit-testable without a live SPV engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMembership {
    /// In the DML and valid / enabled.
    ValidEntry,
    /// In the DML but flagged invalid (PoSe-banned / `is_valid == false`).
    InvalidEntry,
    /// Not in the DML (collateral spent / revoked / expired).
    Absent,
    /// The DML isn't available yet (SPV not running / masternode sync
    /// incomplete) — status is indeterminate.
    ListUnavailable,
}

/// Displayed masternode status, derived from [`ListMembership`]. The
/// `u8` discriminant is the FFI wire value; `Unknown` (DML unavailable)
/// tells the persist layer to KEEP the previously stored status rather
/// than overwrite it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MasternodeStatus {
    Active,
    Inactive,
    Retired,
    #[default]
    Unknown,
}

impl MasternodeStatus {
    pub fn from_membership(membership: ListMembership) -> Self {
        match membership {
            ListMembership::ValidEntry => Self::Active,
            ListMembership::InvalidEntry => Self::Inactive,
            ListMembership::Absent => Self::Retired,
            ListMembership::ListUnavailable => Self::Unknown,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Active => 0,
            Self::Inactive => 1,
            Self::Retired => 2,
            Self::Unknown => 3,
        }
    }
}

/// One masternode as the wallet layer knows it: the aggregate of a
/// wallet's provider special transactions grouped by proTxHash
/// (`source == Wallet`). Pure/testable output of
/// [`aggregate_masternodes`]; the FFI layers flatten it into their wire
/// entry (`MasternodeEntryFFI`) and own nothing but the encoding.
#[derive(Default, Debug, Clone)]
pub struct MasternodeRecord {
    /// proTxHash (32 wire bytes). For a ProRegTx this is its own txid;
    /// updates / revocations link to it via their `pro_tx_hash`.
    pub pro_tx_hash: [u8; 32],
    /// Whether a ProRegTx for this proTxHash was in the input set.
    pub has_registration: bool,
    /// Core height of the ProRegTx (0 when unseen) — the stable
    /// registration-order sort key.
    pub registration_height: u32,
    /// Latest known service endpoint `"ip:port"` (latest-height update
    /// wins; seeded by the ProRegTx address).
    pub service_address: Option<String>,
    /// Platform HTTP (DAPI gRPC) port from the same ProRegTx / ProUpServTx
    /// that set `service_address` — evonodes only, `None` for a regular
    /// masternode or a pre-v19 payload without platform fields. With the
    /// service IP this addresses the node's DAPI (`https://<ip>:<port>`).
    pub platform_http_port: Option<u16>,
    /// Height that set `service_address` / `platform_http_port` (drives
    /// latest-wins).
    pub(crate) service_height: u32,
    /// evonode / HPMN flag from the ProRegTx `masternode_type`.
    pub is_evonode: bool,
    /// Owner key hash (hash160) from the ProRegTx.
    pub owner_key_hash: Option<[u8; 20]>,
    /// Voting key hash (hash160) — follows the latest ProRegTx / ProUpReg.
    pub voting_key_hash: Option<[u8; 20]>,
    /// Height that set `voting_key_hash` (drives latest-wins).
    pub(crate) voting_height: u32,
    /// Operator BLS public key (48 bytes) — follows the latest ProRegTx /
    /// ProUpReg.
    pub operator_public_key: Option<[u8; 48]>,
    pub(crate) operator_height: u32,
    /// Platform node id (SHA256[..20] Tenderdash, #884, 20 bytes) for evonodes — follows the
    /// latest ProRegTx / ProUpServ.
    pub platform_node_id: Option<[u8; 20]>,
    pub(crate) platform_node_height: u32,
    /// Payout script (raw bytes) — follows the latest ProRegTx / ProUpReg
    /// (owner payout); `None` while the latest one names no single payout
    /// script (see `owner_payout_script`). Encoded to a base58 address by
    /// `masternode_entry_ffi` where the network is available.
    pub payout_script: Option<Vec<u8>>,
    pub(crate) payout_height: u32,
    /// Collateral outpoint (`txid` wire bytes, `vout`) from the ProRegTx.
    pub collateral: Option<([u8; 32], u32)>,
    /// A ProUpRevTx was seen ⇒ the masternode was revoked ("previously
    /// had"). `revocation_reason` keeps the latest reason for reference.
    pub revoked: bool,
    pub revocation_reason: u16,
    /// Count of provider txs seen for this proTxHash.
    pub tx_count: u32,
    /// 1-based index WITHIN this masternode's type, in registration order —
    /// evonodes and regular masternodes each get their own sequence
    /// ("Evonode 1, 2, …" / "Masternode 1, 2, …"). `orderIndex` remains the
    /// cross-type stable sort key.
    pub type_index: u32,
    /// Status against the current DML (authoritative). `Unknown` when the
    /// DML isn't available. Note: this is NOT `revoked`-derived — a
    /// ProUpRevTx merely tends to make the node `Absent` (⇒ `Retired`);
    /// the DML is the source of truth. `revoked` / `revocation_reason`
    /// are retained as separate data.
    pub status: MasternodeStatus,
    /// Where this record came from — see [`MasternodeSource`].
    pub source: MasternodeSource,
    /// Stable cross-type position in the caller's sorted record list
    /// (the "Masternode N" ordering key); assigned by the lister, 0 from
    /// [`aggregate_masternodes`] alone.
    pub order_index: u32,
    /// Derive-and-compare ownership of the operator BLS key: the wallet's
    /// `ProviderOperatorKeys` index whose public key (modern or legacy
    /// serialization) equals `operator_public_key`. `None` when not in the
    /// wallet or unresolved.
    pub operator_key_index: Option<u32>,
    /// Derive-and-compare ownership of the platform node key: the wallet's
    /// `ProviderPlatformKeys` index whose Tenderdash node id equals
    /// `platform_node_id`. `None` when not in the wallet or unresolved.
    pub platform_key_index: Option<u32>,
    /// Host-facing display label. Only tracked records carry one; wallet
    /// aggregation always leaves it `None`.
    pub label: Option<String>,
    /// Whether the platform-node ownership check was actually *possible*:
    /// `true` when the wallet's derived platform-node index had entries to
    /// compare against, `false` when it was empty / unavailable (no platform
    /// pool, or a seedless restore before the persisted key batch rehydrated
    /// it). Lets a persister distinguish a definitive
    /// `platform_key_index == None` (checked, not ours) from "couldn't check
    /// yet", so it never clobbers stale ownership.
    pub platform_ownership_checked: bool,
}

/// Provenance of a [`MasternodeRecord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MasternodeSource {
    /// Aggregated from a wallet's own retained provider transactions —
    /// the masternode is registered with (some of) that wallet's keys.
    #[default]
    Wallet,
    /// Deliberately tracked by the user, independent of every wallet
    /// (see [`super::tracked`]).
    Tracked,
}

impl MasternodeSource {
    /// FFI wire value: 0 wallet, 1 tracked.
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Wallet => 0,
            Self::Tracked => 1,
        }
    }
}

/// Aggregate a wallet's provider special transactions into masternode
/// entities, grouped by proTxHash. Each input is `(core_height, tx)`;
/// height drives latest-wins for the mutable fields (service address,
/// voting key), so callers may feed records in any order. Non-provider
/// txs are ignored.
///
/// Output is sorted by registration height then proTxHash for stable
/// "Masternode N" numbering; entities seen only via an update
/// (registration not in the input set — e.g. the ProRegTx was evicted or
/// isn't ours) sort last.
///
/// Status is resolved against the DML via the injected `list_lookup`
/// closure (`proTxHash -> ListMembership`), keeping this function free of
/// any live SPV dependency so tests can stub the lookup.
///
/// Pure — no I/O; allocation is limited to the aggregate strings. The
/// record source (which txs to feed) is the caller's concern (see the
/// query fn), which is why this is decoupled and unit-testable.
pub fn aggregate_masternodes<'a, F>(
    txs: impl Iterator<Item = (u32, u32, &'a dashcore::Transaction)>,
    list_lookup: F,
) -> Vec<MasternodeRecord>
where
    F: Fn(&[u8; 32]) -> ListMembership,
{
    use dashcore::blockdata::transaction::special_transaction::provider_registration::ProviderMasternodeType;
    use dashcore::transaction::TransactionPayload;

    // Each input item is `(height, in_block_position, tx)`. Core's
    // `RebuildListFromBlock` applies same-block provider updates in
    // `block.vtx` order, so the per-field latest-wins below must resolve
    // ties by `(height, position)`, not by the arbitrary txid order the
    // caller's `BTreeMap<Txid, _>` dedup produces. Process ascending
    // `(height, position)` so the block-latest write for each field lands
    // last and wins under the `>= *_height` guards. Stable so equal keys
    // keep their incoming order.
    //
    // The position is stamped onto `BlockInfo` during block processing
    // (rust-dashcore#891) and round-tripped through persistence; legacy
    // rows confirmed before the field existed come back as 0 and fall
    // back to feed order among themselves.
    let mut ordered: Vec<(u32, u32, &'a dashcore::Transaction)> = txs.collect();
    ordered.sort_by_key(|(height, position, _)| (*height, *position));

    let mut order: Vec<[u8; 32]> = Vec::new();
    let mut by_hash: std::collections::HashMap<[u8; 32], MasternodeRecord> =
        std::collections::HashMap::new();

    for (height, _position, tx) in ordered {
        // proTxHash key: a ProRegTx's own txid, else the update's link.
        let key = match &tx.special_transaction_payload {
            Some(TransactionPayload::ProviderRegistrationPayloadType(_)) => {
                provider_hash_to_32(tx.txid().as_ref())
            }
            Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) => {
                provider_hash_to_32(p.pro_tx_hash.as_ref())
            }
            Some(TransactionPayload::ProviderUpdateRegistrarPayloadType(p)) => {
                provider_hash_to_32(p.pro_tx_hash.as_ref())
            }
            Some(TransactionPayload::ProviderUpdateRevocationPayloadType(p)) => {
                provider_hash_to_32(p.pro_tx_hash.as_ref())
            }
            _ => continue,
        };

        let agg = by_hash.entry(key).or_insert_with(|| {
            order.push(key);
            MasternodeRecord {
                pro_tx_hash: key,
                ..Default::default()
            }
        });
        agg.tx_count = agg.tx_count.saturating_add(1);

        match &tx.special_transaction_payload {
            Some(TransactionPayload::ProviderRegistrationPayloadType(p)) => {
                agg.has_registration = true;
                agg.registration_height = height;
                agg.is_evonode = p.masternode_type == ProviderMasternodeType::HighPerformance;
                agg.owner_key_hash = Some(provider_hash_to_20(p.owner_key_hash.as_ref()));
                agg.collateral = Some((
                    provider_hash_to_32(p.collateral_outpoint.txid.as_ref()),
                    p.collateral_outpoint.vout,
                ));
                // Registration seeds the service address and voting key;
                // treat both as updates observed at this height.
                if agg.service_address.is_none() || height >= agg.service_height {
                    agg.service_address = p
                        .service_address
                        .primary_service_address()
                        .map(|address| address.to_string());
                    agg.platform_http_port =
                        platform_http_port(&p.service_address, p.platform_http_port);
                    agg.service_height = height;
                }
                if agg.voting_key_hash.is_none() || height >= agg.voting_height {
                    agg.voting_key_hash = Some(provider_hash_to_20(p.voting_key_hash.as_ref()));
                    agg.voting_height = height;
                }
                if agg.operator_public_key.is_none() || height >= agg.operator_height {
                    let bls: &[u8; 48] = p.operator_public_key.as_ref();
                    agg.operator_public_key = Some(*bls);
                    agg.operator_height = height;
                }
                if agg.platform_node_id.is_none() || height >= agg.platform_node_height {
                    // Evonode-only; `None` on a regular masternode.
                    // `platform_node_id` is a `PlatformNodeId` newtype
                    // (rust-dashcore #885) whose `consensus_decode` normalizes
                    // the wire's reversed uint160-internal bytes to the
                    // canonical Tenderdash `SHA256(pubkey)[..20]` order
                    // (rust-dashcore #887/#889), so `to_byte_array()` here is
                    // already canonical and matches the derived ownership
                    // index (`accessors.rs`) and dashmate display directly —
                    // do NOT reverse platform-side.
                    if let Some(node_id) = p.platform_node_id {
                        agg.platform_node_id = Some(node_id.to_byte_array());
                        agg.platform_node_height = height;
                    }
                }
                if agg.payout_script.is_none() || height >= agg.payout_height {
                    agg.payout_script = owner_payout_script(&p.script_payout, p.payouts.as_deref())
                        .map(|script| script.as_bytes().to_vec());
                    agg.payout_height = height;
                }
            }
            Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) => {
                if agg.service_address.is_none() || height >= agg.service_height {
                    agg.service_address = p
                        .service_address
                        .primary_service_address()
                        .map(provider_ip_port);
                    agg.platform_http_port =
                        platform_http_port(&p.service_address, p.platform_http_port);
                    agg.service_height = height;
                }
                // ProUpServ's `platform_node_id` is now `Option<PlatformNodeId>`
                // (rust-dashcore #885, was `Option<[u8; 20]>`); decoded bytes
                // are canonical forward order (see the ProRegTx arm above).
                if let Some(node_id) = p.platform_node_id {
                    if agg.platform_node_id.is_none() || height >= agg.platform_node_height {
                        agg.platform_node_id = Some(node_id.to_byte_array());
                        agg.platform_node_height = height;
                    }
                }
            }
            Some(TransactionPayload::ProviderUpdateRegistrarPayloadType(p)) => {
                if agg.voting_key_hash.is_none() || height >= agg.voting_height {
                    agg.voting_key_hash = Some(provider_hash_to_20(p.voting_key_hash.as_ref()));
                    agg.voting_height = height;
                }
                if agg.operator_public_key.is_none() || height >= agg.operator_height {
                    let bls: &[u8; 48] = p.operator_public_key.as_ref();
                    agg.operator_public_key = Some(*bls);
                    agg.operator_height = height;
                }
                if agg.payout_script.is_none() || height >= agg.payout_height {
                    agg.payout_script = owner_payout_script(&p.script_payout, p.payouts.as_deref())
                        .map(|script| script.as_bytes().to_vec());
                    agg.payout_height = height;
                }
            }
            Some(TransactionPayload::ProviderUpdateRevocationPayloadType(p)) => {
                agg.revoked = true;
                agg.revocation_reason = p.reason;
            }
            _ => {}
        }
    }

    let mut result: Vec<MasternodeRecord> = order
        .into_iter()
        .filter_map(|k| by_hash.remove(&k))
        .collect();
    // Stable registration-order numbering: registered masternodes by
    // ascending registration height then proTxHash; update-only entities
    // (no ProRegTx seen) sort last via a MAX height sentinel.
    result.sort_by(|a, b| {
        let ha = if a.has_registration {
            a.registration_height
        } else {
            u32::MAX
        };
        let hb = if b.has_registration {
            b.registration_height
        } else {
            u32::MAX
        };
        ha.cmp(&hb).then_with(|| a.pro_tx_hash.cmp(&b.pro_tx_hash))
    });

    // Resolve authoritative status against the DML and assign per-type
    // numbering (separate Evonode / Masternode sequences), both in the
    // stable registration order established above.
    let mut evonode_n: u32 = 0;
    let mut masternode_n: u32 = 0;
    for agg in result.iter_mut() {
        agg.status = MasternodeStatus::from_membership(list_lookup(&agg.pro_tx_hash));
        if agg.is_evonode {
            evonode_n += 1;
            agg.type_index = evonode_n;
        } else {
            masternode_n += 1;
            agg.type_index = masternode_n;
        }
    }
    result
}

/// Provider transactions mined by Dash Core v24 (see the README next to the
/// files), shared by the masternode tests.
#[cfg(test)]
pub(crate) mod core_v24_vectors {
    use std::str::FromStr;

    pub(crate) const PRO_REG_TX_ONE_PAYOUT: (&str, &str) = (
        include_str!("../../tests/vectors/core_v24_devnet/pro_reg_tx_v3_one_payout.hex"),
        "20814c480e94dce7f3fe3f3370ec6bacd1fdc4f446f17aa7de7886be0b5d07c2",
    );
    pub(crate) const PRO_REG_TX_TWO_PAYOUTS: (&str, &str) = (
        include_str!("../../tests/vectors/core_v24_devnet/pro_reg_tx_v3_two_payouts.hex"),
        "f7e6fad0084ae34e410a8b710e5d74c0af1b95a29c1f3ae5507bed8e08b38adb",
    );
    pub(crate) const PRO_REG_TX_P2SH_PAYOUT: (&str, &str) = (
        include_str!("../../tests/vectors/core_v24_devnet/pro_reg_tx_v3_p2sh_payout.hex"),
        "1bff9d395c2647e370202e87c80fa3942930567d217238392363cd2542fb496f",
    );
    pub(crate) const EVO_PRO_UP_SERV_TX: (&str, &str) = (
        include_str!("../../tests/vectors/core_v24_devnet/evo_pro_up_serv_tx_v3.hex"),
        "7c58d9776ef31869b8cae3f94366e9fedb914e41a5b4c26d048c12f30f5e51ac",
    );
    /// Moves the owner payout of the one-payout registration to another
    /// address.
    pub(crate) const PRO_UP_REG_TX: (&str, &str) = (
        include_str!("../../tests/vectors/core_v24_devnet/pro_up_reg_tx_v3.hex"),
        "16b69f49a4aba21954e199efe7209478763133f7c473fab6668d5d2b79e55775",
    );

    /// Decode a vector, checking it hashes to the txid Core reported.
    pub(crate) fn decode((hex, txid): (&str, &str)) -> dashcore::Transaction {
        let bytes = hex::decode(hex.trim()).expect("vector is hex");
        let tx: dashcore::Transaction =
            dashcore::consensus::encode::deserialize(&bytes).expect("Core v24 transaction decodes");
        assert_eq!(tx.txid(), dashcore::Txid::from_str(txid).expect("txid"));
        tx
    }

    /// The one-payout registration as an evonode whose extended addresses
    /// carry the platform ports, as the version-3 ProUpServTx's do.
    pub(crate) fn evonode_registration() -> dashcore::Transaction {
        use dashcore::blockdata::transaction::special_transaction::provider_registration::ProviderMasternodeType;
        use dashcore::transaction::TransactionPayload;

        let Some(TransactionPayload::ProviderUpdateServicePayloadType(update)) =
            decode(EVO_PRO_UP_SERV_TX).special_transaction_payload
        else {
            panic!("ProUpServTx");
        };
        let mut registration = decode(PRO_REG_TX_ONE_PAYOUT);
        if let Some(TransactionPayload::ProviderRegistrationPayloadType(p)) =
            &mut registration.special_transaction_payload
        {
            p.masternode_type = ProviderMasternodeType::HighPerformance;
            p.service_address = update.service_address;
        }
        registration
    }

    /// The Devnet address `script` pays.
    pub(crate) fn devnet_address(script: &[u8]) -> String {
        dashcore::Address::from_script(
            &dashcore::ScriptBuf::from_bytes(script.to_vec()),
            dashcore::Network::Devnet,
        )
        .expect("standard script")
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ProRegTx provider payload is lifted from the DIP-3 special-tx
    /// body for the UI. Fixture is the testnet
    /// collateral-provider-registration transaction from rust-dashcore's
    /// own `provider_registration` tests
    /// (`test_collateral_provider_registration_transaction`), whose
    /// service address is `1.2.5.6:19999` and whose owner/voting key
    /// hashes are asserted below. ProRegTx carries no explicit
    /// `pro_tx_hash` (its own txid is the proTxHash), so that field
    /// stays `None`.
    #[test]
    fn provider_registration_payload_fields_extracted() {
        let raw = "0300010001ca9a43051750da7c5f858008f2ff7732d15691e48eb7f845c791e5dca78bab58010000006b483045022100fe8fec0b3880bcac29614348887769b0b589908e3f5ec55a6cf478a6652e736502202f30430806a6690524e4dd599ba498e5ff100dea6a872ebb89c2fd651caa71ed012103d85b25d6886f0b3b8ce1eef63b720b518fad0b8e103eba4e85b6980bfdda2dfdffffffff018e37807e090000001976a9144ee1d4e5d61ac40a13b357ac6e368997079678c888ac00000000fd1201010000000000ca9a43051750da7c5f858008f2ff7732d15691e48eb7f845c791e5dca78bab580000000000000000000000000000ffff010205064e1f3dd03f9ec192b5f275a433bfc90f468ee1a3eb4c157b10706659e25eb362b5d902d809f9160b1688e201ee6e94b40f9b5062d7074683ef05a2d5efb7793c47059c878dfad38a30fafe61575db40f05ab0a08d55119b0aad300001976a9144fbc8fb6e11e253d77e5a9c987418e89cf4a63d288ac3477990b757387cb0406168c2720acf55f83603736a314a37d01b135b873a27b411fb37e49c1ff2b8057713939a5513e6e711a71cff2e517e6224df724ed750aef1b7f9ad9ec612b4a7250232e1e400da718a9501e1d9a5565526e4b1ff68c028763";
        let bytes = hex::decode(raw).expect("valid fixture hex");
        let tx: dashcore::Transaction =
            dashcore::consensus::encode::deserialize(&bytes).expect("decode ProRegTx");

        let fields = provider_payload_fields(&tx);

        assert_eq!(
            fields.service_address.as_deref(),
            Some("1.2.5.6:19999"),
            "service address must be lifted from the ProRegTx payload"
        );
        assert!(
            fields.collateral.is_some(),
            "ProRegTx carries a collateral outpoint"
        );
        assert_eq!(
            hex::encode(fields.owner_key_hash.expect("owner key hash")),
            "3dd03f9ec192b5f275a433bfc90f468ee1a3eb4c"
        );
        assert_eq!(
            hex::encode(fields.voting_key_hash.expect("voting key hash")),
            "d38a30fafe61575db40f05ab0a08d55119b0aad3"
        );
        assert!(
            fields.pro_tx_hash.is_none(),
            "ProRegTx has no explicit pro_tx_hash"
        );
    }

    /// ProUpServTx (provider-update-service) also carries a service
    /// address — on the wire an IPv4-mapped IPv6 address and a port —
    /// plus an explicit `pro_tx_hash` linking it to the registration. Fixture is rust-dashcore's own
    /// `test_provider_update_service_transaction` vector, whose endpoint
    /// is `52.36.64.148:19999`. The `pro_tx_hash` is asserted in raw
    /// wire order (what `to_32(txid.as_ref())` stores) — the reverse of
    /// the block-explorer display form.
    #[test]
    fn provider_update_service_payload_fields_extracted() {
        let raw = "03000200018f3fe6683e36326669b6e34876fb2a2264e8327e822f6fec304b66f47d61b3e1010000006b48304502210082af6727408f0f2ec16c7da1c42ccf0a026abea6a3a422776272b03c8f4e262a022033b406e556f6de980b2d728e6812b3ae18ee1c863ae573ece1cbdf777ca3e56101210351036c1192eaf763cd8345b44137482ad24b12003f23e9022ce46752edf47e6effffffff0180220e43000000001976a914123cbc06289e768ca7d743c8174b1e6eeb610f1488ac00000000b501003a72099db84b1c1158568eec863bea1b64f90eccee3304209cebe1df5e7539fd00000000000000000000ffff342440944e1f00e6725f799ea20480f06fb105ebe27e7c4845ab84155e4c2adf2d6e5b73a998b1174f9621bbeda5009c5a6487bdf75edcf602b67fe0da15c275cc91777cb25f5fd4bb94e84fd42cb2bb547c83792e57c80d196acd47020e4054895a0640b7861b3729c41dd681d4996090d5750f65c4b649a5cd5b2bdf55c880459821e53d91c9";
        let bytes = hex::decode(raw).expect("valid fixture hex");
        let tx: dashcore::Transaction =
            dashcore::consensus::encode::deserialize(&bytes).expect("decode ProUpServTx");

        let fields = provider_payload_fields(&tx);

        assert_eq!(
            fields.service_address.as_deref(),
            Some("52.36.64.148:19999"),
            "ProUpServTx endpoint must be rendered as IPv4 ip:port"
        );
        assert_eq!(
            fields.pro_tx_hash.map(hex::encode).as_deref(),
            Some("3a72099db84b1c1158568eec863bea1b64f90eccee3304209cebe1df5e7539fd"),
            "ProUpServTx carries an explicit pro_tx_hash (wire order)"
        );
        assert!(
            fields.collateral.is_none(),
            "ProUpServTx has no collateral outpoint"
        );
        assert!(fields.owner_key_hash.is_none());
        assert!(fields.voting_key_hash.is_none());
    }

    /// A plain (non-provider) transaction yields no provider fields, so
    /// the FFI record emits null/zeroed/`false` for all of them.
    #[test]
    fn non_provider_tx_has_no_provider_fields() {
        let tx = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        };
        let fields = provider_payload_fields(&tx);
        assert!(fields.service_address.is_none());
        assert!(fields.pro_tx_hash.is_none());
        assert!(fields.collateral.is_none());
        assert!(fields.owner_key_hash.is_none());
        assert!(fields.voting_key_hash.is_none());
    }

    // rust-dashcore's own test vectors (see the payload extraction tests
    // above). Both are unrelated masternodes, so they aggregate into
    // distinct proTxHash buckets.
    const PROREG_HEX: &str = "0300010001ca9a43051750da7c5f858008f2ff7732d15691e48eb7f845c791e5dca78bab58010000006b483045022100fe8fec0b3880bcac29614348887769b0b589908e3f5ec55a6cf478a6652e736502202f30430806a6690524e4dd599ba498e5ff100dea6a872ebb89c2fd651caa71ed012103d85b25d6886f0b3b8ce1eef63b720b518fad0b8e103eba4e85b6980bfdda2dfdffffffff018e37807e090000001976a9144ee1d4e5d61ac40a13b357ac6e368997079678c888ac00000000fd1201010000000000ca9a43051750da7c5f858008f2ff7732d15691e48eb7f845c791e5dca78bab580000000000000000000000000000ffff010205064e1f3dd03f9ec192b5f275a433bfc90f468ee1a3eb4c157b10706659e25eb362b5d902d809f9160b1688e201ee6e94b40f9b5062d7074683ef05a2d5efb7793c47059c878dfad38a30fafe61575db40f05ab0a08d55119b0aad300001976a9144fbc8fb6e11e253d77e5a9c987418e89cf4a63d288ac3477990b757387cb0406168c2720acf55f83603736a314a37d01b135b873a27b411fb37e49c1ff2b8057713939a5513e6e711a71cff2e517e6224df724ed750aef1b7f9ad9ec612b4a7250232e1e400da718a9501e1d9a5565526e4b1ff68c028763";
    const PROUPSERV_HEX: &str = "03000200018f3fe6683e36326669b6e34876fb2a2264e8327e822f6fec304b66f47d61b3e1010000006b48304502210082af6727408f0f2ec16c7da1c42ccf0a026abea6a3a422776272b03c8f4e262a022033b406e556f6de980b2d728e6812b3ae18ee1c863ae573ece1cbdf777ca3e56101210351036c1192eaf763cd8345b44137482ad24b12003f23e9022ce46752edf47e6effffffff0180220e43000000001976a914123cbc06289e768ca7d743c8174b1e6eeb610f1488ac00000000b501003a72099db84b1c1158568eec863bea1b64f90eccee3304209cebe1df5e7539fd00000000000000000000ffff342440944e1f00e6725f799ea20480f06fb105ebe27e7c4845ab84155e4c2adf2d6e5b73a998b1174f9621bbeda5009c5a6487bdf75edcf602b67fe0da15c275cc91777cb25f5fd4bb94e84fd42cb2bb547c83792e57c80d196acd47020e4054895a0640b7861b3729c41dd681d4996090d5750f65c4b649a5cd5b2bdf55c880459821e53d91c9";

    fn decode_tx(hex: &str) -> dashcore::Transaction {
        let bytes = hex::decode(hex).expect("valid fixture hex");
        dashcore::consensus::encode::deserialize(&bytes).expect("decode tx")
    }

    /// Stub DML lookup: the list is never available (⇒ every entity is
    /// `Unknown`). Mirrors "SPV not running / masternode sync incomplete".
    fn unavailable_dml(_pro_tx_hash: &[u8; 32]) -> ListMembership {
        ListMembership::ListUnavailable
    }

    /// A lone ProRegTx aggregates into one active masternode carrying its
    /// service address, key hashes, and collateral, keyed by its own txid.
    #[test]
    fn aggregate_single_registration() {
        let reg = decode_tx(PROREG_HEX);
        let expected_pro_tx = provider_hash_to_32(reg.txid().as_ref());

        let mns = aggregate_masternodes([(100u32, 0u32, &reg)].into_iter(), unavailable_dml);
        assert_eq!(mns.len(), 1);
        let mn = &mns[0];
        assert_eq!(mn.pro_tx_hash, expected_pro_tx);
        assert_eq!(mn.status, MasternodeStatus::Unknown, "no DML ⇒ Unknown");
        assert!(mn.has_registration);
        assert!(!mn.revoked);
        assert!(!mn.is_evonode, "legacy ProRegTx fixture is a regular MN");
        assert_eq!(mn.service_address.as_deref(), Some("1.2.5.6:19999"));
        assert!(mn.owner_key_hash.is_some());
        assert!(mn.voting_key_hash.is_some());
        assert!(mn.collateral.is_some());
        // Key-ownership extraction: operator BLS key + payout script
        // are lifted; the legacy (v1) fixture is a regular MN so it has no
        // platform node id.
        assert!(
            mn.operator_public_key.is_some(),
            "ProRegTx carries a 48-byte operator BLS key"
        );
        assert!(
            mn.payout_script.as_ref().is_some_and(|s| !s.is_empty()),
            "ProRegTx carries a payout script"
        );
        assert!(
            mn.platform_node_id.is_none(),
            "legacy regular-MN fixture has no platform node id"
        );
        assert!(
            mn.platform_http_port.is_none(),
            "legacy regular-MN fixture has no platform HTTP port"
        );
        assert_eq!(mn.tx_count, 1);
    }

    /// A ProUpServTx whose registration isn't in the input set still
    /// yields a masternode (keyed by its `pro_tx_hash`) with the updated
    /// service address but no registration-only fields.
    #[test]
    fn aggregate_update_only_masternode() {
        let ups = decode_tx(PROUPSERV_HEX);
        let mns = aggregate_masternodes([(50u32, 0u32, &ups)].into_iter(), unavailable_dml);
        assert_eq!(mns.len(), 1);
        let mn = &mns[0];
        assert!(!mn.has_registration);
        assert_eq!(mn.service_address.as_deref(), Some("52.36.64.148:19999"));
        assert!(mn.owner_key_hash.is_none());
        assert!(mn.collateral.is_none());
        assert_eq!(mn.tx_count, 1);
    }

    /// Two unrelated provider txs bucket into two masternodes.
    #[test]
    fn aggregate_groups_by_pro_tx_hash() {
        let reg = decode_tx(PROREG_HEX);
        let ups = decode_tx(PROUPSERV_HEX);
        let mns = aggregate_masternodes(
            [(100u32, 0u32, &reg), (200u32, 0u32, &ups)].into_iter(),
            unavailable_dml,
        );
        assert_eq!(mns.len(), 2, "distinct proTxHashes ⇒ two masternodes");
    }

    /// A ProUpRevTx linked to a registration flips the masternode to
    /// revoked ("previously had") while its service address and count
    /// reflect the full provider-tx set. Built programmatically because
    /// rust-dashcore ships no ProUpRevTx raw-hex vector.
    #[test]
    fn aggregate_revocation_marks_revoked() {
        use dashcore::blockdata::transaction::special_transaction::provider_update_revocation::ProviderUpdateRevocationPayload;
        use dashcore::transaction::TransactionPayload;

        let reg = decode_tx(PROREG_HEX);
        let pro_tx_hash = reg.txid();

        let rev_payload = ProviderUpdateRevocationPayload {
            version: 1,
            pro_tx_hash,
            reason: 2,
            inputs_hash: [3u8; 32].into(),
            payload_sig: [0u8; 96].into(),
        };
        let rev = dashcore::Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(
                TransactionPayload::ProviderUpdateRevocationPayloadType(rev_payload),
            ),
        };

        // A ProUpRevTx'd node is Absent from the DML here ⇒ Retired.
        let revoked_pro_tx = provider_hash_to_32(pro_tx_hash.as_ref());
        let lookup = |pt: &[u8; 32]| {
            if *pt == revoked_pro_tx {
                ListMembership::Absent
            } else {
                ListMembership::ListUnavailable
            }
        };

        // Revocation feed order shouldn't matter (height drives merges).
        let mns = aggregate_masternodes(
            [(300u32, 0u32, &rev), (100u32, 0u32, &reg)].into_iter(),
            lookup,
        );
        assert_eq!(mns.len(), 1);
        let mn = &mns[0];
        assert_eq!(mn.pro_tx_hash, revoked_pro_tx);
        assert!(mn.has_registration);
        assert!(mn.revoked, "a ProUpRevTx marks the revoked-data flag");
        assert_eq!(mn.revocation_reason, 2);
        assert_eq!(
            mn.status,
            MasternodeStatus::Retired,
            "absent from the DML ⇒ Retired (status is DML-derived, not revoked-derived)"
        );
        assert_eq!(mn.service_address.as_deref(), Some("1.2.5.6:19999"));
        assert_eq!(mn.tx_count, 2);
    }

    /// Status is derived from the injected DML lookup, not from tx history:
    /// a valid entry ⇒ Active, a present-but-invalid entry ⇒ Inactive, an
    /// absent entry ⇒ Retired — all for the same (unrevoked) ProRegTx.
    #[test]
    fn aggregate_status_follows_dml_membership() {
        let reg = decode_tx(PROREG_HEX);
        let pro_tx = provider_hash_to_32(reg.txid().as_ref());

        for (membership, expected) in [
            (ListMembership::ValidEntry, MasternodeStatus::Active),
            (ListMembership::InvalidEntry, MasternodeStatus::Inactive),
            (ListMembership::Absent, MasternodeStatus::Retired),
            (ListMembership::ListUnavailable, MasternodeStatus::Unknown),
        ] {
            let lookup = |pt: &[u8; 32]| {
                assert_eq!(*pt, pro_tx);
                membership
            };
            let mns = aggregate_masternodes([(100u32, 0u32, &reg)].into_iter(), lookup);
            assert_eq!(mns.len(), 1);
            assert_eq!(mns[0].status, expected);
            assert!(!mns[0].revoked, "no ProUpRevTx ⇒ revoked flag stays false");
        }
    }

    /// Evonodes and regular masternodes get INDEPENDENT 1-based per-type
    /// sequences: an evonode + a regular in one aggregation each get
    /// `type_index == 1`. Built by cloning the regular ProRegTx fixture and
    /// flipping its `masternode_type` (plus `lock_time`, so the txid — and
    /// thus the proTxHash group key — differs).
    #[test]
    fn aggregate_per_type_numbering() {
        use dashcore::blockdata::transaction::special_transaction::provider_registration::ProviderMasternodeType;
        use dashcore::transaction::TransactionPayload;

        let regular = decode_tx(PROREG_HEX);

        let mut evonode = decode_tx(PROREG_HEX);
        evonode.lock_time = 4242; // change the txid ⇒ distinct proTxHash
        if let Some(TransactionPayload::ProviderRegistrationPayloadType(p)) =
            &mut evonode.special_transaction_payload
        {
            p.masternode_type = ProviderMasternodeType::HighPerformance;
        }

        let mns = aggregate_masternodes(
            [(100u32, 0u32, &regular), (200u32, 0u32, &evonode)].into_iter(),
            unavailable_dml,
        );
        assert_eq!(mns.len(), 2, "distinct proTxHashes ⇒ two masternodes");

        let evo = mns.iter().find(|m| m.is_evonode).expect("evonode present");
        let reg = mns.iter().find(|m| !m.is_evonode).expect("regular present");
        assert_eq!(evo.type_index, 1, "first (only) evonode ⇒ Evonode 1");
        assert_eq!(reg.type_index, 1, "first (only) regular ⇒ Masternode 1");
    }

    /// Two provider updates for one masternode in the SAME block must resolve
    /// the per-field latest-wins by in-block `position`, matching Core's
    /// `block.vtx` order — NOT by the arbitrary txid order the caller's
    /// `BTreeMap<Txid>` dedup would otherwise impose. Feed the same pair in
    /// both orders; the higher-positioned (block-latest) update wins each time,
    /// proving position — not feed/txid order — decides the outcome.
    #[test]
    fn same_block_updates_resolve_by_position_not_txid() {
        use dashcore::blockdata::transaction::special_transaction::provider_update_service::ProviderUpdateServicePayload;
        use dashcore::transaction::TransactionPayload;

        // Shared registration linkage ⇒ both updates land in one bucket.
        let pro_tx_hash = decode_tx(PROREG_HEX).txid();
        let group_key = provider_hash_to_32(pro_tx_hash.as_ref());

        // Build a ProUpServTx directly (no raw-hex vector needed); `port`
        // distinguishes the resulting service address, `inputs` perturbs the
        // txid so the two txs are genuinely distinct.
        let make_upserv = |port: u16, inputs: u8| -> dashcore::Transaction {
            let payload = ProviderUpdateServicePayload {
                version: 1,
                mn_type: None,
                pro_tx_hash,
                service_address: MasternodeNetInfo::Legacy(std::net::SocketAddr::new(
                    std::net::Ipv6Addr::from(42u128.to_le_bytes()).into(),
                    port,
                )),
                script_payout: dashcore::ScriptBuf::new(),
                inputs_hash: [inputs; 32].into(),
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
                payload_sig: [0u8; 96].into(),
            };
            dashcore::Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: Some(
                    TransactionPayload::ProviderUpdateServicePayloadType(payload),
                ),
            }
        };

        let low = make_upserv(19000, 3); // in-block position 0
        let high = make_upserv(19999, 4); // in-block position 1 (block-latest)

        for feed in [
            [(500u32, 0u32, &low), (500u32, 1u32, &high)],
            // Reversed feed order (block-latest fed first): position, not feed
            // order, must still pick the winner.
            [(500u32, 1u32, &high), (500u32, 0u32, &low)],
        ] {
            let mns = aggregate_masternodes(feed.into_iter(), unavailable_dml);
            assert_eq!(mns.len(), 1, "same proTxHash ⇒ one bucket");
            assert_eq!(mns[0].pro_tx_hash, group_key);
            assert!(
                mns[0]
                    .service_address
                    .as_deref()
                    .unwrap_or_default()
                    .ends_with(":19999"),
                "higher in-block position (block-latest) must win; got {:?}",
                mns[0].service_address
            );
            assert_eq!(mns[0].tx_count, 2, "both updates counted");
        }
    }

    /// The platform HTTP port travels with the service endpoint: the ProRegTx
    /// seeds it and a later ProUpServTx replaces it (latest-wins), so the
    /// DAPI address the wallet builds follows the node's current config.
    #[test]
    fn platform_http_port_follows_the_service_update() {
        use dashcore::blockdata::transaction::special_transaction::provider_update_service::ProviderUpdateServicePayload;
        use dashcore::transaction::special_transaction::provider_registration::ProviderMasternodeType;
        use dashcore::transaction::TransactionPayload;

        let mut reg = decode_tx(PROREG_HEX);
        if let Some(TransactionPayload::ProviderRegistrationPayloadType(p)) =
            &mut reg.special_transaction_payload
        {
            p.masternode_type = ProviderMasternodeType::HighPerformance;
            p.platform_http_port = Some(443);
        }
        let pro_tx_hash = reg.txid();

        let upserv = dashcore::Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(
                TransactionPayload::ProviderUpdateServicePayloadType(
                    ProviderUpdateServicePayload {
                        version: 2,
                        mn_type: Some(1), // HighPerformance (evonode)
                        pro_tx_hash,
                        service_address: MasternodeNetInfo::Legacy(std::net::SocketAddr::new(
                            std::net::Ipv6Addr::from(42u128.to_le_bytes()).into(),
                            19999,
                        )),
                        script_payout: dashcore::ScriptBuf::new(),
                        inputs_hash: [7u8; 32].into(),
                        platform_node_id: None,
                        platform_p2p_port: Some(36656),
                        platform_http_port: Some(1443),
                        payload_sig: [0u8; 96].into(),
                    },
                ),
            ),
        };

        // Registration alone ⇒ the ProRegTx port.
        let mns = aggregate_masternodes([(100u32, 0u32, &reg)].into_iter(), unavailable_dml);
        assert_eq!(mns.len(), 1);
        assert_eq!(mns[0].platform_http_port, Some(443));

        // A later ProUpServTx replaces it along with the service address.
        let mns = aggregate_masternodes(
            [(100u32, 0u32, &reg), (200u32, 0u32, &upserv)].into_iter(),
            unavailable_dml,
        );
        assert_eq!(mns.len(), 1, "same proTxHash ⇒ one bucket");
        assert_eq!(mns[0].platform_http_port, Some(1443));
        assert!(
            mns[0]
                .service_address
                .as_deref()
                .unwrap_or_default()
                .ends_with(":19999"),
            "service address and platform port move together"
        );
    }

    /// Version-3 ProRegTxs as Core v24 mines them decode, and their provider
    /// fields are lifted. These masternodes announce no address, so there is
    /// no service address to show.
    #[test]
    fn should_lift_the_provider_fields_of_core_v24_registrations() {
        use super::core_v24_vectors::*;

        for vector in [
            PRO_REG_TX_ONE_PAYOUT,
            PRO_REG_TX_TWO_PAYOUTS,
            PRO_REG_TX_P2SH_PAYOUT,
        ] {
            let fields = provider_payload_fields(&decode(vector));
            assert_eq!(fields.service_address, None);
            assert!(fields.collateral.is_some());
            assert!(fields.owner_key_hash.is_some());
            assert!(fields.voting_key_hash.is_some());
            assert_eq!(fields.pro_tx_hash, None);
        }
    }

    /// A version-3 ProUpServTx carries its address in the extended address
    /// map; the wallet shows its primary IPv4 entry.
    #[test]
    fn should_lift_the_service_address_of_a_core_v24_service_update() {
        use super::core_v24_vectors::*;

        let fields = provider_payload_fields(&decode(EVO_PRO_UP_SERV_TX));
        assert_eq!(
            fields.service_address.as_deref(),
            Some("68.67.122.237:20001")
        );
        assert!(fields.pro_tx_hash.is_some());
    }

    /// From version 3 the owner payout is a payout list and `script_payout`
    /// is empty. A list with one entry is the payout, whatever its script
    /// type; a list that splits the reward names no single payout, so none
    /// is recorded, as Platform gives such a masternode's owner identity no
    /// TRANSFER key. An empty script is never recorded as the payout.
    #[test]
    fn should_take_a_v3_registration_payout_from_its_payout_list() {
        use super::core_v24_vectors::*;

        let payout_of = |vector| {
            let tx = decode(vector);
            aggregate_masternodes([(15_146u32, 1u32, &tx)].into_iter(), unavailable_dml)[0]
                .payout_script
                .clone()
        };

        let one = payout_of(PRO_REG_TX_ONE_PAYOUT).expect("one payout");
        assert_eq!(devnet_address(&one), "yQohnoFhM5avKnzVzAX79xT8uCKviaPxj4");
        let p2sh = payout_of(PRO_REG_TX_P2SH_PAYOUT).expect("one payout");
        assert_eq!(devnet_address(&p2sh), "8yyRcXHGCXypDWTCNZMHUYZJJEC87GqLfK");
        assert_eq!(payout_of(PRO_REG_TX_TWO_PAYOUTS), None);
    }

    /// A version-3 ProUpRegTx moves the payout through its payout list. It
    /// replaces the payout of a version-3 registration and of a pre-v3 one,
    /// whose payout is a script; it must not clear either with its empty
    /// `script_payout`.
    #[test]
    fn should_follow_a_v3_payout_update() {
        use super::core_v24_vectors::*;
        use dashcore::transaction::TransactionPayload;

        let update = decode(PRO_UP_REG_TX);
        let updated_payout = "yT7ShTqEqPBskpTAiHzWaAggFmD4j1MCT3";

        // The registration it updates, mined before it.
        let registration = decode(PRO_REG_TX_ONE_PAYOUT);
        let mns = aggregate_masternodes(
            [(15_146u32, 1u32, &registration), (15_190u32, 1u32, &update)].into_iter(),
            unavailable_dml,
        );
        assert_eq!(mns.len(), 1, "the update links to the registration");
        let payout = mns[0].payout_script.as_deref().expect("payout");
        assert_eq!(devnet_address(payout), updated_payout);

        // A pre-v3 registration, which pays to `script_payout`.
        let legacy = decode_tx(PROREG_HEX);
        let mut legacy_update = update.clone();
        if let Some(TransactionPayload::ProviderUpdateRegistrarPayloadType(p)) =
            &mut legacy_update.special_transaction_payload
        {
            p.pro_tx_hash = legacy.txid();
        }
        let mns = aggregate_masternodes(
            [(100u32, 0u32, &legacy), (200u32, 0u32, &legacy_update)].into_iter(),
            unavailable_dml,
        );
        assert_eq!(mns.len(), 1);
        let payout = mns[0].payout_script.as_deref().expect("payout");
        assert_eq!(devnet_address(payout), updated_payout);

        // An update that splits the reward with a second payee leaves no
        // single payout.
        let Some(TransactionPayload::ProviderRegistrationPayloadType(p2sh)) =
            decode(PRO_REG_TX_P2SH_PAYOUT).special_transaction_payload
        else {
            panic!("ProRegTx");
        };
        let mut second = p2sh.payouts.expect("version 3 has payouts")[0].clone();
        second.reward = 4_000;
        let mut split = update.clone();
        if let Some(TransactionPayload::ProviderUpdateRegistrarPayloadType(p)) =
            &mut split.special_transaction_payload
        {
            let mut shares = p.payouts.clone().expect("version 3 has payouts");
            shares[0].reward = 6_000;
            shares.push(second);
            p.payouts = Some(shares);
        }
        let mns = aggregate_masternodes(
            [(15_146u32, 1u32, &registration), (15_190u32, 1u32, &split)].into_iter(),
            unavailable_dml,
        );
        assert_eq!(mns[0].payout_script, None);
    }

    /// From version 3 the platform HTTP port is the `PlatformHttps` entry of
    /// the extended address map, not a payload field, and travels with the
    /// service address.
    #[test]
    fn should_take_a_v3_service_update_platform_port_from_its_addresses() {
        use super::core_v24_vectors::*;

        let update = decode(EVO_PRO_UP_SERV_TX);
        let mns = aggregate_masternodes([(15_182u32, 1u32, &update)].into_iter(), unavailable_dml);
        assert_eq!(
            mns[0].service_address.as_deref(),
            Some("68.67.122.237:20001")
        );
        assert_eq!(mns[0].platform_http_port, Some(1443));
    }

    /// A version-3 evonode registration carries its platform HTTP port in
    /// its extended address map too.
    #[test]
    fn should_take_a_v3_registration_platform_port_from_its_addresses() {
        use super::core_v24_vectors::*;

        let registration = evonode_registration();
        let mns = aggregate_masternodes(
            [(15_146u32, 1u32, &registration)].into_iter(),
            unavailable_dml,
        );
        assert!(mns[0].is_evonode);
        assert_eq!(
            mns[0].service_address.as_deref(),
            Some("68.67.122.237:20001")
        );
        assert_eq!(mns[0].platform_http_port, Some(1443));
    }

    /// Hosts reach the node's DAPI at the service address's IP, so a
    /// `PlatformHttps` entry at another IP, or a domain, gives no port
    /// rather than a wrong endpoint.
    #[test]
    fn should_take_no_platform_port_from_an_https_entry_elsewhere() {
        use super::core_v24_vectors::*;
        use dashcore::transaction::TransactionPayload;

        let Some(TransactionPayload::ProviderUpdateServicePayloadType(update)) =
            decode(EVO_PRO_UP_SERV_TX).special_transaction_payload
        else {
            panic!("ProUpServTx");
        };
        let with_https = |entry: NetInfoEntry| {
            let mut service_address = update.service_address.clone();
            let MasternodeNetInfo::Extended(net_info) = &mut service_address else {
                panic!("version 3 carries extended addresses");
            };
            for (purpose, entries) in &mut net_info.purposes {
                if *purpose == NetInfoPurpose::PlatformHttps {
                    *entries = vec![entry.clone()];
                }
            }
            service_address
        };

        let same_ip = with_https(NetInfoEntry::Service {
            network: Bip155Network::Ipv4,
            addr: vec![68, 67, 122, 237],
            port: 443,
        });
        assert_eq!(platform_http_port(&same_ip, None), Some(443));
        let other_ip = with_https(NetInfoEntry::Service {
            network: Bip155Network::Ipv4,
            addr: vec![10, 0, 0, 1],
            port: 443,
        });
        assert_eq!(platform_http_port(&other_ip, None), None);
        let domain = with_https(NetInfoEntry::Domain {
            host: "dapi.example.org".to_string(),
            port: 443,
        });
        assert_eq!(platform_http_port(&domain, None), None);
    }

    /// The version-3 ProUpServTx's extended addresses with `core_p2p` as its
    /// `CoreP2P` entries and `https` as its `PlatformHttps` entries.
    fn v3_service_address(
        core_p2p: Vec<NetInfoEntry>,
        https: Vec<NetInfoEntry>,
    ) -> MasternodeNetInfo {
        use super::core_v24_vectors::*;
        use dashcore::transaction::TransactionPayload;

        let Some(TransactionPayload::ProviderUpdateServicePayloadType(update)) =
            decode(EVO_PRO_UP_SERV_TX).special_transaction_payload
        else {
            panic!("ProUpServTx");
        };
        let mut service_address = update.service_address;
        let MasternodeNetInfo::Extended(net_info) = &mut service_address else {
            panic!("version 3 carries extended addresses");
        };
        let mut replaced = 0;
        for (purpose, entries) in &mut net_info.purposes {
            match purpose {
                NetInfoPurpose::CoreP2P => *entries = core_p2p.clone(),
                NetInfoPurpose::PlatformHttps => *entries = https.clone(),
                NetInfoPurpose::PlatformP2P => continue,
            }
            replaced += 1;
        }
        assert_eq!(replaced, 2, "the vector has CoreP2P and PlatformHttps");
        service_address
    }

    /// An IPv4 service entry at `octets` with `port`.
    fn ipv4_entry(octets: [u8; 4], port: u16) -> NetInfoEntry {
        NetInfoEntry::Service {
            network: Bip155Network::Ipv4,
            addr: octets.to_vec(),
            port,
        }
    }

    /// An IPv6 service entry holding the IPv4-mapped form of `octets`.
    fn ipv4_mapped_entry(octets: [u8; 4], port: u16) -> NetInfoEntry {
        NetInfoEntry::Service {
            network: Bip155Network::Ipv6,
            addr: std::net::Ipv4Addr::from(octets)
                .to_ipv6_mapped()
                .octets()
                .to_vec(),
            port,
        }
    }

    /// An IPv4-mapped IPv6 address is the IPv4 address it maps, on either
    /// side of the comparison: an HTTPS entry written one way serves a
    /// service address written the other way.
    #[test]
    fn should_match_an_ipv4_mapped_https_entry_to_the_service_ip() {
        let ip = [68, 67, 122, 237];

        let mapped_https = v3_service_address(
            vec![ipv4_entry(ip, 20001)],
            vec![ipv4_mapped_entry(ip, 443)],
        );
        assert_eq!(platform_http_port(&mapped_https, None), Some(443));

        let mapped_service = v3_service_address(
            vec![ipv4_mapped_entry(ip, 20001)],
            vec![ipv4_entry(ip, 443)],
        );
        assert_eq!(platform_http_port(&mapped_service, None), Some(443));
    }

    /// Each `PlatformHttps` entry that cannot give the port is skipped on its
    /// own, so a matching entry after it still gives the port: one at another
    /// IP, a domain, one on a network without an IP address, IPv4 and IPv6
    /// entries of the wrong length, and an invalid entry. Every skipped
    /// entry has its own port, so taking any of them shows in the result.
    #[test]
    fn should_skip_each_unusable_https_entry_before_a_matching_one() {
        let ip = [68, 67, 122, 237];
        let https = vec![
            ipv4_entry([10, 0, 0, 1], 441),
            NetInfoEntry::Domain {
                host: "dapi.example.org".to_string(),
                port: 442,
            },
            NetInfoEntry::Service {
                network: Bip155Network::TorV3,
                addr: vec![7; 32],
                port: 443,
            },
            NetInfoEntry::Service {
                network: Bip155Network::Ipv4,
                addr: ip[..3].to_vec(),
                port: 444,
            },
            NetInfoEntry::Service {
                network: Bip155Network::Ipv6,
                addr: ip.to_vec(),
                port: 445,
            },
            NetInfoEntry::Invalid,
            ipv4_entry(ip, 8443),
        ];

        let service_address = v3_service_address(vec![ipv4_entry(ip, 20001)], https);
        assert_eq!(platform_http_port(&service_address, None), Some(8443));
    }

    /// A native IPv6 service address renders bracketed, `[ip]:port`, so a
    /// host can split it into IP and port, and a ProRegTx and a ProUpServTx
    /// render the same address alike. An IPv4-mapped address renders as
    /// IPv4.
    #[test]
    fn should_bracket_a_native_ipv6_service_address() {
        use dashcore::transaction::TransactionPayload;
        use std::net::{Ipv4Addr, SocketAddr};

        let native: SocketAddr = "[2001:db8::1]:20001".parse().expect("socket address");
        let mut registration = decode_tx(PROREG_HEX);
        if let Some(TransactionPayload::ProviderRegistrationPayloadType(p)) =
            &mut registration.special_transaction_payload
        {
            p.service_address = MasternodeNetInfo::Legacy(native);
        }
        let mut update = decode_tx(PROUPSERV_HEX);
        if let Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) =
            &mut update.special_transaction_payload
        {
            p.service_address = MasternodeNetInfo::Legacy(native);
        }
        for tx in [&registration, &update] {
            assert_eq!(
                provider_payload_fields(tx).service_address.as_deref(),
                Some("[2001:db8::1]:20001")
            );
        }

        let mapped = SocketAddr::new(
            Ipv4Addr::new(52, 36, 64, 148).to_ipv6_mapped().into(),
            19999,
        );
        assert_eq!(provider_ip_port(mapped), "52.36.64.148:19999");
    }

    /// An empty script is never the owner payout: not an empty
    /// `script_payout` without a list, nor an empty list, nor a single share
    /// that pays an empty script.
    #[test]
    fn should_never_take_an_empty_script_as_the_owner_payout() {
        use dashcore::hashes::Hash;
        use dashcore::{PubkeyHash, ScriptBuf};

        let empty = ScriptBuf::new();
        let paying = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([1; 20]));
        let share = |script_payout: &ScriptBuf| MasternodePayoutShare {
            script_payout: script_payout.clone(),
            reward: 10_000,
        };

        assert_eq!(owner_payout_script(&empty, None), None);
        assert_eq!(owner_payout_script(&empty, Some(&[])), None);
        assert_eq!(owner_payout_script(&empty, Some(&[share(&empty)])), None);
        assert_eq!(
            owner_payout_script(&empty, Some(&[share(&paying)])),
            Some(&paying)
        );
        assert_eq!(owner_payout_script(&paying, None), Some(&paying));
    }
}
