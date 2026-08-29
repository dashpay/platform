//! Tracked masternodes: nodes the user follows that belong to NO wallet.
//!
//! A tracked masternode is a registry row — proTxHash, optional label, and a
//! cached [`TrackedMasternodeSnapshot`] of everything the wallet layer has
//! learned about the node from three sources:
//!
//! * the deterministic masternode list (service address, operator key,
//!   voting key id, platform node id, validity) — refreshed on every read;
//! * the node's Platform **owner identity** (id = display-order proTxHash;
//!   key 0 = payout-address TRANSFER key, key 1 = owner OWNER key) and
//!   **operator identity** (operator payout TRANSFER key) — the only
//!   sources for the owner / payout key hashes an SPV client can't see;
//! * the ProRegTx itself via DAPI Core `getTransaction` (registration
//!   height, collateral, original keys / payout script).
//!
//! Secrets never enter this module: keys a user attaches to a tracked node
//! live in the host's secure storage (Keychain / Keystore) and are passed
//! per call into [`PlatformWalletManager::tracked_masternode_withdraw`],
//! mirroring `dash_sdk_contested_resource_cast_vote`.
//!
//! Persistence goes through
//! [`PlatformWalletPersistence::persist_tracked_masternodes`] /
//! [`load_tracked_masternodes`](PlatformWalletPersistence::load_tracked_masternodes)
//! as a whole-set replace per network (the set is user-curated and small).
//! A backend that doesn't implement the pair simply keeps tracking
//! session-scoped; hosts read
//! [`PersistenceCapabilities::TRACKED_MASTERNODES`] to know which they got.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use dash_sdk::platform::Fetch;
use dashcore::hashes::Hash;
use dashcore::transaction::special_transaction::provider_registration::ProviderMasternodeType;
use dashcore::transaction::TransactionPayload;
use dashcore::{Address as DashAddress, Network};
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, Purpose};
use dpp::prelude::Identifier;
use serde_json::{json, Value};

use super::list::MasternodeListSummary;
use super::locator::{p2pkh_script_hash, MasternodeKeyReference, MasternodeKeyRole};
use super::record::{ListMembership, MasternodeRecord, MasternodeSource, MasternodeStatus};
use crate::changeset::{PersistenceCapabilities, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::manager::PlatformWalletManager;
use crate::wallet::masternode_withdrawal::{
    execute_masternode_withdrawal, MasternodeWithdrawalKey, RawSecretCoreSigner,
};

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// What the ProRegTx said at registration. Fetched once (DAPI Core
/// `getTransaction`) and cached; the DML / Platform snapshots carry the
/// *current* values where they can change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationDetails {
    /// Confirmation height of the ProRegTx (0 = still unknown).
    pub height: u32,
    /// Collateral outpoint (txid wire bytes, vout).
    pub collateral: ([u8; 32], u32),
    pub owner_key_hash: [u8; 20],
    pub voting_key_hash: [u8; 20],
    pub operator_public_key: [u8; 48],
    /// Raw payout script as registered.
    pub payout_script: Vec<u8>,
    /// `"ip:port"` as registered.
    pub service_address: Option<String>,
    pub is_evonode: bool,
    pub platform_node_id: Option<[u8; 20]>,
    pub platform_http_port: Option<u16>,
}

/// Key hashes learned from the node's Platform identities — the fields the
/// masternode list does not carry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlatformKeySnapshot {
    /// OWNER key of the owner identity.
    pub owner_key_hash: Option<[u8; 20]>,
    /// TRANSFER key of the owner identity = hash160 behind the CURRENT
    /// payout address (what a withdrawal is signed with / paid to).
    pub payout_key_hash: Option<[u8; 20]>,
    /// TRANSFER key of the operator identity.
    pub operator_payout_key_hash: Option<[u8; 20]>,
    /// Owner identity balance in credits at the last refresh (the
    /// claimable amount). Display hint only — hosts re-read live before a
    /// withdrawal.
    pub owner_identity_balance: Option<u64>,
}

/// Everything learned about a tracked masternode so far. Every field is
/// re-fetchable; missing pieces stay `None` and the record models them as
/// unknown rather than inventing defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackedMasternodeSnapshot {
    /// The node's DML entry as of the last refresh.
    pub list: Option<MasternodeListSummary>,
    /// The node has been seen on the list at least once — distinguishes
    /// "retired" (was listed, now gone) from "never confirmed".
    pub ever_listed: bool,
    pub registration: Option<RegistrationDetails>,
    pub platform: Option<PlatformKeySnapshot>,
    /// Unix seconds of the last fully successful
    /// [`PlatformWalletManager::refresh_tracked_masternode`].
    pub refreshed_at: Option<u64>,
}

/// One tracked masternode — the persisted registry row. No secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedMasternode {
    /// proTxHash, wire order (like every other `[u8; 32]` proTxHash in
    /// this crate).
    pub pro_tx_hash: [u8; 32],
    pub label: Option<String>,
    /// Unix seconds when the user tracked it.
    pub added_at: u64,
    pub snapshot: TrackedMasternodeSnapshot,
}

impl TrackedMasternode {
    /// The key references this node's snapshot can verify a key against.
    /// Current values win over registration-time values.
    pub fn key_reference(&self) -> MasternodeKeyReference {
        let list = self.snapshot.list.as_ref();
        let reg = self.snapshot.registration.as_ref();
        let platform = self.snapshot.platform.as_ref();
        MasternodeKeyReference {
            owner_key_hash: platform
                .and_then(|p| p.owner_key_hash)
                .or(reg.map(|r| r.owner_key_hash)),
            voting_key_id: list
                .map(|l| l.voting_key_id)
                .or(reg.map(|r| r.voting_key_hash)),
            operator_public_key: list
                .map(|l| l.operator_public_key)
                .or(reg.map(|r| r.operator_public_key)),
            platform_node_id: list
                .and_then(|l| l.platform_node_id)
                .or(reg.and_then(|r| r.platform_node_id)),
            payout_key_hash: platform
                .and_then(|p| p.payout_key_hash)
                .or(reg.and_then(|r| p2pkh_script_hash(&r.payout_script))),
            operator_payout_key_hash: platform.and_then(|p| p.operator_payout_key_hash),
        }
    }

