//! Changeset types for delta-based wallet persistence.
//!
//! Every wallet mutation produces a [`PlatformWalletChangeSet`] delta that
//! is applied to in-memory state and persisted atomically. No full-state
//! snapshots — only deltas.
//!
//! # Shape
//!
//! `PlatformWalletChangeSet` embeds [`key_wallet::changeset::WalletChangeSet`]
//! verbatim in its `core` field — that sub-changeset carries every
//! core-wallet delta (chain, accounts, UTXOs, transactions, balance) in the
//! BDK-style per-account bucketing defined by key-wallet. Platform-specific
//! state that doesn't exist in key-wallet lives in dedicated sub-changesets:
//! identities, contacts, platform addresses, asset locks, and token balances.
//!
//! Earlier revisions of this file defined its own `ChainChangeSet`,
//! `TransactionChangeSet`, `UtxoChangeSet`, and `AccountChangeSet`. Those
//! were stand-ins from before key-wallet had its own changeset module and
//! used lossy flattened entries (e.g. `BTreeMap<OutPoint, u64>` for UTXOs,
//! losing address/script/is_coinbase/confirmation state). They are all
//! deleted; the `core` field replaces them with native `key-wallet` types.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::blockdata::transaction::{OutPoint, Transaction};

use dash_sdk::platform::address_sync::AddressFunds;
use dpp::prelude::AssetLockProof;
use key_wallet::PlatformP2PKHAddress;

use crate::wallet::platform_wallet::WalletId;

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::wallet::asset_lock::tracked::AssetLockStatus;

use crate::changeset::merge::Merge;
use crate::wallet::dashpay::{ContactRequest, DashPayProfile, EstablishedContact, PaymentEntry};
use crate::wallet::identity::managed_identity::{
    BlockTime, DpnsNameInfo, IdentityStatus, KeyStorage, ManagedIdentity,
};

// ---------------------------------------------------------------------------
// Bridge: key_wallet::changeset::WalletChangeSet -> platform-wallet Merge
// ---------------------------------------------------------------------------
//
// platform-wallet has its own `Merge` trait that is semantically
// richer than key-wallet's (recursive merge on `BTreeMap<K, V: Merge>`),
// so we can't just import key-wallet's trait wholesale. This one-off
// impl delegates to the key-wallet `Merge` implementation that ships
// with `WalletChangeSet` so that
// `Option<key_wallet::changeset::WalletChangeSet>` satisfies
// `crate::changeset::merge::Merge` via the blanket impl.
impl Merge for key_wallet::changeset::WalletChangeSet {
    fn merge(&mut self, other: Self) {
        <Self as key_wallet::changeset::Merge>::merge(self, other)
    }

    fn is_empty(&self) -> bool {
        <Self as key_wallet::changeset::Merge>::is_empty(self)
    }
}

// ---------------------------------------------------------------------------
// Identities
// ---------------------------------------------------------------------------

/// A full snapshot of a managed identity's state, keyed into
/// [`IdentityChangeSet`] by identity ID.
///
/// Mirrors every persistable field of
/// [`ManagedIdentity`](crate::wallet::identity::ManagedIdentity) except
/// contact state (which lives in [`ContactChangeSet`]) — mutation
/// methods call [`IdentityEntry::from_managed`] to produce a fresh
/// snapshot so the merge can resolve the latest state by last-write-wins.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityEntry {
    /// The Platform identity.
    pub identity: Identity,
    /// HD identity index used during registration.
    pub identity_index: u32,
    /// User-defined label.
    pub label: Option<String>,
    /// Last block time when balance was updated.
    pub last_updated_balance_block_time: Option<BlockTime>,
    /// Last block time when keys were synced.
    pub last_synced_keys_block_time: Option<BlockTime>,
    /// DPNS usernames with acquisition metadata.
    pub dpns_names: Vec<DpnsNameInfo>,
    /// Identity lifecycle status on Platform.
    pub status: IdentityStatus,
    /// Private key storage (public keys + private key data for each KeyID).
    pub key_storage: KeyStorage,
    /// Wallet identifier (`SHA256(root_pub_key || chain_code)`) of
    /// the wallet that owns this identity, if known.
    pub wallet_id: Option<[u8; 32]>,
    /// DashPay profile snapshot (display name, bio, avatar, public
    /// message). `None` until the profile has been fetched or set.
    pub dashpay_profile: Option<DashPayProfile>,
    /// DashPay payment history keyed by transaction id (hex string).
    /// Mutations that don't touch payments still carry the current
    /// map via `from_managed`, so merge can use plain extend semantics
    /// without losing history.
    pub dashpay_payments: BTreeMap<String, PaymentEntry>,
}

