//! Changeset types for delta-based wallet persistence.
//!
//! Every wallet mutation produces a [`PlatformWalletChangeSet`] delta that
//! is applied to in-memory state and persisted atomically. No full-state
//! snapshots — only deltas.
//!
//! # Shape
//!
//! `PlatformWalletChangeSet` carries a [`CoreChangeSet`] in its `core`
//! field — a platform-owned projection of the data that upstream's
//! `WalletEvent` bus delivers (transaction records + UTXO deltas + heights
//! + InstantSend locks). Platform-specific state that doesn't exist in
//! key-wallet lives in dedicated sub-changesets: identities, contacts,
//! platform addresses, asset locks, and token balances.
//!
//! Earlier revisions of this file used `key_wallet::changeset::WalletChangeSet`
//! verbatim in the `core` field. That upstream type was deleted in favour
//! of an event-bus model (see PR #696 in rust-dashcore). Platform-wallet
//! subscribes to the event bus, projects each event into a `CoreChangeSet`,
//! and routes it through this changeset's `core` slot — keeping the
//! per-domain merge / apply shape downstream consumers already know.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::blockdata::transaction::{OutPoint, Transaction};
use dashcore::ephemerealdata::instant_lock::InstantLock;
use dashcore::Txid;

use dash_sdk::platform::address_sync::AddressFunds;
use dpp::prelude::AssetLockProof;
use key_wallet::account::AccountType;
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::{AddressInfo, Network, PlatformP2PKHAddress, Utxo};

use crate::wallet::platform_wallet::WalletId;

use dpp::balances::credits::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::{Identifier, Revision};

use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::wallet::asset_lock::tracked::AssetLockStatus;

use crate::changeset::merge::Merge;
use crate::wallet::identity::state::managed_identity::{
    BlockTime, DpnsNameInfo, IdentityStatus, ManagedIdentity,
};
use crate::wallet::identity::{ContactRequest, DashPayProfile, EstablishedContact, PaymentEntry};

// ---------------------------------------------------------------------------
// Core wallet changeset — projection of upstream `WalletEvent` data
// ---------------------------------------------------------------------------

/// Platform-owned projection of the core-wallet deltas that upstream's
/// `WalletEvent` bus delivers.
///
/// Built by the platform-wallet event adapter from `WalletEvent` variants
/// emitted by `WalletManager`. Every field is purely additive — the
/// merge implementation uses last-write-wins for the height watermarks
/// (monotonic-max), `extend` for the records / utxos vecs, and
/// last-write-wins for the IS-lock map.
///
/// # Why a projection instead of the upstream type
///
/// Upstream `key_wallet::changeset::WalletChangeSet` was deleted in favour
/// of `WalletEvent`. Forking that deleted type would re-introduce the
/// merge-ordering hazards the upstream PR removed. This projection
/// captures exactly what the persister needs (records to write, UTXOs to
/// add/remove, height checkpoints, IS-lock updates) without inheriting
/// the merge complexity of the deleted upstream type.
///
/// Not `PartialEq` — `TransactionRecord` upstream is `Debug + Clone` only,
/// so structural equality on `records` would require us to fork the
/// upstream type. Tests that need to inspect a changeset's contents
/// reach into individual fields directly.
#[derive(Debug, Clone, Default)]
pub struct CoreChangeSet {
    /// Transaction records produced by this batch.
    ///
    /// Includes records first stored (`TransactionDetected`,
    /// `BlockProcessed.inserted`), records whose context advanced
    /// (`BlockProcessed.updated` — e.g. a mempool tx that just confirmed),
    /// and coinbase records that crossed the maturity threshold
    /// (`BlockProcessed.matured`). All persisted; the persister's
    /// `txid` uniqueness constraint handles dedup on replay.
    pub records: Vec<TransactionRecord>,

    /// UTXOs to remove — outpoints that records in this batch spent.
    /// The full `Utxo` is carried (not just `OutPoint`) so a persister
    /// audit trail / spent-output history can keep the original metadata
    /// without a follow-up read.
    pub spent_utxos: Vec<Utxo>,