    /// Build the display record. `list_now` is the node's CURRENT list
    /// entry: `None` = the DML isn't available, `Some(None)` = available
    /// but the node is gone (retired), `Some(Some(_))` = present.
    pub fn record(&self, list_now: Option<Option<&MasternodeListSummary>>) -> MasternodeRecord {
        let live = list_now.flatten();
        let cached = self.snapshot.list.as_ref();
        let list = live.or(cached);
        let reg = self.snapshot.registration.as_ref();
        let platform = self.snapshot.platform.as_ref();

        let membership = match list_now {
            None => ListMembership::ListUnavailable,
            Some(None) => ListMembership::Absent,
            Some(Some(entry)) => {
                if entry.is_valid {
                    ListMembership::ValidEntry
                } else {
                    ListMembership::InvalidEntry
                }
            }
        };

        // Payout: the owner identity's CURRENT transfer key wins over the
        // registered script; both may be absent before enrichment.
        let payout_script = platform
            .and_then(|p| p.payout_key_hash)
            .map(|hash| p2pkh_script(&hash))
            .or_else(|| reg.map(|r| r.payout_script.clone()));

        MasternodeRecord {
            pro_tx_hash: self.pro_tx_hash,
            has_registration: reg.is_some(),
            registration_height: reg.map(|r| r.height).unwrap_or(0),
            service_address: list
                .and_then(|l| l.service_address.map(|a| a.to_string()))
                .or_else(|| reg.and_then(|r| r.service_address.clone())),
            platform_http_port: list
                .and_then(|l| l.platform_http_port)
                .or(reg.and_then(|r| r.platform_http_port)),
            is_evonode: list
                .map(|l| l.is_evonode)
                .unwrap_or_else(|| reg.map(|r| r.is_evonode).unwrap_or(false)),
            owner_key_hash: platform
                .and_then(|p| p.owner_key_hash)
                .or(reg.map(|r| r.owner_key_hash)),
            voting_key_hash: list
                .map(|l| l.voting_key_id)
                .or(reg.map(|r| r.voting_key_hash)),
            operator_public_key: list
                .map(|l| l.operator_public_key)
                .or(reg.map(|r| r.operator_public_key)),
            platform_node_id: list
                .and_then(|l| l.platform_node_id)
                .or(reg.and_then(|r| r.platform_node_id)),
            payout_script,
            collateral: reg.map(|r| r.collateral),
            revoked: false,
            revocation_reason: 0,
            tx_count: 0,
            type_index: 0, // assigned by the lister
            status: MasternodeStatus::from_membership(membership),
            source: MasternodeSource::Tracked,
            order_index: 0, // assigned by the lister
            operator_key_index: None,
            platform_key_index: None,
            platform_ownership_checked: false,
            label: self.label.clone(),
            service_height: 0,
            voting_height: 0,
            operator_height: 0,
            platform_node_height: 0,
            payout_height: 0,
        }
    }
}

/// `OP_DUP OP_HASH160 <hash> OP_EQUALVERIFY OP_CHECKSIG`.
fn p2pkh_script(hash: &[u8; 20]) -> Vec<u8> {
    let mut script = Vec::with_capacity(25);
    script.extend_from_slice(&[0x76, 0xa9, 0x14]);
    script.extend_from_slice(hash);
    script.extend_from_slice(&[0x88, 0xac]);
    script
}

// ---------------------------------------------------------------------------
// What a set of attached keys enables
// ---------------------------------------------------------------------------

/// What a host can do with a masternode given the key roles it holds for
/// it. Pure policy shared by both mobile hosts, so the gating never
/// diverges.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MasternodeCapabilities {
    /// Withdraw the owner identity's claimable balance (owner key — the
    /// identity's OWNER key — or the payout-address TRANSFER key; a
    /// withdrawal transition accepts either purpose).
    pub can_withdraw: bool,
    /// Cast governance / contested-resource votes.
    pub can_vote: bool,
    /// Sign ProUpServTx (operator BLS key).
    pub can_update_service: bool,
    /// Prove which Tenderdash node this is (no wallet action uses it).
    pub identifies_platform_node: bool,
}

/// Capabilities for the roles a host holds keys for.
pub fn capabilities_for_roles(
    roles: impl IntoIterator<Item = MasternodeKeyRole>,
) -> MasternodeCapabilities {
    let mut caps = MasternodeCapabilities::default();
    for role in roles {
        match role {
            MasternodeKeyRole::Owner | MasternodeKeyRole::OwnerPayout => caps.can_withdraw = true,
            MasternodeKeyRole::Voting => caps.can_vote = true,
            MasternodeKeyRole::Operator => caps.can_update_service = true,
            MasternodeKeyRole::PlatformNode => caps.identifies_platform_node = true,
            MasternodeKeyRole::OperatorPayout => {}
        }
    }
    caps
}

// ---------------------------------------------------------------------------
// Snapshot JSON codec (persistence wire format)
// ---------------------------------------------------------------------------
//
// Hand-rolled over `serde_json::Value` so the crate's optional `serde`
// derive feature stays optional. Versioned; readers are lenient (a missing
// or malformed section is simply "not learned yet"), so the format can grow
// fields without a migration.

const SNAPSHOT_JSON_VERSION: u64 = 1;

fn hex_opt<const N: usize>(bytes: Option<[u8; N]>) -> Value {
    bytes
        .map(hex::encode)
        .map(Value::String)
        .unwrap_or(Value::Null)
}

fn parse_hex<const N: usize>(value: &Value) -> Option<[u8; N]> {
    let bytes = hex::decode(value.as_str()?).ok()?;
    bytes.as_slice().try_into().ok()
}

fn list_to_json(list: &MasternodeListSummary) -> Value {
    json!({
        "proTxHash": hex::encode(list.pro_tx_hash),
        "serviceAddress": list.service_address.map(|a| a.to_string()),
        "platformHttpPort": list.platform_http_port,
        "operatorPubKey": hex::encode(list.operator_public_key),
        "votingKeyId": hex::encode(list.voting_key_id),
        "platformNodeId": hex_opt(list.platform_node_id),
        "isValid": list.is_valid,
        "isEvonode": list.is_evonode,
        "hasExtendedNetInfo": list.has_extended_net_info,
        "operatorKeyIsLegacy": list.operator_key_is_legacy,
    })
}

fn list_from_json(value: &Value) -> Option<MasternodeListSummary> {
    Some(MasternodeListSummary {
        pro_tx_hash: parse_hex(&value["proTxHash"])?,
        service_address: value["serviceAddress"]
            .as_str()
            .and_then(|s| s.parse().ok()),
        platform_http_port: value["platformHttpPort"].as_u64().map(|p| p as u16),
        operator_public_key: parse_hex(&value["operatorPubKey"])?,
        voting_key_id: parse_hex(&value["votingKeyId"])?,
        platform_node_id: parse_hex(&value["platformNodeId"]),
        is_valid: value["isValid"].as_bool()?,
        is_evonode: value["isEvonode"].as_bool()?,
        // Absent on snapshots persisted before the field existed; the
        // next refresh rewrites it from the live entry.
        has_extended_net_info: value["hasExtendedNetInfo"].as_bool().unwrap_or(false),
        operator_key_is_legacy: value["operatorKeyIsLegacy"].as_bool().unwrap_or(false),
    })
}

fn registration_to_json(reg: &RegistrationDetails) -> Value {
    json!({
        "height": reg.height,
        "collateralTxid": hex::encode(reg.collateral.0),
        "collateralVout": reg.collateral.1,
        "ownerKeyHash": hex::encode(reg.owner_key_hash),
        "votingKeyHash": hex::encode(reg.voting_key_hash),
        "operatorPubKey": hex::encode(reg.operator_public_key),
        "payoutScript": hex::encode(&reg.payout_script),
        "serviceAddress": reg.service_address,
        "isEvonode": reg.is_evonode,
        "platformNodeId": hex_opt(reg.platform_node_id),
        "platformHttpPort": reg.platform_http_port,
    })
}