impl IdentityEntry {
    /// Capture a full snapshot of a [`ManagedIdentity`] as an entry.
    ///
    /// Every persistable field is copied into the entry so that
    /// [`IdentityChangeSet::merge`] can resolve the latest state via
    /// last-write-wins without needing partial-update diffing.
    pub fn from_managed(managed: &ManagedIdentity) -> Self {
        Self {
            identity: managed.identity.clone(),
            identity_index: managed.identity_index,
            label: managed.label.clone(),
            last_updated_balance_block_time: managed.last_updated_balance_block_time,
            last_synced_keys_block_time: managed.last_synced_keys_block_time,
            dpns_names: managed.dpns_names.clone(),
            status: managed.status,
            key_storage: managed.key_storage.clone(),
            wallet_id: managed.wallet_id,
            dashpay_profile: managed.dashpay_profile.clone(),
            dashpay_payments: managed.dashpay_payments.clone(),
        }
    }
}

/// Changes to the identity store.
///
/// Carries inserted/updated identities, tombstones for removals, and
/// wallet-level metadata mutated via [`IdentityManager`](crate::wallet::identity::IdentityManager)
/// (primary identity selection, gap-limit scan watermark).
///
/// # Merge ordering hazard
///
/// `IdentityChangeSet::merge` does NOT resolve `identities` vs
/// `removed` for the same key — both fields are extended
/// independently. Apply runs inserts before removes, so a merged
/// changeset that contains both an insert and a tombstone for the
/// same identity will end up "removed". Same hazard as
/// [`ContactChangeSet`]; same mitigation: every current emitter
/// produces only one of {insert, tombstone} per key per mutation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IdentityChangeSet {
    /// Inserted or updated identities keyed by identifier.
    pub identities: BTreeMap<Identifier, IdentityEntry>,
    /// Identities removed from the wallet.
    pub removed: BTreeSet<Identifier>,
    /// New primary identity selection. `None` means no change.
    pub primary_identity: Option<Identifier>,
    /// New gap-limit scan watermark. `None` means no change.
    pub last_scanned_index: Option<u32>,
}

impl Merge for IdentityChangeSet {
    fn merge(&mut self, other: Self) {
        // IdentityEntry is a full snapshot via `IdentityEntry::from_managed`,
        // so "later wins" is the correct policy for scalar fields. DPNS
        // names are merged as a union because each mutation method
        // produces a complete current snapshot but partial per-field
        // races across wallets are possible.
        for (id, entry) in other.identities {
            self.identities
                .entry(id)
                .and_modify(|existing| {
                    // Last write wins for the identity blob if the revision
                    // is at least the current one.
                    if entry.identity.revision() >= existing.identity.revision() {
                        existing.identity = entry.identity.clone();
                    }
                    existing.label = entry.label.clone();
                    existing.last_updated_balance_block_time =
                        entry.last_updated_balance_block_time;
                    existing.last_synced_keys_block_time = entry.last_synced_keys_block_time;
                    existing.status = entry.status;
                    // `wallet_id` is immutable per identity (SHA256 of
                    // root public key + chain code), so LWW and FWW are
                    // equivalent. We use LWW for consistency with the
                    // other scalars in this block.
                    existing.wallet_id = entry.wallet_id;
                    // DashPay profile: last-write-wins. Same policy as
                    // every other Option<T> scalar in this block —
                    // every mutation snapshot copies the current
                    // profile via `from_managed`, so LWW converges
                    // correctly within a single wallet.
                    existing.dashpay_profile = entry.dashpay_profile.clone();
                    // Append new DPNS names (by label).
                    for name in &entry.dpns_names {
                        if !existing.dpns_names.iter().any(|n| n.label == name.label) {
                            existing.dpns_names.push(name.clone());
                        }
                    }
                    // Merge key storage entries (last write wins per KeyID).
                    for (kid, slot) in &entry.key_storage {
                        existing.key_storage.insert(*kid, slot.clone());
                    }
                    // Merge DashPay payments (last-write-wins per tx_id).
                    // Every mutation snapshot copies the full map via
                    // `from_managed`, so extend converges within a
                    // single wallet.
                    for (tx_id, payment) in &entry.dashpay_payments {
                        existing
                            .dashpay_payments
                            .insert(tx_id.clone(), payment.clone());
                    }
                })
                .or_insert(entry);
        }
        self.removed.extend(other.removed);
        if other.primary_identity.is_some() {
            self.primary_identity = other.primary_identity;
        }
        // Monotonic merge for `last_scanned_index` — the gap-limit
        // scan watermark only advances forward. Defending against
        // stale replay / reordered flushes (holistic review S2):
        // even though the current writer is single, out-of-order
        // merging during staged-changeset accumulation or flush
        // failure recovery could clobber a newer value with an
        // older one. Use MAX for safety.
        match (self.last_scanned_index, other.last_scanned_index) {
            (None, Some(v)) => self.last_scanned_index = Some(v),
            (Some(current), Some(v)) if v > current => {
                self.last_scanned_index = Some(v);
            }
            _ => {}
        }
    }