    /// UTXOs to add — outputs created by records in this batch that pay
    /// to one of our addresses (i.e. `OutputRole::Received` or
    /// `OutputRole::Change` per the upstream `TransactionRecord`).
    pub new_utxos: Vec<Utxo>,

    /// InstantSend locks observed for records that are NOT yet in a
    /// chain-locked block (i.e. records still in `Mempool`,
    /// `InstantSend`, or `InBlock` context — anything `InChainLockedBlock`
    /// is excluded since chain-lock finality already supersedes IS).
    ///
    /// Populated from `WalletEvent::TransactionInstantLocked`. The
    /// persister applies these by looking up the matching record and
    /// updating its `context` to `TransactionContext::InstantSend(..)`.
    /// Chain-locked records skip this map entirely — by the time a
    /// transaction is chain-locked it's final, and IS-lock state is
    /// no longer informative.
    pub instant_locks_for_non_final_records: BTreeMap<Txid, InstantLock>,

    /// From `WalletEvent::BlockProcessed.height` — advance the wallet's
    /// `last_processed_height` to this value. Monotonic-max on merge.
    pub last_processed_height: Option<u32>,

    /// From `WalletEvent::SyncHeightAdvanced.height` — advance the
    /// durable filter-batch sync checkpoint to this value. Monotonic-max
    /// on merge.
    pub synced_height: Option<u32>,
}

impl Merge for CoreChangeSet {
    fn merge(&mut self, other: Self) {
        // Records / utxo deltas: append-only. The event adapter never
        // produces duplicates within a single batch (each event covers
        // a distinct moment); cross-batch dedup is the persister's
        // responsibility (txid uniqueness for records, outpoint
        // uniqueness for utxos).
        self.records.extend(other.records);
        self.spent_utxos.extend(other.spent_utxos);
        self.new_utxos.extend(other.new_utxos);

        // IS-lock map: last-write-wins per txid. A second IS-lock for
        // the same txid (e.g. a follow-up event re-confirming the lock)
        // overwrites — the lock object itself is canonical.
        self.instant_locks_for_non_final_records
            .extend(other.instant_locks_for_non_final_records);

        // Height watermarks: monotonic-max. A later changeset can only
        // advance the watermark, never roll it back. `None` means
        // "no update in this batch".
        if let Some(h) = other.last_processed_height {
            self.last_processed_height = Some(
                self.last_processed_height
                    .map_or(h, |existing| existing.max(h)),
            );
        }
        if let Some(h) = other.synced_height {
            self.synced_height = Some(self.synced_height.map_or(h, |existing| existing.max(h)));
        }
    }

    fn is_empty(&self) -> bool {
        self.records.is_empty()
            && self.spent_utxos.is_empty()
            && self.new_utxos.is_empty()
            && self.instant_locks_for_non_final_records.is_empty()
            && self.last_processed_height.is_none()
            && self.synced_height.is_none()
    }
}

// ---------------------------------------------------------------------------
// Identities
// ---------------------------------------------------------------------------