fn registration_from_json(value: &Value) -> Option<RegistrationDetails> {
    Some(RegistrationDetails {
        height: value["height"].as_u64()? as u32,
        collateral: (
            parse_hex(&value["collateralTxid"])?,
            value["collateralVout"].as_u64()? as u32,
        ),
        owner_key_hash: parse_hex(&value["ownerKeyHash"])?,
        voting_key_hash: parse_hex(&value["votingKeyHash"])?,
        operator_public_key: parse_hex(&value["operatorPubKey"])?,
        payout_script: hex::decode(value["payoutScript"].as_str()?).ok()?,
        service_address: value["serviceAddress"].as_str().map(str::to_string),
        is_evonode: value["isEvonode"].as_bool()?,
        platform_node_id: parse_hex(&value["platformNodeId"]),
        platform_http_port: value["platformHttpPort"].as_u64().map(|p| p as u16),
    })
}

fn platform_to_json(platform: &PlatformKeySnapshot) -> Value {
    json!({
        "ownerKeyHash": hex_opt(platform.owner_key_hash),
        "payoutKeyHash": hex_opt(platform.payout_key_hash),
        "operatorPayoutKeyHash": hex_opt(platform.operator_payout_key_hash),
        "ownerIdentityBalance": platform.owner_identity_balance,
    })
}

fn platform_from_json(value: &Value) -> Option<PlatformKeySnapshot> {
    if !value.is_object() {
        return None;
    }
    Some(PlatformKeySnapshot {
        owner_key_hash: parse_hex(&value["ownerKeyHash"]),
        payout_key_hash: parse_hex(&value["payoutKeyHash"]),
        operator_payout_key_hash: parse_hex(&value["operatorPayoutKeyHash"]),
        owner_identity_balance: value["ownerIdentityBalance"].as_u64(),
    })
}

/// Serialize a snapshot for the persistence row.
pub fn snapshot_to_json(snapshot: &TrackedMasternodeSnapshot) -> String {
    json!({
        "v": SNAPSHOT_JSON_VERSION,
        "everListed": snapshot.ever_listed,
        "refreshedAt": snapshot.refreshed_at,
        "list": snapshot.list.as_ref().map(list_to_json),
        "registration": snapshot.registration.as_ref().map(registration_to_json),
        "platform": snapshot.platform.as_ref().map(platform_to_json),
    })
    .to_string()
}

/// Read a persisted snapshot. Lenient: an unreadable document yields the
/// empty snapshot ("nothing learned yet"), and each section is optional —
/// tracked rows are cache, every field is re-fetchable.
pub fn snapshot_from_json(text: &str) -> TrackedMasternodeSnapshot {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return TrackedMasternodeSnapshot::default();
    };
    TrackedMasternodeSnapshot {
        list: list_from_json(&value["list"]),
        ever_listed: value["everListed"].as_bool().unwrap_or(false),
        registration: registration_from_json(&value["registration"]),
        platform: platform_from_json(&value["platform"]),
        refreshed_at: value["refreshedAt"].as_u64(),
    }
}

// ---------------------------------------------------------------------------
// Registry (manager surface)
// ---------------------------------------------------------------------------

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn not_tracked(pro_tx_hash: &[u8; 32]) -> PlatformWalletError {
    let mut display = *pro_tx_hash;
    display.reverse();
    PlatformWalletError::InvalidParameter(format!(
        "masternode {} is not tracked",
        hex::encode(display)
    ))
}

/// Assign `order_index` / `type_index` over a sorted record list, the same
/// per-type numbering the wallet aggregation uses.
fn number_records(records: &mut [MasternodeRecord]) {
    let (mut evonode_n, mut masternode_n) = (0u32, 0u32);
    for (idx, record) in records.iter_mut().enumerate() {
        record.order_index = idx as u32;
        if record.is_evonode {
            evonode_n += 1;
            record.type_index = evonode_n;
        } else {
            masternode_n += 1;
            record.type_index = masternode_n;
        }
    }
}

/// The manager's tracked-masternode registry: the rows plus the per-node
/// refresh gates. Both live behind one `Arc`, so every
/// [`TrackedMasternodes`] handle a manager hands out shares the rows AND
/// the gates that order the refreshes writing to them.
#[derive(Default)]
pub(crate) struct TrackedMasternodeRegistry {
    rows: std::sync::RwLock<TrackedMasternodeMap>,
    /// One gate per node, created on demand and dropped once no pass holds
    /// it. Held for a whole refresh pass — see [`refresh_row_and_persist`].
    gates: std::sync::Mutex<BTreeMap<[u8; 32], std::sync::Arc<tokio::sync::Mutex<()>>>>,
}

impl TrackedMasternodeRegistry {
    /// Take `pro_tx_hash`'s refresh gate. The `std` lock over the gate map
    /// is released before the `await`, so waiting for a gate never blocks
    /// the runtime thread.
    async fn refresh_gate(&self, pro_tx_hash: &[u8; 32]) -> tokio::sync::OwnedMutexGuard<()> {
        let gate = {
            let mut gates = self
                .gates
                .lock()
                .expect("tracked masternode gate map lock poisoned");
            // Every pass keeps its own clone alive for its whole duration,
            // so a gate the map alone references has no pass on it.
            gates.retain(|_, gate| std::sync::Arc::strong_count(gate) > 1);
            std::sync::Arc::clone(gates.entry(*pro_tx_hash).or_default())
        };
        gate.lock_owned().await
    }
}

/// Shared handle to the tracked-masternode registry and everything its
/// operations need (SPV for the list, the SDK for Platform / DAPI, the
/// persister for durability). Cloneable and `Send + Sync`, so hosts can run
/// the network operations ([`Self::refresh`], [`Self::withdraw`]) on a
/// worker without holding the manager. Built by
/// [`PlatformWalletManager::tracked_masternodes_service`]; every clone
/// shares one registry.
#[derive(Clone)]
pub struct TrackedMasternodes {
    registry: std::sync::Arc<TrackedMasternodeRegistry>,
    spv: std::sync::Arc<crate::spv::SpvRuntime>,
    sdk: std::sync::Arc<dash_sdk::Sdk>,
    persister: std::sync::Arc<dyn PlatformWalletPersistence>,
    network: Network,
}

/// Apply one registry mutation and durably replace the persisted set as one
/// linearizable operation.
fn mutate_registry_and_persist<T>(
    registry: &TrackedMasternodeRegistry,
    persister: &dyn PlatformWalletPersistence,
    network: Network,
    mutation: impl FnOnce(&mut TrackedMasternodeMap) -> Result<T, PlatformWalletError>,
) -> Result<T, PlatformWalletError> {
    let mut guard = registry
        .rows
        .write()
        .expect("tracked masternode registry lock poisoned");
    let before = guard.clone();
    let result = match mutation(&mut guard) {
        Ok(result) => result,
        Err(error) => {
            *guard = before;
            return Err(error);
        }
    };
    let records: Vec<TrackedMasternode> = guard.values().cloned().collect();
    if let Err(e) = persister.persist_tracked_masternodes(network, &records) {
        *guard = before;
        return Err(PlatformWalletError::WalletCreation(format!(
            "failed to persist tracked masternodes: {e}"
        )));
    }
    Ok(result)
}