    fn is_empty(&self) -> bool {
        self.identities.is_empty()
            && self.removed.is_empty()
            && self.primary_identity.is_none()
            && self.last_scanned_index.is_none()
    }
}

// ---------------------------------------------------------------------------
// Contacts
// ---------------------------------------------------------------------------

/// A single contact request entry in the changeset.
///
/// Modelled after [`crate::wallet::dashpay::ContactRequest`].
#[derive(Debug, Clone, PartialEq)]
pub struct ContactRequestEntry {
    /// The contact request.
    pub request: ContactRequest,
}

/// Key for sent contact requests: the **owner** sent a request TO the
/// **recipient**. Used for `sent_requests` and `removed_sent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SentContactRequestKey {
    /// The identity owned by this wallet (the sender).
    pub owner_id: Identifier,
    /// The identity we sent the request to.
    pub recipient_id: Identifier,
}

/// Key for incoming contact requests: the **owner** received a request
/// FROM the **sender**. Used for `incoming_requests` and
/// `removed_incoming`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReceivedContactRequestKey {
    /// The identity owned by this wallet (the recipient).
    pub owner_id: Identifier,
    /// The identity that sent the request to us.
    pub sender_id: Identifier,
}

/// Changes to the DashPay contact store.
///
/// All maps and sets key by `(owner_identity_id, contact_identity_id)` —
/// the first element is always the identity owned by this wallet. This
/// matches `ManagedIdentity`'s per-identity `sent_contact_requests` /
/// `incoming_contact_requests` / `established_contacts` layout and the
/// evo-tool DB shape, so `apply_changeset` can route each entry to the
/// correct `ManagedIdentity` without disambiguation logic.
///
/// # Auto-establishment contract
///
/// When a contact is added to `established`, the `apply` path MUST drop
/// any matching entries in `sent_contact_requests` and
/// `incoming_contact_requests` for the same `(owner, contact)` pair —
/// establishment implies the pending requests on both sides are
/// consumed. Mutation methods rely on this contract and do NOT also
/// emit `removed_sent` / `removed_incoming` tombstones for the
/// auto-establishment case; those sets are reserved for explicit
/// removals (e.g. `remove_sent_contact_request`).
///
/// `established` carries the full [`EstablishedContact`] (both
/// underlying [`ContactRequest`]s) rather than a bare `(owner, contact)`
/// pair, so `apply_changeset` can reconstruct the contact without
/// access to any prior runtime state.
///
/// # Merge ordering hazard
///
/// `ContactChangeSet::merge` is a pure `extend` over every field — it
/// does NOT cancel an insert against a same-key tombstone in the
/// opposing field. Callers must NOT merge a `removed_sent` for key K
/// followed by a `sent_requests` insert for key K and expect the
/// insert to win: apply runs inserts before removes, so the final
/// state is "removed", losing the intended re-send. The same applies
/// to `incoming_requests` vs `removed_incoming`.
///
/// In practice this is latent — every current emitter produces either
/// an insert XOR a tombstone for a given key in a single mutation,
/// not both. If a future caller needs the merged-cancellation
/// semantics, the merge impl should resolve `sent_requests ∩
/// removed_sent` by last-seen rather than carrying both.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContactChangeSet {
    /// Sent contact requests keyed by (owner → recipient).
    pub sent_requests: BTreeMap<SentContactRequestKey, ContactRequestEntry>,
    /// Sent requests explicitly removed (e.g. `remove_sent_contact_request`).
    pub removed_sent: BTreeSet<SentContactRequestKey>,
    /// Incoming contact requests keyed by (owner ← sender).
    pub incoming_requests: BTreeMap<ReceivedContactRequestKey, ContactRequestEntry>,
    /// Incoming requests explicitly removed (e.g. `remove_incoming_contact_request`).
    pub removed_incoming: BTreeSet<ReceivedContactRequestKey>,
    /// Newly established contacts keyed by (owner, contact). The full
    /// [`EstablishedContact`] is carried so the apply path can rebuild
    /// the relationship without reaching back into prior state. Uses
    /// [`SentContactRequestKey`] since from the owner's perspective the
    /// contact is the "recipient" of the relationship.
    pub established: BTreeMap<SentContactRequestKey, EstablishedContact>,
}

