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

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AssetLockProof;

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::wallet::asset_lock::tracked::AssetLockStatus;

use crate::changeset::merge::Merge;
use crate::wallet::dashpay::{
    ContactRequest, DashPayProfile, EstablishedContact, PaymentEntry,
};
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
    /// Top-up history: maps top-up index to amount (in duffs).
    pub top_ups: BTreeMap<u32, u64>,
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
            top_ups: managed.top_ups.clone(),
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
        // names and top-ups are merged as unions because each mutation
        // method produces a complete current snapshot but partial per-field
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
                    // Merge top-ups (last write wins per index).
                    existing.top_ups.extend(entry.top_ups.iter());
                    // Merge key storage entries (last write wins per KeyID).
                    for (kid, slot) in &entry.key_storage {
                        existing.key_storage.insert(*kid, slot.clone());
                    }
                    // Merge DashPay payments (last-write-wins per tx_id).
                    // Same reasoning as top_ups: every mutation snapshot
                    // copies the full map via `from_managed`, so extend
                    // converges within a single wallet.
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
    /// Sent contact requests keyed by (owner, recipient).
    pub sent_requests: BTreeMap<(Identifier, Identifier), ContactRequestEntry>,
    /// Sent requests explicitly removed (e.g. `remove_sent_contact_request`).
    pub removed_sent: BTreeSet<(Identifier, Identifier)>,
    /// Incoming contact requests keyed by (owner, sender).
    pub incoming_requests: BTreeMap<(Identifier, Identifier), ContactRequestEntry>,
    /// Incoming requests explicitly removed (e.g. `remove_incoming_contact_request`).
    pub removed_incoming: BTreeSet<(Identifier, Identifier)>,
    /// Newly established contacts keyed by `(owner, contact)`. The full
    /// [`EstablishedContact`] is carried so the apply path can rebuild
    /// the relationship without reaching back into prior state.
    pub established: BTreeMap<(Identifier, Identifier), EstablishedContact>,
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
/// Mirrors [`PlatformWalletInfo.platform_address_balances`] exactly:
/// a map from [`PlatformAddress`] (P2PKH or P2SH) to [`Credits`]
/// (the balance in duffs). Plus a tombstone set for addresses whose
/// balance dropped to zero / address was spent out during a transfer
/// or withdrawal. Last-write-wins on merge.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlatformAddressChangeSet {
    /// Updated platform addresses keyed by `PlatformAddress`.
    pub addresses: BTreeMap<PlatformAddress, Credits>,
    /// Addresses removed from the cache (e.g. drained by a transfer/withdraw).
    pub removed: BTreeSet<PlatformAddress>,
}

impl Merge for PlatformAddressChangeSet {
    fn merge(&mut self, other: Self) {
        // Last write wins — the latest balance is the most current.
        self.addresses.extend(other.addresses);
        self.removed.extend(other.removed);
    }

    fn is_empty(&self) -> bool {
        self.addresses.is_empty() && self.removed.is_empty()
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
            self.unwatched.entry(identity_id).or_default().extend(tokens);
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
    }

    fn is_empty(&self) -> bool {
        self.core.is_empty()
            && self.identities.is_empty()
            && self.contacts.is_empty()
            && self.platform_addresses.is_empty()
            && self.asset_locks.is_empty()
            && self.token_balances.is_empty()
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
        let addr1 = PlatformAddress::P2pkh([1u8; 20]);
        let addr2 = PlatformAddress::P2pkh([2u8; 20]);

        let mut a = PlatformAddressChangeSet::default();
        a.addresses.insert(addr1.clone(), 100);

        let mut b = PlatformAddressChangeSet::default();
        b.addresses.insert(addr1.clone(), 200); // last write wins
        b.addresses.insert(addr2.clone(), 50);

        a.merge(b);
        assert_eq!(a.addresses.get(&addr1), Some(&200));
        assert_eq!(a.addresses.get(&addr2), Some(&50));
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
            identities: Some(IdentityChangeSet::default()),
            ..Default::default()
        };
        // The identities changeset is an empty placeholder — so cs is
        // still `is_empty()`. Push a real entry.
        let identity_id = Identifier::from([1u8; 32]);
        // We can't easily construct a full Identity here, so use asset_locks
        // for the non-empty path.
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
        let _ = identity_id;
        let taken = cs.take();
        assert!(taken.is_some());
        assert!(cs.is_empty());
    }
}