/// What a refresh pass's network half learned besides the snapshot: the
/// masternode list it read (the returned record renders the node's LIVE
/// status from it) and the first error it hit, if any.
#[derive(Default)]
struct RefreshOutcome {
    list_now: Option<Vec<MasternodeListSummary>>,
    first_error: Option<PlatformWalletError>,
}

/// One refresh pass over `pro_tx_hash`: take the node's refresh gate, read
/// the row under it, hand its snapshot to `learn`, then write the learned
/// snapshot back and durably replace the persisted set.
///
/// The gate spans the read AND the write, so two passes over one node never
/// both start from the same snapshot: `learn` always sees everything
/// earlier passes learned, and keeps a field by simply not overwriting it —
/// otherwise the pass that finished last would put its own pre-read clone
/// back over the other's findings and persist the loss. Serializing also
/// spares the second pass the network work the first one already did.
///
/// Only the snapshot is written: a relabel that raced the pass keeps its
/// label, and an untrack that raced it wins — the row is gone, so the pass
/// errors instead of resurrecting it.
async fn refresh_row_and_persist<F, Fut>(
    registry: &TrackedMasternodeRegistry,
    persister: &dyn PlatformWalletPersistence,
    network: Network,
    pro_tx_hash: &[u8; 32],
    learn: F,
) -> Result<(TrackedMasternode, RefreshOutcome), PlatformWalletError>
where
    F: FnOnce(TrackedMasternodeSnapshot) -> Fut,
    Fut: std::future::Future<Output = (TrackedMasternodeSnapshot, RefreshOutcome)>,
{
    let _gate = registry.refresh_gate(pro_tx_hash).await;

    let mut tracked = registry
        .rows
        .read()
        .expect("tracked masternode registry lock poisoned")
        .get(pro_tx_hash)
        .cloned()
        .ok_or_else(|| not_tracked(pro_tx_hash))?;

    let (snapshot, outcome) = learn(tracked.snapshot).await;
    tracked.snapshot = snapshot;

    let label = mutate_registry_and_persist(registry, persister, network, |guard| {
        match guard.get_mut(pro_tx_hash) {
            Some(live) => {
                live.snapshot = tracked.snapshot.clone();
                Ok(live.label.clone())
            }
            None => Err(not_tracked(pro_tx_hash)),
        }
    })?;
    tracked.label = label;

    Ok((tracked, outcome))
}

impl std::fmt::Debug for TrackedMasternodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackedMasternodes")
            .field("network", &self.network)
            .finish_non_exhaustive()
    }
}

impl TrackedMasternodes {
    /// Whether the configured persister keeps tracked masternodes across
    /// restarts. When `false`, tracking still works but is session-scoped —
    /// hosts surface that rather than pretending durability.
    pub fn durable(&self) -> bool {
        self.persister
            .persistence_capabilities()
            .contains(PersistenceCapabilities::TRACKED_MASTERNODES)
    }

    /// Apply one registry mutation and durably replace the persisted set as
    /// one linearizable operation. The write guard deliberately spans the
    /// persistence call: every backend is bound by the trait's non-reentrant
    /// callback contract, and releasing it earlier would let an older
    /// snapshot overwrite a newer concurrent mutation.
    ///
    /// The registry is user-curated and small, so retaining a complete
    /// before-image is cheap and lets a rejected write restore exactly the
    /// in-memory state the caller observed before the operation.
    fn mutate_and_persist<T>(
        &self,
        mutation: impl FnOnce(&mut TrackedMasternodeMap) -> Result<T, PlatformWalletError>,
    ) -> Result<T, PlatformWalletError> {
        mutate_registry_and_persist(
            self.registry.as_ref(),
            self.persister.as_ref(),
            self.network,
            mutation,
        )
    }