impl Merge for ContactChangeSet {
    fn merge(&mut self, other: Self) {
        self.sent_requests.extend(other.sent_requests);
        self.removed_sent.extend(other.removed_sent);
        self.incoming_requests.extend(other.incoming_requests);
        self.removed_incoming.extend(other.removed_incoming);
        self.established.extend(other.established);
    }

    fn is_empty(&self) -> bool {
        self.sent_requests.is_empty()
            && self.removed_sent.is_empty()
            && self.incoming_requests.is_empty()
            && self.removed_incoming.is_empty()
            && self.established.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Platform Addresses
// ---------------------------------------------------------------------------

/// Changes to the Platform address store.
///
/// A map from [`PlatformAddress`] (P2PKH or P2SH) to the full
/// [`AddressFunds`] snapshot (balance in credits + anti-replay nonce).
/// Last-write-wins on merge.
///
/// Addresses are never removed — they are deterministically derived
/// from the HD seed. A drained address simply has balance 0 (nonce
/// preserved so subsequent top-ups don't replay).
///
/// Also carries the incremental-sync watermark (`sync_height` /
/// `sync_timestamp`). Without it the provider would start fresh after
/// every restart and force a full rescan instead of a delta catch-up.
/// The watermark is tracked per wallet (not per account): it's the max
/// across accounts — on the next sync every provider rewinds to this
/// point, so no account can silently skip a range.
/// One updated platform payment address inside a
/// [`PlatformAddressChangeSet`]. Carries full routing context —
/// wallet id + DIP-17 account index + derivation index + P2PKH — so
/// persisters can apply the entry without guessing which account or
/// HD slot it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformAddressBalanceEntry {
    pub wallet_id: WalletId,
    pub account_index: u32,
    pub address_index: u32,
    pub address: PlatformP2PKHAddress,
    pub funds: AddressFunds,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlatformAddressChangeSet {
    /// Updated platform addresses produced by the last sync pass.
    /// A `Vec` rather than a map because the diff already deduplicates
    /// per `(wallet, account, address)` within one changeset, and
    /// consumers either apply each entry independently or merge
    /// append-only.
    pub addresses: Vec<PlatformAddressBalanceEntry>,
    /// Highest block height covered by the last sync, across all
    /// accounts. `None` means "no change".
    pub sync_height: Option<u64>,
    /// Latest timestamp covered by the last sync, across all accounts.
    /// `None` means "no change".
    pub sync_timestamp: Option<u64>,
    /// Last block height with recent address changes (compaction marker).
    /// `None` means "no change".
    pub last_known_recent_block: Option<u64>,
}

impl Merge for PlatformAddressChangeSet {
    fn merge(&mut self, other: Self) {
        // Append-only merge — no dedup, last entry wins at apply
        // time. Duplicates across changesets are rare (each sync's
        // diff covers a different point in time); paying the cost of
        // hashing/sorting to dedup here isn't worthwhile.
        self.addresses.extend(other.addresses);
        // Monotonic-max merge — a later sync can only advance the
        // watermark, never roll it back. `None` means "no update in
        // this changeset".
        if let Some(h) = other.sync_height {
            self.sync_height = Some(self.sync_height.map_or(h, |existing| existing.max(h)));
        }
        if let Some(t) = other.sync_timestamp {
            self.sync_timestamp = Some(self.sync_timestamp.map_or(t, |existing| existing.max(t)));
        }
        if let Some(r) = other.last_known_recent_block {
            self.last_known_recent_block = Some(
                self.last_known_recent_block
                    .map_or(r, |existing| existing.max(r)),
            );
        }
    }

    fn is_empty(&self) -> bool {
        self.addresses.is_empty()
            && self.sync_height.is_none()
            && self.sync_timestamp.is_none()
            && self.last_known_recent_block.is_none()
    }
}

// ---------------------------------------------------------------------------
// Asset Locks
// ---------------------------------------------------------------------------

/// Changes to the asset lock store.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssetLockChangeSet {
    /// Asset lock entries keyed by outpoint (txid + output index).
    ///
    /// Each credit output in an asset lock transaction is tracked
    /// independently because a single transaction can have up to 255
    /// credit outputs (DIP-0027), each consumable separately.
    pub asset_locks: BTreeMap<OutPoint, AssetLockEntry>,
    /// Asset locks removed (consumed by identity registration / top-up).
    pub removed: BTreeSet<OutPoint>,
}

/// A single asset lock entry in the changeset.
///
/// Contains all fields needed to fully reconstruct a
/// [`TrackedAssetLock`](crate::wallet::asset_lock::tracked::TrackedAssetLock).
#[derive(Debug, Clone, PartialEq)]
pub struct AssetLockEntry {
    /// The outpoint identifying this credit output (txid + vout).
    pub out_point: OutPoint,
    /// The full asset lock transaction.
    pub transaction: Transaction,
    /// BIP44 account index that funded this asset lock (UTXO source).
    pub account_index: u32,
    /// Which funding account to derive the one-time key from.
    pub funding_type: AssetLockFundingType,
    /// Identity index used during creation.
    pub identity_index: u32,
    /// The amount locked (in duffs).
    pub amount_duffs: u64,
    /// Current status on Core chain.
    pub status: AssetLockStatus,
    /// The asset lock proof, available once IS-locked or ChainLocked.
    pub proof: Option<AssetLockProof>,
}

impl Merge for AssetLockChangeSet {
    fn merge(&mut self, other: Self) {
        // Last write wins — later status is higher finality.
        self.asset_locks.extend(other.asset_locks);
        self.removed.extend(other.removed);
    }