/// A scalar-only snapshot of a managed identity's state, keyed into
/// [`IdentityChangeSet`] by identity ID.
///
/// Carries the per-identity scalars (id / balance / revision + wallet
/// metadata) but NOT the DPP `public_keys` map or the private
/// `KeyStorage`. Keys live in the sibling [`IdentityKeysChangeSet`]
/// keyed by `(identity_id, key_id)` so that a simple scalar mutation
/// (e.g. a balance refresh) serializes only the scalar fields without
/// re-serializing every public-key byte and private-key data blob.
///
/// Mirrors every persistable scalar field of
/// [`ManagedIdentity`](crate::wallet::identity::ManagedIdentity) except
/// contact state (which lives in [`ContactChangeSet`]) and identity
/// keys (which live in [`IdentityKeysChangeSet`]) — mutation methods
/// call [`IdentityEntry::from_managed`] to produce a fresh scalar
/// snapshot so the merge can resolve the latest state by last-write-wins.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityEntry {
    /// Identity identifier.
    pub id: Identifier,
    /// Identity balance (credits).
    pub balance: Credits,
    /// On-chain identity revision; used as the monotonic gate for
    /// `balance`/`revision` replacement in merge + apply.
    pub revision: Revision,
    /// HD identity index used during registration. `Some(idx)` for
    /// wallet-owned identities (matches the bucket key in
    /// `IdentityManager.wallet_identities[wallet_id][idx]`); `None` for
    /// out-of-wallet (observed) identities, which have no derivation
    /// context. Mirrors the shape of
    /// [`ManagedIdentity::identity_index`](crate::wallet::identity::ManagedIdentity).
    pub identity_index: Option<u32>,
    /// Last block time when balance was updated.
    pub last_updated_balance_block_time: Option<BlockTime>,
    /// Last block time when keys were synced.
    pub last_synced_keys_block_time: Option<BlockTime>,
    /// DPNS usernames with acquisition metadata.
    pub dpns_names: Vec<DpnsNameInfo>,
    /// DPNS labels this identity is currently contending for —
    /// mirrored from `ManagedIdentity.contested_dpns_names`.
    /// Contest metadata (contenders, votes, end time) isn't cached
    /// here; the UI queries it on demand against Platform.
    pub contested_dpns_names: Vec<String>,
    /// Identity lifecycle status on Platform.
    pub status: IdentityStatus,
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
    /// Capture a scalar-only snapshot of a [`ManagedIdentity`] as an entry.
    ///
    /// Only the scalars copied by this method are carried; per-identity
    /// public keys and private-key storage are captured separately via
    /// [`ManagedIdentity::keys_snapshot_changeset`](crate::wallet::identity::ManagedIdentity)
    /// into an [`IdentityKeysChangeSet`].
    pub fn from_managed(managed: &ManagedIdentity) -> Self {
        Self {
            id: managed.identity.id(),
            balance: managed.identity.balance(),
            revision: managed.identity.revision(),
            identity_index: managed.identity_index,
            last_updated_balance_block_time: managed.last_updated_balance_block_time,
            last_synced_keys_block_time: managed.last_synced_keys_block_time,
            dpns_names: managed.dpns_names.clone(),
            contested_dpns_names: managed.contested_dpns_names.clone(),
            status: managed.status,
            wallet_id: managed.wallet_id,
            dashpay_profile: managed.dashpay_profile.clone(),
            dashpay_payments: managed.dashpay_payments.clone(),
        }
    }
}

/// DIP-9 derivation coordinates for an identity key.
///
/// Paired with `wallet_id` on [`IdentityKeyEntry`], this is
/// everything a client needs to reproduce the signing private key
/// from the wallet's mnemonic at the DIP-9 identity authentication
/// path — platform-wallet itself never carries or persists the key
/// bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentityKeyDerivationIndices {
    /// DIP-9 identity index (hardened).
    pub identity_index: u32,
    /// DIP-9 key index within the identity (hardened).
    pub key_index: u32,
}

/// A single identity-key entry in an [`IdentityKeysChangeSet`].
///
/// Platform-wallet only carries the DPP public-key record and a
/// breadcrumb pointing at the wallet derivation that produced it;
/// private-key bytes live exclusively on the client side (iOS
/// Keychain, Android Keystore, etc.), populated by the client
/// deriving locally from the owning wallet's mnemonic. When
/// `wallet_id` + `derivation_indices` are both set, the client
/// should re-derive the 32-byte scalar at
/// `m/9'/coin'/5'/0'/ECDSA'/identity_index'/key_index'` and
/// persist it. When either is `None` the key is watch-only from
/// this wallet's point of view.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityKeyEntry {
    /// Owning identity.
    pub identity_id: Identifier,
    /// Key id inside that identity's public-key map.
    pub key_id: KeyID,
    /// The DPP public-key record.
    pub public_key: IdentityPublicKey,
    /// 20-byte RIPEMD160(SHA256) of the compressed public key.
    /// Precomputed on the Rust side so clients don't need
    /// RIPEMD-160 implementations of their own.
    pub public_key_hash: [u8; 20],
    /// The wallet whose mnemonic derives this key. `None` for
    /// watched-only identities with no wallet association.
    pub wallet_id: Option<[u8; 32]>,
    /// DIP-9 `(identity_index, key_index)` pair the client needs to
    /// re-derive the private key. `None` means "view-only, no
    /// private key recoverable".
    pub derivation_indices: Option<IdentityKeyDerivationIndices>,
}