    /// Hydrate the registry from the persister. A load failure logs and
    /// leaves the registry empty rather than failing wallet hydration.
    pub(crate) fn load_from_persistence(&self) {
        match self.persister.load_tracked_masternodes(self.network) {
            Ok(rows) => {
                let mut guard = self
                    .registry
                    .rows
                    .write()
                    .expect("tracked masternode registry lock poisoned");
                for row in rows {
                    guard.insert(row.pro_tx_hash, row);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to load tracked masternodes; starting empty");
            }
        }
    }

    /// The wire proTxHashes currently tracked (for locate's
    /// `already_tracked` mark).
    pub fn hashes(&self) -> std::collections::BTreeSet<[u8; 32]> {
        self.registry
            .rows
            .read()
            .expect("tracked masternode registry lock poisoned")
            .keys()
            .copied()
            .collect()
    }

    /// The tracked row itself (snapshot included), when present.
    pub fn get(&self, pro_tx_hash: &[u8; 32]) -> Option<TrackedMasternode> {
        self.registry
            .rows
            .read()
            .expect("tracked masternode registry lock poisoned")
            .get(pro_tx_hash)
            .cloned()
    }

    /// Track `pro_tx_hash` (wire order). Seeds the snapshot from the
    /// current DML entry when the list is available (local; no network —
    /// call [`Self::refresh`] afterwards for the Platform / registration
    /// details). Errors when already tracked. Blocking.
    pub fn track_blocking(
        &self,
        pro_tx_hash: [u8; 32],
        label: Option<String>,
    ) -> Result<MasternodeRecord, PlatformWalletError> {
        let list_now = self.spv.masternode_list_summaries_blocking();
        let entry = list_now
            .as_ref()
            .map(|summaries| summaries.iter().find(|s| s.pro_tx_hash == pro_tx_hash));

        let tracked = self.mutate_and_persist(|guard| {
            if guard.contains_key(&pro_tx_hash) {
                return Err(PlatformWalletError::InvalidParameter(
                    "this masternode is already tracked".to_string(),
                ));
            }
            let entry = entry.flatten();
            let tracked = TrackedMasternode {
                pro_tx_hash,
                label: label.filter(|l| !l.trim().is_empty()),
                added_at: now_unix(),
                snapshot: TrackedMasternodeSnapshot {
                    list: entry.cloned(),
                    ever_listed: entry.is_some(),
                    ..Default::default()
                },
            };
            guard.insert(pro_tx_hash, tracked.clone());
            Ok(tracked)
        })?;

        Ok(tracked.record(entry))
    }

    /// Stop tracking. Returns `Ok(true)` when a row was removed. The
    /// host owns any attached keys (secure storage) and deletes them
    /// itself. Blocking.
    pub fn untrack_blocking(&self, pro_tx_hash: &[u8; 32]) -> Result<bool, PlatformWalletError> {
        self.mutate_and_persist(|guard| Ok(guard.remove(pro_tx_hash).is_some()))
    }

    /// Rename a tracked masternode (`None` / blank clears the label).
    /// Blocking.
    pub fn set_label_blocking(
        &self,
        pro_tx_hash: &[u8; 32],
        label: Option<String>,
    ) -> Result<(), PlatformWalletError> {
        self.mutate_and_persist(|guard| {
            let tracked = guard
                .get_mut(pro_tx_hash)
                .ok_or_else(|| not_tracked(pro_tx_hash))?;
            tracked.label = label.filter(|l| !l.trim().is_empty());
            Ok(())
        })
    }

    /// Every tracked masternode as a display record, with the CURRENT list
    /// entry overlaid (status Active / Inactive / Retired, or Unknown when
    /// the list isn't available). Sorted by when they were tracked.
    /// Blocking.
    pub fn list_blocking(&self) -> Vec<MasternodeRecord> {
        let list_now = self.spv.masternode_list_summaries_blocking();
        self.records_with(list_now)
    }

    fn records_with(&self, list_now: Option<Vec<MasternodeListSummary>>) -> Vec<MasternodeRecord> {
        let mut rows: Vec<TrackedMasternode> = {
            let guard = self
                .registry
                .rows
                .read()
                .expect("tracked masternode registry lock poisoned");
            guard.values().cloned().collect()
        };
        rows.sort_by_key(|t| (t.added_at, t.pro_tx_hash));
        let mut records: Vec<MasternodeRecord> = rows
            .iter()
            .map(|tracked| {
                let entry = list_now
                    .as_ref()
                    .map(|s| s.iter().find(|e| e.pro_tx_hash == tracked.pro_tx_hash));
                tracked.record(entry)
            })
            .collect();
        number_records(&mut records);
        records
    }

    /// Refresh everything the wallet layer can learn about a tracked
    /// masternode: its DML entry (local), its owner / operator identities
    /// on Platform (owner + payout key hashes, claimable balance), and —
    /// once — its ProRegTx via DAPI Core (registration height, collateral,
    /// original keys). Partial results are kept and persisted before an
    /// error is returned, so a flaky step never discards what an earlier
    /// step learned; `refreshed_at` advances only on a fully successful
    /// pass.
    ///
    /// Serialized per masternode: a second refresh of the same node waits
    /// for the one in flight instead of racing it — two passes that both
    /// started from the same snapshot would end with the one finishing last
    /// putting its own pre-read clone back over the other's findings.
    pub async fn refresh(
        &self,
        pro_tx_hash: &[u8; 32],
    ) -> Result<MasternodeRecord, PlatformWalletError> {
        let (tracked, outcome) = refresh_row_and_persist(
            self.registry.as_ref(),
            self.persister.as_ref(),
            self.network,
            pro_tx_hash,
            |snapshot| self.learn(pro_tx_hash, snapshot),
        )
        .await?;

        let entry = outcome
            .list_now
            .as_ref()
            .map(|s| s.iter().find(|e| e.pro_tx_hash == *pro_tx_hash));
        match outcome.first_error {
            Some(e) => Err(e),
            None => Ok(tracked.record(entry)),
        }
    }

    /// The network half of a [`Self::refresh`] pass: everything the wallet
    /// layer can learn about `pro_tx_hash`, merged into `snapshot`. Runs
    /// under the node's refresh gate, so `snapshot` already carries what
    /// earlier passes learned and a step that comes back empty leaves its
    /// field alone rather than clearing it.
    async fn learn(
        &self,
        pro_tx_hash: &[u8; 32],
        mut snapshot: TrackedMasternodeSnapshot,
    ) -> (TrackedMasternodeSnapshot, RefreshOutcome) {
        // 1. Current list entry (local).
        let list_now = self.spv.masternode_list_summaries().await;
        if let Some(Some(entry)) = list_now
            .as_ref()
            .map(|s| s.iter().find(|e| e.pro_tx_hash == *pro_tx_hash))
        {
            snapshot.list = Some(entry.clone());
            snapshot.ever_listed = true;
        }

        let mut display = *pro_tx_hash;
        display.reverse();
        let mut first_error: Option<PlatformWalletError> = None;

        // 2. Owner identity: owner + payout key hashes and the claimable
        // balance.
        match Identity::fetch(self.sdk.as_ref(), Identifier::from(display)).await {
            Ok(Some(identity)) => {
                let mut platform = snapshot.platform.clone().unwrap_or_default();
                for key in identity.public_keys().values() {
                    let data: Option<[u8; 20]> = key.data().as_slice().try_into().ok();
                    match key.purpose() {
                        Purpose::OWNER => {
                            platform.owner_key_hash = data.or(platform.owner_key_hash)
                        }
                        Purpose::TRANSFER => {
                            platform.payout_key_hash = data.or(platform.payout_key_hash)
                        }
                        _ => {}
                    }
                }
                platform.owner_identity_balance = Some(identity.balance());
                snapshot.platform = Some(platform);
            }
            Ok(None) => {
                // No owner identity (node registered before Platform, or a
                // lagging replica) — leave the platform snapshot as-is.
            }
            Err(e) => {
                first_error.get_or_insert(PlatformWalletError::InvalidIdentityData(format!(
                    "failed to fetch the masternode's owner identity: {e}"
                )));
            }
        }

        // 3. Operator identity (needs the operator key — list, else
        // registration).
        let operator_key = snapshot
            .list
            .as_ref()
            .map(|l| l.operator_public_key)
            .or(snapshot
                .registration
                .as_ref()
                .map(|r| r.operator_public_key));
        if let Some(operator_key) = operator_key {
            let operator_id = Identifier::create_operator_identifier(&display, &operator_key);
            match Identity::fetch(self.sdk.as_ref(), operator_id).await {
                Ok(Some(identity)) => {
                    let mut platform = snapshot.platform.clone().unwrap_or_default();
                    platform.operator_payout_key_hash = identity
                        .public_keys()
                        .values()
                        .find(|k| k.purpose() == Purpose::TRANSFER)
                        .and_then(|k| k.data().as_slice().try_into().ok())
                        .or(platform.operator_payout_key_hash);
                    snapshot.platform = Some(platform);
                }
                Ok(None) => {}
                Err(e) => {
                    first_error.get_or_insert(PlatformWalletError::InvalidIdentityData(format!(
                        "failed to fetch the masternode's operator identity: {e}"
                    )));
                }
            }
        }

        // 4. ProRegTx (once): registration height, collateral, original
        // keys / payout script.
        if snapshot.registration.is_none() {
            match self.sdk.get_transaction(&hex::encode(display)).await {
                Ok(Some(fetched)) => {
                    if let Some(details) =
                        registration_from_transaction(&fetched.transaction, fetched.height)
                    {
                        snapshot.registration = Some(details);
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    first_error.get_or_insert(PlatformWalletError::InvalidIdentityData(format!(
                        "failed to fetch the registration transaction: {e}"
                    )));
                }
            }
        }

        if first_error.is_none() {
            snapshot.refreshed_at = Some(now_unix());
        }

        (
            snapshot,
            RefreshOutcome {
                list_now,
                first_error,
            },
        )
    }

    /// Withdraw from a TRACKED masternode's owner identity with a
    /// host-supplied key: the owner key (`role == Owner`; pays the
    /// registered payout address, no destination allowed) or the
    /// payout-address key (`role == OwnerPayout`; destination optional,
    /// defaults to the payout address itself). The key is used for this
    /// call only — nothing is retained. Returns the identity's new
    /// balance in credits.
    pub async fn withdraw(
        &self,
        pro_tx_hash: &[u8; 32],
        amount_credits: u64,
        role: MasternodeKeyRole,
        secret: &[u8; 32],
        destination: Option<String>,
    ) -> Result<u64, PlatformWalletError> {
        let tracked = self
            .get(pro_tx_hash)
            .ok_or_else(|| not_tracked(pro_tx_hash))?;
        let reference = tracked.key_reference();
        let network = self.network;

        let signer = RawSecretCoreSigner::from_bytes(secret)?;
        let key_hash = signer.public_key_hash160();

        let (signing_key, expected, destination) = match role {
            MasternodeKeyRole::Owner => {
                if destination.is_some() {
                    return Err(PlatformWalletError::InvalidParameter(
                        "an owner-key withdrawal pays the registered payout address; a \
                         destination cannot be chosen"
                            .to_string(),
                    ));
                }
                let expected = reference.owner_key_hash.ok_or_else(|| {
                    PlatformWalletError::InvalidParameter(
                        "the masternode's owner key isn't known yet — refresh the node first"
                            .to_string(),
                    )
                })?;
                (MasternodeWithdrawalKey::Owner, expected, None)
            }
            MasternodeKeyRole::OwnerPayout => {
                let expected = reference.payout_key_hash.ok_or_else(|| {
                    PlatformWalletError::InvalidParameter(
                        "the masternode's payout address isn't known yet — refresh the node first"
                            .to_string(),
                    )
                })?;
                // Default destination: the payout address itself.
                let destination_text = match destination {
                    Some(text) => text,
                    None => p2pkh_address(&expected, network),
                };
                let destination = destination_text
                    .parse::<DashAddress<dashcore::address::NetworkUnchecked>>()
                    .map_err(|e| {
                        PlatformWalletError::InvalidParameter(format!(
                            "destination is not a valid Dash address: {e}"
                        ))
                    })?
                    .require_network(network)
                    .map_err(|e| {
                        PlatformWalletError::InvalidParameter(format!(
                            "destination is for another network: {e}"
                        ))
                    })?;
                (
                    MasternodeWithdrawalKey::Transfer,
                    expected,
                    Some(destination),
                )
            }
            _ => {
                return Err(PlatformWalletError::InvalidParameter(
                    "a withdrawal signs with the owner key or the payout-address key".to_string(),
                ));
            }
        };

        // Refuse a mismatched key BEFORE any network work — same
        // derive-and-compare the identity signer re-checks later.
        if key_hash != expected {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "this key does not match the masternode's {} (hash160 {} vs {})",
                match role {
                    MasternodeKeyRole::Owner => "owner key",
                    _ => "payout address key",
                },
                hex::encode(key_hash),
                hex::encode(expected),
            )));
        }