    fn is_empty(&self) -> bool {
        self.asset_locks.is_empty() && self.removed.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Token Balances
// ---------------------------------------------------------------------------

/// Changes to watched Platform token balances.
///
/// Mirrors `PlatformWalletInfo.token_balances`
/// (`BTreeMap<(Identifier, Identifier), TokenAmount>`) and
/// `PlatformWalletInfo.token_watched`
/// (`BTreeMap<Identifier, BTreeSet<Identifier>>`), plus tombstones for
/// entries removed by `unwatch` / `unwatch_identity`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TokenBalanceChangeSet {
    /// Updated token balances keyed by `(identity_id, token_id)`.
    /// Last write wins on merge.
    pub balances: BTreeMap<(Identifier, Identifier), u64>,

    /// Balances removed (`unwatch` / `unwatch_identity` / sync returned `None`).
    pub removed_balances: BTreeSet<(Identifier, Identifier)>,

    /// Tokens newly watched per identity.
    /// Merged via set union on the inner `BTreeSet`.
    pub watched: BTreeMap<Identifier, BTreeSet<Identifier>>,

    /// Tokens unwatched per identity.
    /// Merged via set union on the inner `BTreeSet`.
    pub unwatched: BTreeMap<Identifier, BTreeSet<Identifier>>,
}

impl Merge for TokenBalanceChangeSet {
    fn merge(&mut self, other: Self) {
        self.balances.extend(other.balances);
        self.removed_balances.extend(other.removed_balances);
        for (identity_id, tokens) in other.watched {
            self.watched.entry(identity_id).or_default().extend(tokens);
        }
        for (identity_id, tokens) in other.unwatched {
            self.unwatched
                .entry(identity_id)
                .or_default()
                .extend(tokens);
        }
    }

