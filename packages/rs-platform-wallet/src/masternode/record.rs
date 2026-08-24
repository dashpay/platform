//! Masternode records: the wallet-side model of a masternode / evonode.
//!
//! [`aggregate_masternodes`] folds a wallet's retained provider special
//! transactions (ProRegTx / ProUpServTx / ProUpRegTx / ProUpRevTx) into one
//! [`MasternodeRecord`] per proTxHash, resolving the displayed
//! [`MasternodeStatus`] against the deterministic masternode list through an
//! injected [`ListMembership`] lookup. Everything here is pure and
//! host-agnostic; the FFI crates only marshal the results.

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

/// Rebuild an `"ip:port"` string from a ProUpServTx-style little-endian
/// IPv6-mapped `u128` address + `port`, collapsing IPv4-mapped addresses
/// to V4 so a normal masternode renders as `"1.2.3.4:port"`.
pub fn provider_ip_port(ip_address: u128, port: u16) -> String {
    let v6 = std::net::Ipv6Addr::from(ip_address.to_le_bytes());
    let ip = v6
        .to_ipv4_mapped()
        .map(std::net::IpAddr::V4)
        .unwrap_or(std::net::IpAddr::V6(v6));
    format!("{}:{}", ip, port)
}

/// Provider (masternode) special-transaction payload fields lifted for
/// host UIs. All optional / gated — only a ProRegTx or ProUpServTx
/// populates them. The single seam where the DIP-3 payload is decoded;
/// the FFI layers only marshal the flat results.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ProviderPayloadFields {
    /// Service endpoint as `"ip:port"`.
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
/// the returned service-address `String`.
pub fn provider_payload_fields(tx: &dashcore::Transaction) -> ProviderPayloadFields {
    use dashcore::transaction::TransactionPayload;

    match &tx.special_transaction_payload {
        Some(TransactionPayload::ProviderRegistrationPayloadType(p)) => ProviderPayloadFields {
            service_address: Some(p.service_address.to_string()),
            pro_tx_hash: None,
            collateral: Some((
                provider_hash_to_32(p.collateral_outpoint.txid.as_ref()),
                p.collateral_outpoint.vout,
            )),
            owner_key_hash: Some(provider_hash_to_20(p.owner_key_hash.as_ref())),
            voting_key_hash: Some(provider_hash_to_20(p.voting_key_hash.as_ref())),
        },
        Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) => ProviderPayloadFields {
            service_address: Some(provider_ip_port(p.ip_address, p.port)),
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
    /// (owner payout). Encoded to a base58 address by `masternode_entry_ffi`
    /// where the network is available.
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
                    agg.service_address = Some(p.service_address.to_string());
                    agg.platform_http_port = p.platform_http_port;
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
                    agg.payout_script = Some(p.script_payout.as_bytes().to_vec());
                    agg.payout_height = height;
                }
            }
            Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) => {
                if agg.service_address.is_none() || height >= agg.service_height {
                    agg.service_address = Some(provider_ip_port(p.ip_address, p.port));
                    agg.platform_http_port = p.platform_http_port;
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
                    agg.payout_script = Some(p.script_payout.as_bytes().to_vec());
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
    /// address — reconstructed here from its little-endian IPv6-mapped
    /// `ip_address` + `port` — plus an explicit `pro_tx_hash` linking it
    /// to the registration. Fixture is rust-dashcore's own
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
            "ProUpServTx endpoint must be rebuilt from ip_address + port"
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
        // #4116 key-ownership extraction: operator BLS key + payout script
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
                ip_address: 42,
                port,
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
                        ip_address: 42,
                        port: 19999,
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
}