        // The signer ignores the path; pass the DIP-3 owner base so logs
        // stay meaningful.
        let path = crate::wallet::masternode_withdrawal::provider_owner_key_path(network, 0)?;
        execute_masternode_withdrawal(
            self.sdk.as_ref(),
            *pro_tx_hash,
            amount_credits,
            signing_key,
            expected,
            path,
            destination,
            &signer,
        )
        .await
    }
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// The tracked-masternode registry as a cloneable, manager-independent
    /// service handle. Every handle shares this manager's registry.
    pub fn tracked_masternodes_service(&self) -> TrackedMasternodes {
        TrackedMasternodes {
            registry: std::sync::Arc::clone(&self.tracked_masternodes),
            spv: self.spv_arc(),
            sdk: self.sdk_arc(),
            persister: std::sync::Arc::clone(&self.persister) as _,
            network: self.sdk().network,
        }
    }

    /// Hydrate the tracked registry from the persister (startup).
    pub(crate) fn load_tracked_masternodes_from_persistence(&self) {
        self.tracked_masternodes_service().load_from_persistence();
    }

    /// See [`TrackedMasternodes::hashes`].
    pub fn tracked_masternode_hashes(&self) -> std::collections::BTreeSet<[u8; 32]> {
        self.tracked_masternodes_service().hashes()
    }

    /// See [`TrackedMasternodes::get`].
    pub fn tracked_masternode(&self, pro_tx_hash: &[u8; 32]) -> Option<TrackedMasternode> {
        self.tracked_masternodes_service().get(pro_tx_hash)
    }
}

/// Base58 P2PKH address for `hash` on `network`.
fn p2pkh_address(hash: &[u8; 20], network: Network) -> String {
    use dashcore::address::Payload;
    use dashcore::PubkeyHash;
    DashAddress::new(
        network,
        Payload::PubkeyHash(PubkeyHash::from_byte_array(*hash)),
    )
    .to_string()
}

/// Lift a ProRegTx into [`RegistrationDetails`]; `None` for any other
/// transaction.
pub fn registration_from_transaction(
    tx: &dashcore::Transaction,
    height: u32,
) -> Option<RegistrationDetails> {
    let Some(TransactionPayload::ProviderRegistrationPayloadType(p)) =
        &tx.special_transaction_payload
    else {
        return None;
    };
    let mut collateral_txid = [0u8; 32];
    collateral_txid.copy_from_slice(p.collateral_outpoint.txid.as_ref());
    let mut owner_key_hash = [0u8; 20];
    owner_key_hash.copy_from_slice(p.owner_key_hash.as_ref());
    let mut voting_key_hash = [0u8; 20];
    voting_key_hash.copy_from_slice(p.voting_key_hash.as_ref());
    let operator: &[u8; 48] = p.operator_public_key.as_ref();
    Some(RegistrationDetails {
        height,
        collateral: (collateral_txid, p.collateral_outpoint.vout),
        owner_key_hash,
        voting_key_hash,
        operator_public_key: *operator,
        payout_script: p.script_payout.as_bytes().to_vec(),
        service_address: Some(p.service_address.to_string()),
        is_evonode: p.masternode_type == ProviderMasternodeType::HighPerformance,
        platform_node_id: p.platform_node_id.map(|id| id.to_byte_array()),
        platform_http_port: p.platform_http_port,
    })
}

/// Whole-registry map type held by the manager.
pub(crate) type TrackedMasternodeMap = BTreeMap<[u8; 32], TrackedMasternode>;