    fn is_empty(&self) -> bool {
        self.balances.is_empty()
            && self.removed_balances.is_empty()
            && self.watched.is_empty()
            && self.unwatched.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Top-Level PlatformWalletChangeSet
// ---------------------------------------------------------------------------

/// Delta of all wallet state changes from a single operation.
///
/// `core` carries the full `key_wallet::changeset::WalletChangeSet` — chain,
/// balance, account_keys, and per-account buckets (UTXOs, transactions,
/// addresses used, highest-used index). Platform-specific deltas (identities,
/// contacts, platform addresses, asset locks, token balances) live in
/// dedicated sub-changesets.
///
/// Composed of optional sub-changesets — `None` means no change in that
/// area. Use [`Merge::merge`] to combine multiple deltas before persisting.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlatformWalletChangeSet {
    /// Core wallet state from key-wallet: chain, balance, account keys,
    /// and per-account buckets (UTXOs, transactions, addresses used,
    /// highest-used index).
    pub core: Option<key_wallet::changeset::WalletChangeSet>,
    /// Identity changes (registered, updated).
    pub identities: Option<IdentityChangeSet>,
    /// DashPay contact changes (requests sent/received, established).
    pub contacts: Option<ContactChangeSet>,
    /// Platform address balance/nonce changes.
    pub platform_addresses: Option<PlatformAddressChangeSet>,
    /// Asset lock lifecycle changes (created, locked, used).
    pub asset_locks: Option<AssetLockChangeSet>,
    /// Platform token balance / watch changes.
    pub token_balances: Option<TokenBalanceChangeSet>,
    /// DashPay profile overlays keyed by identity ID. Applied AFTER
    /// `identities` — updates the `dashpay_profile` field on existing
    /// `ManagedIdentity` entries without touching other fields. Used
    /// by the persister load path where only DashPay data is available
    /// (the identity blob lives in the external `identity` table).
    /// Identities not in the wallet are silently skipped.
    pub dashpay_profiles: Option<BTreeMap<Identifier, Option<DashPayProfile>>>,
    /// DashPay payment history overlays keyed by identity ID. Same
    /// semantics as `dashpay_profiles` — extends existing payment maps
    /// via `BTreeMap::extend` (last-write-wins per tx_id).
    pub dashpay_payments_overlay: Option<BTreeMap<Identifier, BTreeMap<String, PaymentEntry>>>,
}

impl From<PlatformAddressChangeSet> for PlatformWalletChangeSet {
    fn from(cs: PlatformAddressChangeSet) -> Self {
        Self {
            platform_addresses: Some(cs),
            ..Default::default()
        }
    }
}

impl From<IdentityChangeSet> for PlatformWalletChangeSet {
    fn from(cs: IdentityChangeSet) -> Self {
        Self {
            identities: Some(cs),
            ..Default::default()
        }
    }
}

impl From<ContactChangeSet> for PlatformWalletChangeSet {
    fn from(cs: ContactChangeSet) -> Self {
        Self {
            contacts: Some(cs),
            ..Default::default()
        }
    }
}

impl From<AssetLockChangeSet> for PlatformWalletChangeSet {
    fn from(cs: AssetLockChangeSet) -> Self {
        Self {
            asset_locks: Some(cs),
            ..Default::default()
        }
    }
}

impl From<TokenBalanceChangeSet> for PlatformWalletChangeSet {
    fn from(cs: TokenBalanceChangeSet) -> Self {
        Self {
            token_balances: Some(cs),
            ..Default::default()
        }
    }
}

impl Merge for PlatformWalletChangeSet {
    fn merge(&mut self, other: Self) {
        // `key_wallet::changeset::WalletChangeSet` implements `Merge`
        // itself; delegate via the `Option<T>: Merge` blanket impl from
        // this crate's merge module.
        self.core.merge(other.core);
        self.identities.merge(other.identities);
        self.contacts.merge(other.contacts);
        self.platform_addresses.merge(other.platform_addresses);
        self.asset_locks.merge(other.asset_locks);
        self.token_balances.merge(other.token_balances);
        // DashPay overlays: LWW per identity_id.
        if let Some(other_profiles) = other.dashpay_profiles {
            self.dashpay_profiles
                .get_or_insert_with(Default::default)
                .extend(other_profiles);
        }
        if let Some(other_payments) = other.dashpay_payments_overlay {
            let target = self
                .dashpay_payments_overlay
                .get_or_insert_with(Default::default);
            for (id, payments) in other_payments {
                target.entry(id).or_default().extend(payments);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.core.is_empty()
            && self.identities.is_empty()
            && self.contacts.is_empty()
            && self.platform_addresses.is_empty()
            && self.asset_locks.is_empty()
            && self.token_balances.is_empty()
            && self
                .dashpay_profiles
                .as_ref()
                .map_or(true, |m| m.is_empty())
            && self
                .dashpay_payments_overlay
                .as_ref()
                .map_or(true, |m| m.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_changeset() {
        let cs = PlatformWalletChangeSet::default();
        assert!(cs.is_empty());
    }

    #[test]
    fn test_platform_address_changeset_merge() {
        let wallet_id: WalletId = [9u8; 32];
        let addr1 = PlatformP2PKHAddress::new([1u8; 20]);
        let addr2 = PlatformP2PKHAddress::new([2u8; 20]);

        let funds = |balance, nonce| AddressFunds { balance, nonce };
        let entry = |address_index, address, funds| PlatformAddressBalanceEntry {
            wallet_id,
            account_index: 0,
            address_index,
            address,
            funds,
        };

        let mut a = PlatformAddressChangeSet::default();
        a.addresses.push(entry(0, addr1, funds(100, 1)));

        let mut b = PlatformAddressChangeSet::default();
        b.addresses.push(entry(0, addr1, funds(200, 2)));
        b.addresses.push(entry(1, addr2, funds(50, 3)));

        a.merge(b);
        // Append-only: three entries total; apply-time "last wins" is
        // what gives `addr1 → funds(200, 2)` on replay.
        assert_eq!(a.addresses.len(), 3);
        assert_eq!(a.addresses[0], entry(0, addr1, funds(100, 1)));
        assert_eq!(a.addresses[1], entry(0, addr1, funds(200, 2)));
        assert_eq!(a.addresses[2], entry(1, addr2, funds(50, 3)));
    }

    #[test]
    fn test_token_balance_changeset_merge() {
        let identity_a = Identifier::from([1u8; 32]);
        let identity_b = Identifier::from([2u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let token_y = Identifier::from([11u8; 32]);

        let mut a = TokenBalanceChangeSet::default();
        a.balances.insert((identity_a, token_x), 100);
        a.watched.entry(identity_a).or_default().insert(token_x);

        let mut b = TokenBalanceChangeSet::default();
        // Same identity/token — last-write-wins.
        b.balances.insert((identity_a, token_x), 200);
        // New token on same identity — merged into the watched set.
        b.watched.entry(identity_a).or_default().insert(token_y);
        // New identity.
        b.balances.insert((identity_b, token_x), 50);

        a.merge(b);

        assert_eq!(a.balances.get(&(identity_a, token_x)), Some(&200));
        assert_eq!(a.balances.get(&(identity_b, token_x)), Some(&50));
        let watched_a = a.watched.get(&identity_a).unwrap();
        assert!(watched_a.contains(&token_x));
        assert!(watched_a.contains(&token_y));
    }

    #[test]
    fn test_take_empty_changeset() {
        let mut cs = PlatformWalletChangeSet::default();
        assert!(cs.take().is_none());
    }

    #[test]
    fn test_take_non_empty_changeset() {
        let mut cs = PlatformWalletChangeSet {
            asset_locks: Some(AssetLockChangeSet {
                asset_locks: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        OutPoint::default(),
                        AssetLockEntry {
                            out_point: OutPoint::default(),
                            transaction: Transaction {
                                version: 3,
                                lock_time: 0,
                                input: vec![],
                                output: vec![],
                                special_transaction_payload: None,
                            },
                            account_index: 0,
                            funding_type: AssetLockFundingType::IdentityRegistration,
                            identity_index: 0,
                            amount_duffs: 1000,
                            status: AssetLockStatus::Built,
                            proof: None,
                        },
                    );
                    m
                },
                removed: Default::default(),
            }),
            ..Default::default()
        };
        let taken = cs.take();
        assert!(taken.is_some());
        assert!(cs.is_empty());
    }
}