/// Changes to per-identity key storage.
///
/// Keyed by `(identity_id, key_id)`. Entries in `upserts` are
/// applied before entries in `removed`, matching the other changeset
/// conventions in this file. Emitters should produce only one of
/// `{upsert, remove}` per key per mutation — the merge does not resolve
/// insert-vs-tombstone for the same key.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IdentityKeysChangeSet {
    /// Inserted or updated identity keys keyed by (identity_id, key_id).
    pub upserts: BTreeMap<(Identifier, KeyID), IdentityKeyEntry>,
    /// Identity keys explicitly removed, keyed by (identity_id, key_id).
    pub removed: BTreeSet<(Identifier, KeyID)>,
}

impl Merge for IdentityKeysChangeSet {
    fn merge(&mut self, other: Self) {
        // Last-write-wins per composite key. Matches the discipline
        // used by other scalar-entry changesets in this module; the
        // "don't emit both upsert + remove for the same key in one
        // mutation" invariant is enforced on the emitter side.
        self.upserts.extend(other.upserts);
        self.removed.extend(other.removed);
    }

    fn is_empty(&self) -> bool {
        self.upserts.is_empty() && self.removed.is_empty()
    }
}

/// Changes to the identity store.
///
/// Carries inserted/updated identities and tombstones for removals.
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
}