#[cfg(test)]
mod tests {
    use super::super::list::test_support::{evonode, masternode};
    use super::*;
    use crate::changeset::{ClientStartState, PersistenceError, PlatformWalletChangeSet};
    use crate::wallet::platform_wallet::WalletId;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, RwLock};

    fn snapshot_full() -> TrackedMasternodeSnapshot {
        TrackedMasternodeSnapshot {
            list: Some(evonode(7)),
            ever_listed: true,
            registration: Some(RegistrationDetails {
                height: 1000,
                collateral: ([9u8; 32], 1),
                owner_key_hash: [1u8; 20],
                voting_key_hash: [2u8; 20],
                operator_public_key: [3u8; 48],
                payout_script: p2pkh_script(&[4u8; 20]),
                service_address: Some("1.2.3.4:9999".to_string()),
                is_evonode: true,
                platform_node_id: Some([5u8; 20]),
                platform_http_port: Some(443),
            }),
            platform: Some(PlatformKeySnapshot {
                owner_key_hash: Some([1u8; 20]),
                payout_key_hash: Some([6u8; 20]),
                operator_payout_key_hash: Some([7u8; 20]),
                owner_identity_balance: Some(123_456_789_000),
            }),
            refreshed_at: Some(1_700_000_000),
        }
    }

    #[test]
    fn snapshot_json_round_trips() {
        for snapshot in [
            TrackedMasternodeSnapshot::default(),
            snapshot_full(),
            TrackedMasternodeSnapshot {
                list: Some(masternode(3)),
                ever_listed: true,
                ..Default::default()
            },
        ] {
            let encoded = snapshot_to_json(&snapshot);
            assert_eq!(snapshot_from_json(&encoded), snapshot, "{encoded}");
        }
    }

    #[test]
    fn unreadable_snapshot_json_degrades_to_empty() {
        assert_eq!(
            snapshot_from_json("not json"),
            TrackedMasternodeSnapshot::default()
        );
        assert_eq!(
            snapshot_from_json("{\"v\":999}"),
            TrackedMasternodeSnapshot::default()
        );
    }

    fn tracked() -> TrackedMasternode {
        TrackedMasternode {
            pro_tx_hash: [7u8; 32],
            label: Some("my node".to_string()),
            added_at: 1,
            snapshot: snapshot_full(),
        }
    }

    struct RegistryPersister {
        registry: Arc<TrackedMasternodeRegistry>,
        reject: AtomicBool,
        saw_exclusive_registry_lock: AtomicBool,
        writes: Mutex<Vec<Vec<TrackedMasternode>>>,
    }

    impl PlatformWalletPersistence for RegistryPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn persist_tracked_masternodes(
            &self,
            _network: Network,
            records: &[TrackedMasternode],
        ) -> Result<(), PersistenceError> {
            self.saw_exclusive_registry_lock
                .store(self.registry.rows.try_read().is_err(), Ordering::SeqCst);
            if self.reject.load(Ordering::SeqCst) {
                return Err(PersistenceError::backend("tracked write rejected"));
            }
            self.writes.lock().unwrap().push(records.to_vec());
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    fn registry_persister(
        initial: TrackedMasternodeMap,
    ) -> (Arc<TrackedMasternodeRegistry>, Arc<RegistryPersister>) {
        let registry = Arc::new(TrackedMasternodeRegistry {
            rows: RwLock::new(initial),
            gates: Default::default(),
        });
        let persister = Arc::new(RegistryPersister {
            registry: Arc::clone(&registry),
            reject: AtomicBool::new(false),
            saw_exclusive_registry_lock: AtomicBool::new(false),
            writes: Mutex::new(Vec::new()),
        });
        (registry, persister)
    }

    #[test]
    fn registry_mutation_remains_exclusive_until_whole_set_is_persisted() {
        let (registry, persister) = registry_persister(TrackedMasternodeMap::new());
        let row = tracked();

        mutate_registry_and_persist(
            registry.as_ref(),
            persister.as_ref(),
            Network::Testnet,
            |rows| {
                rows.insert(row.pro_tx_hash, row.clone());
                Ok(())
            },
        )
        .unwrap();

        assert!(persister.saw_exclusive_registry_lock.load(Ordering::SeqCst));
        assert_eq!(persister.writes.lock().unwrap().as_slice(), &[vec![row]]);
    }

    #[test]
    fn rejected_whole_set_write_restores_the_complete_registry() {
        let original = tracked();
        let mut initial = TrackedMasternodeMap::new();
        initial.insert(original.pro_tx_hash, original.clone());
        let (registry, persister) = registry_persister(initial.clone());
        persister.reject.store(true, Ordering::SeqCst);

        let error = mutate_registry_and_persist(
            registry.as_ref(),
            persister.as_ref(),
            Network::Testnet,
            |rows| {
                rows.get_mut(&original.pro_tx_hash).unwrap().label = Some("changed".to_string());
                rows.insert(
                    [8; 32],
                    TrackedMasternode {
                        pro_tx_hash: [8; 32],
                        label: None,
                        added_at: 2,
                        snapshot: TrackedMasternodeSnapshot::default(),
                    },
                );
                Ok(())
            },
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("failed to persist tracked masternodes"));
        assert_eq!(*registry.rows.read().unwrap(), initial);
    }

    /// Two refresh passes over one node, interleaved so the pass that
    /// learns nothing writes last. Its snapshot must not be the pre-read
    /// clone that predates what the other pass learned — in memory or in
    /// the persisted set.
    #[tokio::test]
    async fn interleaved_refresh_passes_keep_what_the_other_learned() {
        let hash = [7u8; 32];
        let mut initial = TrackedMasternodeMap::new();
        initial.insert(
            hash,
            TrackedMasternode {
                pro_tx_hash: hash,
                label: Some("my node".to_string()),
                added_at: 1,
                snapshot: TrackedMasternodeSnapshot::default(),
            },
        );
        let (registry, persister) = registry_persister(initial);

        let (learning_tx, learning_rx) = tokio::sync::oneshot::channel();
        let (release_learner_tx, release_learner_rx) = tokio::sync::oneshot::channel();
        let (release_blind_tx, release_blind_rx) = tokio::sync::oneshot::channel();

        // The pass that learns the registration, held mid-pass so the other
        // one can start while it is still in its network half.
        let learner = {
            let (registry, persister) = (Arc::clone(&registry), Arc::clone(&persister));
            tokio::spawn(async move {
                refresh_row_and_persist(
                    registry.as_ref(),
                    persister.as_ref(),
                    Network::Testnet,
                    &hash,
                    move |mut snapshot| async move {
                        learning_tx.send(()).expect("the test awaits this");
                        release_learner_rx.await.expect("the test releases this");
                        snapshot.registration = snapshot_full().registration;
                        (snapshot, RefreshOutcome::default())
                    },
                )
                .await
            })
        };
        learning_rx
            .await
            .expect("the learner starts its network half");

        // The pass whose network half comes back empty: it keeps a field by
        // not overwriting it, so it may only write a snapshot it read AFTER
        // the learner's.
        let blind = {
            let (registry, persister) = (Arc::clone(&registry), Arc::clone(&persister));
            tokio::spawn(async move {
                refresh_row_and_persist(
                    registry.as_ref(),
                    persister.as_ref(),
                    Network::Testnet,
                    &hash,
                    move |snapshot| async move {
                        release_blind_rx.await.expect("the test releases this");
                        (snapshot, RefreshOutcome::default())
                    },
                )
                .await
            })
        };
        // Let it get as far as it can while the learner still holds the row.
        tokio::task::yield_now().await;

        release_learner_tx.send(()).expect("the learner waits");
        learner.await.expect("learner task").expect("learner pass");
        release_blind_tx.send(()).expect("the blind pass waits");
        blind.await.expect("blind task").expect("blind pass");

        let live = registry
            .rows
            .read()
            .unwrap()
            .get(&hash)
            .expect("still tracked")
            .clone();
        assert_eq!(live.snapshot.registration, snapshot_full().registration);
        let persisted = persister
            .writes
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("both passes persisted the set");
        assert_eq!(persisted, vec![live]);
    }

    #[test]
    fn record_prefers_live_list_then_snapshot_then_registration() {
        let t = tracked();
        // Live entry present and valid ⇒ Active, live fields win.
        let mut live = evonode(7);
        live.platform_http_port = Some(1443);
        let record = t.record(Some(Some(&live)));
        assert_eq!(record.status, MasternodeStatus::Active);
        assert_eq!(record.platform_http_port, Some(1443));
        assert_eq!(record.source, MasternodeSource::Tracked);
        assert_eq!(record.label.as_deref(), Some("my node"));
        // Platform payout key wins over the registered script.
        assert_eq!(record.payout_script, Some(p2pkh_script(&[6u8; 20])));
        assert_eq!(record.owner_key_hash, Some([1u8; 20]));
        assert!(record.has_registration);
        assert_eq!(record.registration_height, 1000);

        // List available but node gone ⇒ Retired (snapshot fields still
        // render).
        let record = t.record(Some(None));
        assert_eq!(record.status, MasternodeStatus::Retired);
        assert!(record.service_address.is_some());

        // List unavailable ⇒ Unknown, never a fabricated Active.
        let record = t.record(None);
        assert_eq!(record.status, MasternodeStatus::Unknown);

        // PoSe-banned live entry ⇒ Inactive.
        let mut banned = evonode(7);
        banned.is_valid = false;
        assert_eq!(
            t.record(Some(Some(&banned))).status,
            MasternodeStatus::Inactive
        );
    }

    #[test]
    fn bare_tracked_row_models_everything_unknown() {
        let bare = TrackedMasternode {
            pro_tx_hash: [1u8; 32],
            label: None,
            added_at: 0,
            snapshot: TrackedMasternodeSnapshot::default(),
        };
        let record = bare.record(None);
        assert_eq!(record.status, MasternodeStatus::Unknown);
        assert!(!record.has_registration);
        assert_eq!(record.owner_key_hash, None);
        assert_eq!(record.payout_script, None);
        assert_eq!(record.service_address, None);
        let reference = bare.key_reference();
        assert_eq!(reference, MasternodeKeyReference::default());
    }

    #[test]
    fn key_reference_prefers_current_values() {
        let reference = tracked().key_reference();
        // Voting / operator / node id come from the (current) list entry of
        // `evonode(7)`, not the registration.
        assert_eq!(reference.voting_key_id, Some([7u8; 20]));
        assert_eq!(reference.operator_public_key, Some([7u8; 48]));
        assert_eq!(reference.platform_node_id, Some([7u8 ^ 0xFF; 20]));
        // Owner / payout come from Platform.
        assert_eq!(reference.owner_key_hash, Some([1u8; 20]));
        assert_eq!(reference.payout_key_hash, Some([6u8; 20]));
        assert_eq!(reference.operator_payout_key_hash, Some([7u8; 20]));
    }

    #[test]
    fn capabilities_follow_roles() {
        use MasternodeKeyRole::*;
        assert_eq!(
            capabilities_for_roles([]),
            MasternodeCapabilities::default()
        );
        assert!(capabilities_for_roles([Owner]).can_withdraw);
        assert!(capabilities_for_roles([OwnerPayout]).can_withdraw);
        assert!(!capabilities_for_roles([Voting]).can_withdraw);
        assert!(capabilities_for_roles([Voting]).can_vote);
        assert!(capabilities_for_roles([Operator]).can_update_service);
        assert!(capabilities_for_roles([PlatformNode]).identifies_platform_node);
        let all = capabilities_for_roles(MasternodeKeyRole::ALL);
        assert!(all.can_withdraw && all.can_vote && all.can_update_service);
    }

    #[test]
    fn numbering_is_per_type_in_order() {
        let t = tracked();
        let mut records = vec![
            {
                let mut r = t.record(None);
                r.is_evonode = false;
                r
            },
            t.record(None),
            t.record(None),
        ];
        number_records(&mut records);
        assert_eq!(records[0].order_index, 0);
        assert_eq!(records[0].type_index, 1, "Masternode 1");
        assert_eq!(records[1].type_index, 1, "Evonode 1");
        assert_eq!(records[2].type_index, 2, "Evonode 2");
    }

    #[test]
    fn registration_details_lift_from_a_proregtx() {
        // Reuse the aggregation fixture: rust-dashcore's testnet ProRegTx.
        let raw = "0300010001ca9a43051750da7c5f858008f2ff7732d15691e48eb7f845c791e5dca78bab58010000006b483045022100fe8fec0b3880bcac29614348887769b0b589908e3f5ec55a6cf478a6652e736502202f30430806a6690524e4dd599ba498e5ff100dea6a872ebb89c2fd651caa71ed012103d85b25d6886f0b3b8ce1eef63b720b518fad0b8e103eba4e85b6980bfdda2dfdffffffff018e37807e090000001976a9144ee1d4e5d61ac40a13b357ac6e368997079678c888ac00000000fd1201010000000000ca9a43051750da7c5f858008f2ff7732d15691e48eb7f845c791e5dca78bab580000000000000000000000000000ffff010205064e1f3dd03f9ec192b5f275a433bfc90f468ee1a3eb4c157b10706659e25eb362b5d902d809f9160b1688e201ee6e94b40f9b5062d7074683ef05a2d5efb7793c47059c878dfad38a30fafe61575db40f05ab0a08d55119b0aad300001976a9144fbc8fb6e11e253d77e5a9c987418e89cf4a63d288ac3477990b757387cb0406168c2720acf55f83603736a314a37d01b135b873a27b411fb37e49c1ff2b8057713939a5513e6e711a71cff2e517e6224df724ed750aef1b7f9ad9ec612b4a7250232e1e400da718a9501e1d9a5565526e4b1ff68c028763";
        let bytes = hex::decode(raw).unwrap();
        let tx: dashcore::Transaction = dashcore::consensus::encode::deserialize(&bytes).unwrap();
        let details = registration_from_transaction(&tx, 4242).expect("ProRegTx lifts");
        assert_eq!(details.height, 4242);
        assert_eq!(details.service_address.as_deref(), Some("1.2.5.6:19999"));
        assert!(!details.is_evonode);
        assert_eq!(details.platform_node_id, None);
        assert!(p2pkh_script_hash(&details.payout_script).is_some());
        // A non-provider tx lifts nothing.
        let plain = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        };
        assert_eq!(registration_from_transaction(&plain, 1), None);
    }
}