impl Merge for IdentityChangeSet {
    fn merge(&mut self, other: Self) {
        // IdentityEntry is a scalar-only snapshot via
        // `IdentityEntry::from_managed`; public keys / private keys
        // live in the sibling `IdentityKeysChangeSet`. "Later wins"
        // for scalar fields, revision-gated for balance/revision.
        for (id, entry) in other.identities {
            self.identities
                .entry(id)
                .and_modify(|existing| {
                    // Revision is the monotonic gate for replacing
                    // balance + revision. Balance rides the same gate
                    // because identity state transitions bump both
                    // together.
                    if entry.revision >= existing.revision {
                        existing.balance = entry.balance;
                        existing.revision = entry.revision;
                    }
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
                    // Append new contested DPNS labels. Dedup
                    // directly on the string since the field is a
                    // plain `Vec<String>`. Resolutions (contest
                    // won / locked) flow through a separate setter
                    // that shrinks the list, so an always-extend
                    // policy at merge time is correct.
                    for label in &entry.contested_dpns_names {
                        if !existing.contested_dpns_names.contains(label) {
                            existing.contested_dpns_names.push(label.clone());
                        }
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
    }

    fn is_empty(&self) -> bool {
        self.identities.is_empty() && self.removed.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Contacts
// ---------------------------------------------------------------------------

/// A single contact request entry in the changeset.
///
/// Modelled after [`crate::wallet::identity::ContactRequest`].
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
/// The live watermark is shared across the merged provider for a
/// network and may therefore be repeated across wallet-scoped
/// persistence callbacks; persisters should treat it as one checkpoint
/// per network, not one checkpoint per wallet.
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

/// Per-(identity, token) balance changes emitted by
/// [`crate::manager::identity_sync::IdentitySyncManager::sync_now`].
///
/// The watch list itself is no longer changeset-replicated — it lives
/// purely in the manager's in-memory cache. Persistence carries only
/// the post-sync balance updates and tombstones.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TokenBalanceChangeSet {
    /// Updated token balances keyed by `(identity_id, token_id)`.
    /// Last write wins on merge.
    pub balances: BTreeMap<(Identifier, Identifier), u64>,

    /// Balances removed (sync returned `None`, i.e. the identity no
    /// longer holds this token on Platform).
    pub removed_balances: BTreeSet<(Identifier, Identifier)>,
}

impl Merge for TokenBalanceChangeSet {
    fn merge(&mut self, other: Self) {
        self.balances.extend(other.balances);
        self.removed_balances.extend(other.removed_balances);
    }

    fn is_empty(&self) -> bool {
        self.balances.is_empty() && self.removed_balances.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Wallet registration metadata + per-account spec / address-pool snapshots
// ---------------------------------------------------------------------------

/// Per-wallet metadata captured at registration. Carries fields not
/// derivable from the xpub alone: which network the wallet is bound
/// to and the birth-height best estimate (the SPV tip at create time;
/// 0 means "scan from genesis / unknown").
///
/// The shape sits on [`PlatformWalletChangeSet`] as
/// `Option<WalletMetadataEntry>` because the round emits at most one
/// metadata blob per wallet — last-write-wins covers the rare race
/// where two registrations fire for the same wallet id.
///
/// `Network` does not implement `Default`, so this entry intentionally
/// only enters the changeset via explicit construction at registration
/// time; the parent `Option<...>` field stays `None` for every other
/// flush.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalletMetadataEntry {
    /// Network the wallet is bound to.
    pub network: Network,
    /// Best estimate of the chain tip at creation time. `0` means
    /// "scan from genesis / unknown".
    pub birth_height: u32,
}

/// One entry per registered account. Captures the per-account xpub
/// + type so a future load path can rebuild the wallet watch-only
/// via `Account::from_xpub`. Hardened derivation at the account
/// level means this is the only way to recover without the
/// mnemonic.
///
/// Carried on [`PlatformWalletChangeSet`] as
/// `Vec<AccountRegistrationEntry>`. `AccountType` is `PartialEq`
/// but not `Ord`/`Hash`, so a `BTreeMap` keyed by it isn't possible
/// without a derived index. In practice each account is emitted
/// exactly once per registration round, and the apply path runs
/// these through `Account::from_xpub` which is idempotent on
/// duplicate `(account_type, xpub)` pairs, so the merge policy
/// is simple `extend` and dedup is the apply-side caller's
/// responsibility if it ever matters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRegistrationEntry {
    /// The account variant being registered.
    pub account_type: AccountType,
    /// Bincode-encoded extended public key for this account.
    pub account_xpub: ExtendedPubKey,
}

/// Address-pool snapshot for one `(account_type, pool_type)` pair.
///
/// Routed through the changeset rather than a dedicated trait method
/// so the registration round (metadata + per-account specs +
/// per-pool snapshots) is one atomic
/// [`PlatformWalletPersistence::store`](crate::changeset::PlatformWalletPersistence::store)
/// from the backend's perspective.
///
/// **Merge policy** on the parent
/// [`PlatformWalletChangeSet::account_address_pools`] field is plain
/// `Vec::extend` — entries are *not* deduplicated by
/// `(account_type, pool_type)`. The FFI emits whole-pool snapshots,
/// so a second snapshot for the same key inside one merged round
/// represents the latest pool state and the apply-time consumer is
/// expected to treat the last entry per `(account_type, pool_type)`
/// as authoritative. Mid-round multi-snapshots for the same key are
/// not produced by any current emitter (snapshots fire at register,
/// pool extension, and used-flag flip — each on a fresh `store`
/// round), so this is a forward-looking documentation of intent
/// rather than a hot path.
///
/// Not `PartialEq` — `AddressInfo` upstream is `Debug + Clone` only,
/// so structural equality on `addresses` would require us to fork
/// the upstream type. Tests that need to inspect snapshot contents
/// reach into the `addresses` vec by index instead.
#[derive(Debug, Clone)]
pub struct AccountAddressPoolEntry {
    /// Which account this pool belongs to.
    pub account_type: AccountType,
    /// Pool variant (External / Internal / Absent / AbsentHardened).
    pub pool_type: AddressPoolType,
    /// Snapshot of every `AddressInfo` entry in the pool at emit time.
    pub addresses: Vec<AddressInfo>,
}

// ---------------------------------------------------------------------------
// Top-Level PlatformWalletChangeSet
// ---------------------------------------------------------------------------

/// Delta of all wallet state changes from a single operation.
///
/// `core` carries a [`CoreChangeSet`] — the platform-owned projection of
/// `WalletEvent` data delivered by upstream's event bus (records, UTXO
/// deltas, height checkpoints, IS-lock updates). Platform-specific deltas
/// (identities, contacts, platform addresses, asset locks, token balances)
/// live in dedicated sub-changesets.
///
/// Composed of optional sub-changesets — `None` means no change in that
/// area. Use [`Merge::merge`] to combine multiple deltas before persisting.
///
/// Not `PartialEq` because [`CoreChangeSet`] isn't (its `records` carry
/// `TransactionRecord`, which is `Debug + Clone` only upstream).
#[derive(Debug, Clone, Default)]
pub struct PlatformWalletChangeSet {
    /// Core-wallet deltas projected from upstream `WalletEvent`s:
    /// transaction records, UTXO add/remove, height checkpoints, IS-lock
    /// updates for non-final records.
    pub core: Option<CoreChangeSet>,
    /// Identity changes (registered, updated).
    pub identities: Option<IdentityChangeSet>,
    /// Identity key changes (public keys + private-key storage) keyed
    /// by `(identity_id, key_id)`. Separate from [`IdentityChangeSet`]
    /// so scalar-only mutations (e.g. `set_balance`, `set_label`) don't
    /// drag the full public-key + private-key payload through every
    /// persist.
    pub identity_keys: Option<IdentityKeysChangeSet>,
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
    /// Per-wallet metadata emitted once at registration. See
    /// [`WalletMetadataEntry`] for the merge policy.
    pub wallet_metadata: Option<WalletMetadataEntry>,
    /// Per-account registration entries emitted at registration / on
    /// later `add_account` calls. See [`AccountRegistrationEntry`] for
    /// the merge policy (plain `Vec::extend`, dedup is the apply-side
    /// caller's job).
    pub account_registrations: Vec<AccountRegistrationEntry>,
    /// Address-pool snapshots emitted at wallet create (initial
    /// gap-limit population) and on any pool extension / "used" flip.
    /// See [`AccountAddressPoolEntry`] for the merge policy.
    pub account_address_pools: Vec<AccountAddressPoolEntry>,
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

impl From<IdentityKeysChangeSet> for PlatformWalletChangeSet {
    fn from(cs: IdentityKeysChangeSet) -> Self {
        Self {
            identity_keys: Some(cs),
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
        // `CoreChangeSet` implements `Merge`; delegate via the
        // `Option<T>: Merge` blanket impl from this crate's merge module.
        self.core.merge(other.core);
        self.identities.merge(other.identities);
        self.identity_keys.merge(other.identity_keys);
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
        // Wallet metadata: last-write-wins. `Network` doesn't
        // implement `Default`, so we can't lean on the `Option<T>:
        // Merge` blanket impl (which requires `T: Merge: Default`);
        // instead, `Some(other) -> overwrite`, `None -> keep current`.
        if let Some(meta) = other.wallet_metadata {
            self.wallet_metadata = Some(meta);
        }
        // Per-account specs and address-pool snapshots: append-only.
        // See the type docstrings for the rationale (registration
        // round emits each key once; snapshots are whole-pool, so
        // duplicate keys within one merged round are a no-op).
        self.account_registrations
            .extend(other.account_registrations);
        self.account_address_pools
            .extend(other.account_address_pools);
    }

    fn is_empty(&self) -> bool {
        self.core.is_empty()
            && self.identities.is_empty()
            && self.identity_keys.is_empty()
            && self.contacts.is_empty()
            && self.platform_addresses.is_empty()
            && self.asset_locks.is_empty()
            && self.token_balances.is_empty()
            && self.dashpay_profiles.as_ref().is_none_or(|m| m.is_empty())
            && self
                .dashpay_payments_overlay
                .as_ref()
                .is_none_or(|m| m.is_empty())
            && self.wallet_metadata.is_none()
            && self.account_registrations.is_empty()
            && self.account_address_pools.is_empty()
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
        a.removed_balances.insert((identity_a, token_y));

        let mut b = TokenBalanceChangeSet::default();
        // Same identity/token — last-write-wins on balances.
        b.balances.insert((identity_a, token_x), 200);
        // New identity.
        b.balances.insert((identity_b, token_x), 50);
        // Tombstone propagates as set union.
        b.removed_balances.insert((identity_b, token_y));

        a.merge(b);

        assert_eq!(a.balances.get(&(identity_a, token_x)), Some(&200));
        assert_eq!(a.balances.get(&(identity_b, token_x)), Some(&50));
        assert!(a.removed_balances.contains(&(identity_a, token_y)));
        assert!(a.removed_balances.contains(&(identity_b, token_y)));
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
