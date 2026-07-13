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
use dashcore::ephemerealdata::chain_lock::ChainLock;
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
use crate::wallet::identity::{
    ContactProfileEntry, ContactRequest, DashPayProfile, EstablishedContact, PaymentEntry,
};

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Addresses freshly derived as a side effect of processing the
    /// records in this batch — i.e. `WalletEvent::TransactionDetected.
    /// addresses_derived` and `WalletEvent::BlockProcessed.
    /// addresses_derived`. The persister mirrors these into its
    /// address-pool table (e.g. `PersistentCoreAddress` on the Swift
    /// side) so UTXOs landing on freshly-derived addresses can be
    /// linked back to the canonical pool row instead of being
    /// orphaned at the `coreAddress` link. De-duplicated on merge by
    /// `(account_type, pool_type, derivation_index)` — same key the
    /// upstream `project_derived_addresses` uses, so two records in
    /// the same flush both pushing the same gap-limit boundary
    /// collapse to one entry.
    ///
    /// `#[serde(skip)]`: persisters that need the breadcrumb write
    /// it to a dedicated typed table (see
    /// `platform_wallet_storage::sqlite::schema::core_state`) rather
    /// than serialising the parent changeset wholesale, so excluding
    /// it from the serde round-trip has no functional cost even now
    /// that `key-wallet-manager/serde` would make it serializable.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub addresses_derived: Vec<key_wallet_manager::DerivedAddress>,

    /// Addresses the wallet marked **used** while processing the
    /// records in this batch — the persistence-seam counterpart of
    /// upstream `wallet_checker`'s in-memory `mark_address_used`
    /// calls, which the `WalletEvent` bus does not carry. Rebuilt by
    /// the event bridge from the post-processing pool state (the
    /// authoritative `AddressInfo`, `used == true`) so persisters can
    /// flip their mirrored address rows. Without this delta a match
    /// found during SPV block processing (a TXO landing on a BIP44
    /// address, or a special-tx payload hitting a provider owner /
    /// voting key) updates only the in-memory pool and every store
    /// keeps `is_used = false` forever.
    ///
    /// De-duplicated on merge by `(account_type, pool_type, index)`,
    /// same key discipline as [`Self::addresses_derived`]. Re-emitting
    /// an already-used address is idempotent on the persister side.
    ///
    /// `#[serde(skip)]`: same rationale as `addresses_derived` — the
    /// breadcrumb targets typed persister tables, not the serialized
    /// parent changeset.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub addresses_marked_used: Vec<key_wallet::transaction_checking::DerivedAddressInfo>,

    /// Post-batch highest-used derivation indexes for every account
    /// that had an address marked used in this batch, read from the
    /// authoritative in-memory pools (`AddressPool::highest_used`)
    /// right after the wallet processed the records. Single-pool
    /// accounts (provider keys, identity funding — pool type Absent /
    /// AbsentHardened) surface their pool in the `external` slot,
    /// matching how the FFI account row exposes exactly two
    /// highest-used fields. Monotonic-max on merge per account per
    /// slot; `None` means "no update".
    ///
    /// `#[serde(skip)]`: persister breadcrumb, same as the address
    /// deltas above.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub account_highest_used: BTreeMap<AccountType, HighestUsedIndexes>,

    /// Highest chainlock the wallet has applied (mirrors
    /// `WalletMetadata::last_applied_chain_lock`). Populated by the
    /// `ChainLockProcessed` bridge arm so the persister can
    /// roundtrip this field across app restarts — without persisting
    /// it, `metadata.last_applied_chain_lock` starts as `None` on
    /// every load and the asset-lock-resume metadata fallback
    /// (`proof.rs`) can't fire until SPV re-applies a fresh CL.
    ///
    /// Monotonic-max on merge by `block_height` (a chainlock at a
    /// lower height never overwrites a higher one — chain locks are
    /// strictly forward-advancing per upstream's contract).
    pub last_applied_chain_lock: Option<ChainLock>,
}

/// Highest-used derivation index per pool slot for one account, as
/// carried by [`CoreChangeSet::account_highest_used`].
///
/// Accounts expose at most two persisted highest-used watermarks
/// (external / internal). Standard accounts map their External /
/// Internal pools onto the matching slot; single-pool accounts
/// (provider keys, identity funding) surface their sole pool in
/// `external`. `None` means the pool has never had a used address (or
/// the account has no such pool) — distinct from `Some(0)`, which
/// means index #0 is used.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HighestUsedIndexes {
    /// Highest used index of the external (or sole) pool.
    pub external: Option<u32>,
    /// Highest used index of the internal (change) pool.
    pub internal: Option<u32>,
}

impl HighestUsedIndexes {
    /// Fold `other` in with monotonic-max semantics per slot —
    /// watermarks only advance, `None` never overwrites `Some`.
    pub fn merge_max(&mut self, other: Self) {
        if let Some(v) = other.external {
            self.external = Some(self.external.map_or(v, |e| e.max(v)));
        }
        if let Some(v) = other.internal {
            self.internal = Some(self.internal.map_or(v, |e| e.max(v)));
        }
    }
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

        // Derived-address dedup. Within a single `WalletEvent` the
        // upstream `project_derived_addresses` already deduped on
        // `(account_type, pool_type, derivation_index)`, but a flush
        // can fold multiple events together (TransactionDetected +
        // BlockProcessed for the same wallet over a sync round), and
        // a tx in one event can push the same boundary as a tx in
        // another. Build a fast-lookup set from current entries, then
        // append only entries we haven't seen yet — preserves arrival
        // order so the persister's writes line up with the same
        // derivation order the wallet's pool sees.
        // Chain-lock watermark: monotonic-max by `block_height`. A
        // later changeset's chain lock at a higher (or equal) height
        // wins; anything lower is ignored. `None` means "no update
        // in this batch", same convention as `synced_height`.
        if let Some(other_cl) = other.last_applied_chain_lock {
            let take = self
                .last_applied_chain_lock
                .as_ref()
                .is_none_or(|existing| other_cl.block_height >= existing.block_height);
            if take {
                self.last_applied_chain_lock = Some(other_cl);
            }
        }

        if !other.addresses_derived.is_empty() {
            let mut seen: std::collections::HashSet<(
                key_wallet::account::AccountType,
                key_wallet::managed_account::address_pool::AddressPoolType,
                u32,
            )> = self
                .addresses_derived
                .iter()
                .map(|d| (d.account_type, d.pool_type, d.derivation_index))
                .collect();
            for d in other.addresses_derived {
                let key = (d.account_type, d.pool_type, d.derivation_index);
                if seen.insert(key) {
                    self.addresses_derived.push(d);
                }
            }
        }

        // Marked-used dedup: same `(account_type, pool_type, index)`
        // key as the derived-address dedup above. Re-emitting a used
        // flip is idempotent at the persister, so first-seen-wins is
        // purely a payload-size optimization.
        if !other.addresses_marked_used.is_empty() {
            let mut seen: std::collections::HashSet<(
                key_wallet::account::AccountType,
                key_wallet::managed_account::address_pool::AddressPoolType,
                u32,
            )> = self
                .addresses_marked_used
                .iter()
                .map(|d| (d.account_type, d.pool_type, d.info.index))
                .collect();
            for d in other.addresses_marked_used {
                let key = (d.account_type, d.pool_type, d.info.index);
                if seen.insert(key) {
                    self.addresses_marked_used.push(d);
                }
            }
        }

        // Highest-used watermarks: monotonic-max per account per pool
        // slot, same forward-only discipline as the height watermarks.
        for (account_type, indexes) in other.account_highest_used {
            self.account_highest_used
                .entry(account_type)
                .or_default()
                .merge_max(indexes);
        }
    }

    fn is_empty(&self) -> bool {
        self.records.is_empty()
            && self.spent_utxos.is_empty()
            && self.new_utxos.is_empty()
            && self.instant_locks_for_non_final_records.is_empty()
            && self.last_processed_height.is_none()
            && self.synced_height.is_none()
            && self.addresses_derived.is_empty()
            && self.addresses_marked_used.is_empty()
            && self.account_highest_used.is_empty()
            && self.last_applied_chain_lock.is_none()
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Cached contact profiles keyed by the contact's identity id. Like
    /// `dashpay_payments`, every snapshot carries the full map via
    /// `from_managed`, so merge uses last-write-wins per contact id.
    pub contact_profiles: BTreeMap<Identifier, ContactProfileEntry>,
    /// Senders this identity has chosen to **ignore** (per-sender mute, =
    /// block, reversible — local-only). Every snapshot carries the full set
    /// via `from_managed`, so merge takes the **union** (a member appearing
    /// in either side stays ignored; un-ignore is carried by an explicit
    /// removal on [`ContactChangeSet::unignored`], not by a shrinking
    /// snapshot here — same insert-XOR-tombstone discipline the contact
    /// request fields use).
    pub ignored_senders: BTreeSet<Identifier>,
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
            dashpay_profile: managed.dashpay().profile.clone(),
            dashpay_payments: managed.dashpay().payments.clone(),
            contact_profiles: managed.dashpay().contact_profiles.clone(),
            ignored_senders: managed.dashpay().ignored_senders().clone(),
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdentityKeyDerivationIndices {
    /// DIP-9 identity index (hardened).
    pub identity_index: u32,
    /// DIP-9 key index within the identity (hardened).
    pub key_index: u32,
}

/// A derivation breadcrumb as the raw `(wallet_id, identity_index,
/// key_index)` triple passed to `ManagedIdentity::add_key` / `add_keys`.
/// `Some` lets the client re-derive the private key from the wallet seed;
/// `None` marks a watch-only key.
pub type KeyDerivationBreadcrumb = ([u8; 32], u32, u32);

/// One public key paired with its derivation breadcrumb — the unit
/// `ManagedIdentity::add_keys` consumes and `discovery::breadcrumb_decisions`
/// produces.
///
/// Discovery derives a candidate scalar, validates it against the on-chain
/// key, and emits a breadcrumb (the DIP-9 coordinates) only when it matches.
/// The scalar itself is never carried out — the client derives the key on
/// demand from the Keychain seed at the breadcrumb path. `breadcrumb` is
/// `None` for a watch-only key.
pub struct KeyWithBreadcrumb {
    /// The DPP public-key record.
    pub key: dpp::identity::IdentityPublicKey,
    /// Derivation coordinates for re-derivable keys; `None` for watch-only.
    pub breadcrumb: Option<KeyDerivationBreadcrumb>,
}

/// A single identity-key entry in an [`IdentityKeysChangeSet`].
///
/// Carries the DPP public-key record and a breadcrumb pointing at the wallet
/// derivation that produced it. No private material crosses here: the client
/// derives the key on demand from the Keychain seed at the breadcrumb path
/// (`m/9'/coin'/5'/0'/ECDSA'/identity_index'/key_index'`). When
/// `derivation_indices` is `None` the key is watch-only from this wallet's
/// point of view.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
                    // Merge contact profiles (last-write-wins per contact id),
                    // same policy as `dashpay_payments`.
                    for (contact_id, profile) in &entry.contact_profiles {
                        existing
                            .contact_profiles
                            .insert(*contact_id, profile.clone());
                    }
                    // Ignored senders: UNION. A sender ignored in either
                    // snapshot stays ignored; un-ignore is carried by an
                    // explicit `ContactChangeSet::unignored` removal, so a
                    // snapshot that no longer lists a sender must NOT silently
                    // un-ignore them at merge time.
                    existing
                        .ignored_senders
                        .extend(entry.ignored_senders.iter().copied());
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContactRequestEntry {
    /// The contact request.
    pub request: ContactRequest,
}

/// Key for sent contact requests: the **owner** sent a request TO the
/// **recipient**. Used for `sent_requests` and `removed_sent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
/// # Merge reconciliation
///
/// Every apply layer (in-memory, SQLite, FFI projection) runs all
/// inserts before all removes, so a merged changeset that carried the
/// same key in both an insert map and its opposing tombstone set would
/// always resolve to "removed". `ContactChangeSet::merge` therefore
/// reconciles each insert-vs-tombstone pair last-write-wins per key:
/// the newer delta's action for a key cancels the older opposing action
/// (a `sent_requests` insert clears a prior `removed_sent`, an un-ignore
/// clears a prior ignore, and vice versa), keeping the two sets
/// disjoint. This covers `sent_requests` vs `removed_sent`,
/// `incoming_requests` vs `removed_incoming`, and `ignored` vs
/// `unignored`. `established` has no opposing tombstone set and rides
/// plain last-write-wins.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Ignored senders (per-sender mute, = block, reversible — local-only),
    /// keyed by `(owner, sender)`. Suppresses ALL of the sender's incoming
    /// requests (including rotated, bumped-`accountReference` ones) from the
    /// main pending list, and the suppression survives a recurring re-sync.
    /// Reconciled last-write-wins against [`Self::unignored`] on merge.
    pub ignored: BTreeSet<(Identifier, Identifier)>,
    /// Senders **un-ignored** in this delta, keyed by `(owner, sender)`. The
    /// removal tombstone for [`Self::ignored`] — the persister deletes the
    /// ignored-sender row so the sender's requests resurface on the next
    /// sweep. Kept as a separate set (rather than a shrinking `ignored`
    /// snapshot) so the changeset's insert-XOR-tombstone discipline holds.
    pub unignored: BTreeSet<(Identifier, Identifier)>,
}

impl Merge for ContactChangeSet {
    fn merge(&mut self, other: Self) {
        // Insert-vs-tombstone pairs are reconciled last-write-wins per key:
        // `other` is the newer delta, so a key it inserts cancels an older
        // same-key tombstone and vice versa. Without this the two sets could
        // both carry the same key and apply (which runs inserts before
        // removes at every layer) would always resolve to "removed" — losing
        // a re-send / re-ignore that happened after a remove / un-ignore.
        // The three pairs share this idiom; `established` has no opposing
        // tombstone set and rides plain last-write-wins.
        for key in other.removed_sent.iter() {
            self.sent_requests.remove(key);
        }
        for key in other.sent_requests.keys() {
            self.removed_sent.remove(key);
        }
        self.sent_requests.extend(other.sent_requests);
        self.removed_sent.extend(other.removed_sent);

        for key in other.removed_incoming.iter() {
            self.incoming_requests.remove(key);
        }
        for key in other.incoming_requests.keys() {
            self.removed_incoming.remove(key);
        }
        self.incoming_requests.extend(other.incoming_requests);
        self.removed_incoming.extend(other.removed_incoming);

        self.established.extend(other.established);

        for key in other.unignored.iter() {
            self.ignored.remove(key);
        }
        for key in other.ignored.iter() {
            self.unignored.remove(key);
        }
        self.ignored.extend(other.ignored);
        self.unignored.extend(other.unignored);
    }

    fn is_empty(&self) -> bool {
        self.sent_requests.is_empty()
            && self.removed_sent.is_empty()
            && self.incoming_requests.is_empty()
            && self.removed_incoming.is_empty()
            && self.established.is_empty()
            && self.ignored.is_empty()
            && self.unignored.is_empty()
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlatformAddressBalanceEntry {
    pub wallet_id: WalletId,
    pub account_index: u32,
    pub address_index: u32,
    pub address: PlatformP2PKHAddress,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::changeset::serde_adapters::address_funds")
    )]
    pub funds: AddressFunds,
}

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetLockEntry {
    /// The outpoint identifying this credit output (txid + vout).
    pub out_point: OutPoint,
    /// The full asset lock transaction.
    pub transaction: Transaction,
    /// BIP44 account index that funded this asset lock (UTXO source).
    pub account_index: u32,
    /// Which funding account to derive the one-time key from.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::changeset::serde_adapters::asset_lock_funding_type")
    )]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
/// to, the network-independent group id that ties a seed's per-network
/// wallets together, and the birth-height best estimate (the SPV tip
/// at create time; 0 means "scan from genesis / unknown").
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WalletMetadataEntry {
    /// Network the wallet is bound to.
    pub network: Network,
    /// Network-INDEPENDENT 32-byte id shared by every network's wallet
    /// derived from the same seed. Computed as
    /// `Wallet::compute_wallet_id_from_root_extended_pub_key(root, None)`
    /// — `SHA256(root_public_key || root_chain_code)` with no network
    /// byte folded in. Distinct from the per-network [`Self::network`]-
    /// scoped `wallet_id` the changeset is keyed on: that id differs per
    /// network for the same seed, this one is the same across all of
    /// them, so consumers can group a seed's sibling-network rows by it.
    /// For watch-only / external-signable wallets (which carry no root
    /// key) this falls back to the scoped `wallet_id` — a group of one.
    pub wallet_group_id: [u8; 32],
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountRegistrationEntry {
    /// The account variant being registered.
    pub account_type: AccountType,
    /// Bincode-encoded extended public key for this account.
    pub account_xpub: ExtendedPubKey,
}

/// Non-secp256k1 extended public key carried by a
/// [`ProviderKeyAccountEntry`].
///
/// The BLS operator-key account and the EdDSA platform-node-key account
/// each hold an extended public key over their own curve, not a
/// secp256k1 [`ExtendedPubKey`], so they can't ride the
/// [`AccountRegistrationEntry`] path. Variants are gated on the
/// `bls` / `eddsa` features that make the underlying account types
/// exist upstream; with both off the enum is uninhabited (no provider
/// key account can be produced).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProviderKeyExtendedPubKey {
    /// Extended BLS public key of a `ProviderOperatorKeys` account.
    #[cfg(feature = "bls")]
    Bls(key_wallet::derivation_bls_bip32::ExtendedBLSPubKey),
    /// Extended Ed25519 public key of a `ProviderPlatformKeys` account.
    #[cfg(feature = "eddsa")]
    EdDSA(key_wallet::derivation_slip10::ExtendedEd25519PubKey),
}

/// One pre-derived platform-node (Ed25519) public key captured at
/// registration, in the forms the host displays without needing the
/// seed again.
///
/// Ed25519/SLIP-10 is hardened-only — there is no public-key
/// derivation, so the wallet can never extend its platform-node pool
/// on demand the way the BLS operator pool does (non-hardened
/// `ckd_pub` off the account xpub). Pre-generating a fixed batch while
/// the seed is in hand at registration is therefore the only way to
/// list these keys later from an external-signable / watch-only
/// wallet without re-prompting for the mnemonic. Only the public parts
/// are carried — the private scalar stays resolver-gated per index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProviderPlatformNodePubKey {
    /// Hardened key index within the platform-node pool (`#0..`).
    pub index: u32,
    /// Raw 32-byte Ed25519 public key at this index.
    pub public_key: [u8; 32],
    /// The 20-byte platform node id — `hash160` of the Ed25519 public
    /// key, exactly what a ProRegTx `platform_node_id` field carries.
    /// Precomputed on the Rust side so the host renders it without a
    /// RIPEMD-160 implementation of its own.
    pub node_id: [u8; 20],
}

/// One entry per provider **key-material** account captured at
/// registration — the BLS operator-key account
/// ([`AccountType::ProviderOperatorKeys`]) and the EdDSA
/// platform-node-key account ([`AccountType::ProviderPlatformKeys`]).
///
/// Upstream stores these in dedicated `Option` fields on the
/// `AccountCollection`, which `all_accounts()` deliberately excludes,
/// so they never enter the [`Self::account_xpub`](AccountRegistrationEntry)
/// snapshot the ECDSA accounts ride. Carried on
/// [`PlatformWalletChangeSet`] as
/// `Vec<ProviderKeyAccountEntry>`; the FFI layer bincode-encodes the
/// [`extended_public_key`](Self::extended_public_key) into the same
/// `AccountSpecFFI.account_xpub_bytes` slot the ECDSA accounts use (the
/// `type_tag` disambiguates the decode) and the restore side rebuilds a
/// watch-only `BLSAccount` / `EdDSAAccount` from it. Append-only merge,
/// same as [`AccountRegistrationEntry`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProviderKeyAccountEntry {
    /// `ProviderOperatorKeys` (BLS) or `ProviderPlatformKeys` (EdDSA).
    pub account_type: AccountType,
    /// The account's extended public key.
    pub extended_public_key: ProviderKeyExtendedPubKey,
    /// Pre-derived platform-node (Ed25519) public keys, captured at
    /// registration while the seed was in hand. Only populated for the
    /// `ProviderPlatformKeys` (EdDSA) entry — always empty for the BLS
    /// operator entry, whose pool the wallet can re-derive on demand
    /// from the account xpub (non-hardened `ckd_pub`, no seed). The FFI
    /// layer surfaces these to the host as a flat display array so the
    /// Node Keys screen can list them from persistence with no keychain
    /// prompt. See [`ProviderPlatformNodePubKey`].
    pub derived_platform_node_keys: Vec<ProviderPlatformNodePubKey>,
}

/// A [`ProviderKeyAccountEntry`] minus its
/// [`derived_platform_node_keys`](ProviderKeyAccountEntry::derived_platform_node_keys) —
/// the shape a backend serializes into a single account-registration slot.
///
/// The node-key list is an unbounded one-to-many and belongs in its own
/// rows (the SQLite backend gives it a child table), so a backend that
/// stores the account as one opaque payload carries only the two scalar
/// fields here. [`account_type`](Self::account_type) is kept even though a
/// backend also indexes it out-of-band: it lets a reader cross-check the
/// typed column against the decoded payload and reject a mis-bucketed or
/// cross-curve-confused row.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProviderKeyRegistrationBlob {
    /// `ProviderOperatorKeys` (BLS) or `ProviderPlatformKeys` (EdDSA).
    pub account_type: AccountType,
    /// The account's extended public key.
    pub extended_public_key: ProviderKeyExtendedPubKey,
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountAddressPoolEntry {
    /// Which account this pool belongs to.
    pub account_type: AccountType,
    /// Pool variant (External / Internal / Absent / AbsentHardened).
    pub pool_type: AddressPoolType,
    /// Snapshot of every `AddressInfo` entry in the pool at emit time.
    pub addresses: Vec<AddressInfo>,
}

// ---------------------------------------------------------------------------
// Deferred contact-crypto queue (seedless background-sync deferral)
// ---------------------------------------------------------------------------

/// A DashPay contact-crypto operation that the background sync sweep could not
/// perform because key material wasn't available at the time (watch-only
/// wallet / Keychain signer not unlocked).
///
/// The sweep runs with no signer; rather than churn (receiving account) or
/// irreversibly break the channel (external account), it **enqueues** the op
/// here and the entry is drained when a signer becomes available (Keychain
/// unlock, or any signer-present DashPay action). The queue carries **only
/// ciphertext + public key indices** — never a secret — so it is safe to
/// persist, which it must be: a restore-from-Keychain is exactly when a
/// discovered contact would otherwise be stranded.
///
/// One op per `(owner, contact, kind)` — see [`PendingContactCryptoKey`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PendingContactCryptoOp {
    /// Derive our own DashPay receiving xpub (the friendship key) and register
    /// the receiving account. No secret payload — the path is built from the
    /// `(owner, contact)` identity ids. First-time only; a no-op once the
    /// account is persisted.
    RegisterReceiving,
    /// Decrypt the contact's encrypted xpub via ECDH and register the external
    /// (sending) account. Carries the on-chain ciphertext + the already-
    /// validated key indices — all public.
    RegisterExternal {
        /// The contact's DIP-15 `encryptedPublicKey` blob (ciphertext).
        encrypted_public_key: Vec<u8>,
        /// Our decryption key index (validated upstream).
        our_decryption_key_index: u32,
        /// The contact's encryption key index (validated upstream).
        contact_encryption_key_index: u32,
    },
    /// Re-fetch + decrypt this identity's contactInfo documents. Idempotent;
    /// carries no payload (the drain re-fetches the owned docs).
    ContactInfoDecrypt,
    /// Verify a DIP-15 `autoAcceptProof` on an inbound contact request and, if
    /// valid + unexpired, auto-accept it (send the reciprocal). No payload — the
    /// `contact_id` is the request sender; the drain re-loads the request (and
    /// its proof) from the incoming-requests map. Verify + accept both need a
    /// signer, so this can only run in the signer-present drain, never the sweep.
    AutoAccept,
}

/// The kind discriminant of a [`PendingContactCryptoOp`] — the part of the
/// dedup identity that ignores the (secret-free) payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PendingContactCryptoKind {
    RegisterReceiving,
    RegisterExternal,
    ContactInfoDecrypt,
    AutoAccept,
}

impl PendingContactCryptoOp {
    /// The kind discriminant, for dedup keying.
    pub fn kind(&self) -> PendingContactCryptoKind {
        match self {
            Self::RegisterReceiving => PendingContactCryptoKind::RegisterReceiving,
            Self::RegisterExternal { .. } => PendingContactCryptoKind::RegisterExternal,
            Self::ContactInfoDecrypt => PendingContactCryptoKind::ContactInfoDecrypt,
            Self::AutoAccept => PendingContactCryptoKind::AutoAccept,
        }
    }
}

/// One deferred contact-crypto op. The queue holds at most one entry per
/// [`key`](Self::key); re-enqueuing the same `(owner, contact, kind)` is a
/// no-op (the latest payload wins).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PendingContactCrypto {
    /// The wallet-owned identity the op is for.
    pub owner_identity_id: Identifier,
    /// The contact identity the op concerns.
    pub contact_id: Identifier,
    /// What to do once a signer is available.
    pub op: PendingContactCryptoOp,
    /// Unix-millis enqueue time — observability / ordering only, NOT part of
    /// the dedup identity.
    pub enqueued_at_ms: u64,
}

impl PendingContactCrypto {
    /// The dedup identity: `(owner, contact, kind)`.
    pub fn key(&self) -> PendingContactCryptoKey {
        PendingContactCryptoKey {
            owner_identity_id: self.owner_identity_id,
            contact_id: self.contact_id,
            kind: self.op.kind(),
        }
    }
}

/// Dedup / removal identity for a [`PendingContactCrypto`] entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PendingContactCryptoKey {
    pub owner_identity_id: Identifier,
    pub contact_id: Identifier,
    pub kind: PendingContactCryptoKind,
}

/// Insert `entry` into a deferred-crypto queue, replacing any existing entry
/// with the same [`PendingContactCryptoKey`] (latest payload wins) so the
/// queue holds at most one op per `(owner, contact, kind)`. Used by both the
/// in-memory enqueue and the persisted-queue apply path.
pub fn upsert_pending_contact_crypto(
    queue: &mut Vec<PendingContactCrypto>,
    entry: PendingContactCrypto,
) {
    if let Some(slot) = queue.iter_mut().find(|e| e.key() == entry.key()) {
        *slot = entry;
    } else {
        queue.push(entry);
    }
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Provider key-material accounts (BLS operator keys / EdDSA
    /// platform-node keys) emitted at registration. These live outside
    /// the ECDSA `all_accounts()` set upstream, so they ride their own
    /// vec rather than [`Self::account_registrations`]. See
    /// [`ProviderKeyAccountEntry`] for the merge policy (append-only).
    pub provider_key_account_registrations: Vec<ProviderKeyAccountEntry>,
    /// Address-pool snapshots emitted at wallet create (initial
    /// gap-limit population) and on any pool extension / "used" flip.
    /// See [`AccountAddressPoolEntry`] for the merge policy.
    pub account_address_pools: Vec<AccountAddressPoolEntry>,
    /// Deferred contact-crypto ops enqueued by the seedless background sweep
    /// (key material unavailable). Append-only delta; apply inserts into the
    /// persisted queue, deduped by [`PendingContactCryptoKey`]. Secret-free.
    /// See [`PendingContactCrypto`].
    pub pending_contact_crypto_added: Vec<PendingContactCrypto>,
    /// Keys of deferred ops to remove (drained successfully, or permanently
    /// failed). Append-only delta; apply removes matching `(owner, contact,
    /// kind)` from the persisted queue.
    pub pending_contact_crypto_cleared: Vec<PendingContactCryptoKey>,
    /// Shielded sub-wallet deltas: per-subwallet decrypted notes,
    /// spent marks, sync watermarks, nullifier checkpoints. The
    /// commitment tree itself is **not** in here — it lives on
    /// disk in `ClientPersistentCommitmentTree`'s SQLite file.
    #[cfg(feature = "shielded")]
    pub shielded: Option<crate::changeset::ShieldedChangeSet>,
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
        self.provider_key_account_registrations
            .extend(other.provider_key_account_registrations);
        self.account_address_pools
            .extend(other.account_address_pools);
        // Deferred contact-crypto queue: append-only add/clear deltas; the
        // apply side dedups adds and removes cleared keys.
        self.pending_contact_crypto_added
            .extend(other.pending_contact_crypto_added);
        self.pending_contact_crypto_cleared
            .extend(other.pending_contact_crypto_cleared);
        #[cfg(feature = "shielded")]
        {
            self.shielded.merge(other.shielded);
        }
    }

    fn is_empty(&self) -> bool {
        let core_empty = self.core.is_empty()
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
            && self.provider_key_account_registrations.is_empty()
            && self.account_address_pools.is_empty()
            && self.pending_contact_crypto_added.is_empty()
            && self.pending_contact_crypto_cleared.is_empty();
        #[cfg(feature = "shielded")]
        {
            core_empty && self.shielded.as_ref().is_none_or(|s| s.is_empty())
        }
        #[cfg(not(feature = "shielded"))]
        {
            core_empty
        }
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

    /// The deferred contact-crypto queue rides the changeset as add/clear
    /// deltas: a pending enqueue OR a pending clear must mark the changeset
    /// non-empty (so the persist round isn't skipped and the queue survives a
    /// restart), merge extends both delta vecs, and the dedup key ignores the
    /// (secret-free) payload + timestamp but distinguishes the op kind.
    #[test]
    fn pending_contact_crypto_queue_deltas_merge_and_dedup_key() {
        let owner = Identifier::from([0x11; 32]);
        let contact = Identifier::from([0x22; 32]);

        let receiving = PendingContactCrypto {
            owner_identity_id: owner,
            contact_id: contact,
            op: PendingContactCryptoOp::RegisterReceiving,
            enqueued_at_ms: 0,
        };
        let external = PendingContactCrypto {
            owner_identity_id: owner,
            contact_id: contact,
            op: PendingContactCryptoOp::RegisterExternal {
                encrypted_public_key: vec![1, 2, 3],
                our_decryption_key_index: 4,
                contact_encryption_key_index: 5,
            },
            enqueued_at_ms: 7,
        };

        // A pending enqueue marks the changeset non-empty.
        let mut cs = PlatformWalletChangeSet {
            pending_contact_crypto_added: vec![receiving.clone()],
            ..Default::default()
        };
        assert!(
            !cs.is_empty(),
            "a pending enqueue must mark the changeset non-empty"
        );

        // A clear-only changeset is also non-empty (the removal must persist).
        let clear_only = PlatformWalletChangeSet {
            pending_contact_crypto_cleared: vec![external.key()],
            ..Default::default()
        };
        assert!(
            !clear_only.is_empty(),
            "a pending clear must mark the changeset non-empty"
        );

        // merge extends both delta vecs.
        cs.merge(PlatformWalletChangeSet {
            pending_contact_crypto_added: vec![external.clone()],
            pending_contact_crypto_cleared: vec![receiving.key()],
            ..Default::default()
        });
        assert_eq!(cs.pending_contact_crypto_added.len(), 2);
        assert_eq!(cs.pending_contact_crypto_cleared.len(), 1);

        // Dedup key ignores the payload + timestamp but distinguishes kind.
        let external_other_payload = PendingContactCrypto {
            owner_identity_id: owner,
            contact_id: contact,
            op: PendingContactCryptoOp::RegisterExternal {
                encrypted_public_key: vec![9, 9],
                our_decryption_key_index: 4,
                contact_encryption_key_index: 5,
            },
            enqueued_at_ms: 999,
        };
        assert_eq!(
            external.key(),
            external_other_payload.key(),
            "same (owner, contact, kind) → same dedup key regardless of payload/timestamp"
        );
        assert_ne!(
            receiving.key(),
            external.key(),
            "different op kind → different dedup key"
        );
    }

    /// `upsert_pending_contact_crypto` keeps at most one entry per
    /// `(owner, contact, kind)`: a duplicate kind replaces in place (latest
    /// payload + timestamp win, no growth), while a different kind is a new
    /// entry.
    #[test]
    fn upsert_pending_contact_crypto_dedups_by_key_latest_wins() {
        let owner = Identifier::from([1u8; 32]);
        let contact = Identifier::from([2u8; 32]);
        let mut q: Vec<PendingContactCrypto> = Vec::new();

        let recv = PendingContactCrypto {
            owner_identity_id: owner,
            contact_id: contact,
            op: PendingContactCryptoOp::RegisterReceiving,
            enqueued_at_ms: 1,
        };
        upsert_pending_contact_crypto(&mut q, recv.clone());
        upsert_pending_contact_crypto(&mut q, recv);
        assert_eq!(
            q.len(),
            1,
            "re-enqueuing the same kind must not grow the queue"
        );

        // A different kind is a separate entry.
        upsert_pending_contact_crypto(
            &mut q,
            PendingContactCrypto {
                owner_identity_id: owner,
                contact_id: contact,
                op: PendingContactCryptoOp::RegisterExternal {
                    encrypted_public_key: vec![1],
                    our_decryption_key_index: 0,
                    contact_encryption_key_index: 0,
                },
                enqueued_at_ms: 2,
            },
        );
        assert_eq!(q.len(), 2);

        // Same key, newer payload → replaced in place (latest wins, no growth).
        upsert_pending_contact_crypto(
            &mut q,
            PendingContactCrypto {
                owner_identity_id: owner,
                contact_id: contact,
                op: PendingContactCryptoOp::RegisterExternal {
                    encrypted_public_key: vec![9, 9],
                    our_decryption_key_index: 0,
                    contact_encryption_key_index: 0,
                },
                enqueued_at_ms: 3,
            },
        );
        assert_eq!(q.len(), 2, "replacing must not grow the queue");
        let stored = q
            .iter()
            .find(|e| e.op.kind() == PendingContactCryptoKind::RegisterExternal)
            .expect("external entry present");
        assert_eq!(stored.enqueued_at_ms, 3, "latest timestamp wins");
        match &stored.op {
            PendingContactCryptoOp::RegisterExternal {
                encrypted_public_key,
                ..
            } => assert_eq!(encrypted_public_key, &vec![9, 9], "latest payload wins"),
            _ => panic!("expected RegisterExternal"),
        }
    }

    #[test]
    fn test_platform_address_changeset_merge() {
        let wallet_id: WalletId = [9u8; 32];
        let addr1 = PlatformP2PKHAddress::new([1u8; 20]);
        let addr2 = PlatformP2PKHAddress::new([2u8; 20]);

        let funds = |balance, nonce| AddressFunds {
            balance,
            nonce,
            as_of_height: 0,
        };
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

    fn ignore_key() -> (Identifier, Identifier) {
        (Identifier::from([0xAA; 32]), Identifier::from([0xBB; 32]))
    }

    /// ignore → un-ignore for the same key resolves to exactly "un-ignored":
    /// the newer un-ignore cancels the older ignore, so the key lands in
    /// `unignored` only and never in `ignored`. Without cancellation the key
    /// would sit in both sets and apply (inserts before removes) would drop
    /// the block.
    #[test]
    fn contact_merge_ignore_then_unignore_last_write_wins() {
        let key = ignore_key();

        let mut base = ContactChangeSet {
            ignored: BTreeSet::from([key]),
            ..Default::default()
        };
        let newer = ContactChangeSet {
            unignored: BTreeSet::from([key]),
            ..Default::default()
        };

        base.merge(newer);

        assert!(
            !base.ignored.contains(&key),
            "the newer un-ignore must clear the older ignore"
        );
        assert!(base.unignored.contains(&key), "the key ends up un-ignored");
        assert_eq!(base.ignored.len(), 0);
        assert_eq!(base.unignored.len(), 1);
    }

    /// un-ignore → re-ignore for the same key resolves to exactly "ignored":
    /// the newer ignore cancels the older un-ignore (the F4 case — a
    /// transient-flush re-merge of an un-ignore followed by a re-ignore must
    /// keep the sender blocked).
    #[test]
    fn contact_merge_unignore_then_ignore_last_write_wins() {
        let key = ignore_key();

        let mut base = ContactChangeSet {
            unignored: BTreeSet::from([key]),
            ..Default::default()
        };
        let newer = ContactChangeSet {
            ignored: BTreeSet::from([key]),
            ..Default::default()
        };

        base.merge(newer);

        assert!(
            base.ignored.contains(&key),
            "the newer re-ignore must win over the older un-ignore"
        );
        assert!(
            !base.unignored.contains(&key),
            "the newer re-ignore must clear the older un-ignore"
        );
        assert_eq!(base.ignored.len(), 1);
        assert_eq!(base.unignored.len(), 0);
    }

    /// Cancellation is per key: an un-ignore of one sender must not disturb a
    /// separate sender's ignore carried in the same merge.
    #[test]
    fn contact_merge_ignore_cancellation_is_per_key() {
        let blocked = (Identifier::from([1u8; 32]), Identifier::from([2u8; 32]));
        let unblocked = (Identifier::from([1u8; 32]), Identifier::from([3u8; 32]));

        let mut base = ContactChangeSet {
            ignored: BTreeSet::from([blocked, unblocked]),
            ..Default::default()
        };
        let newer = ContactChangeSet {
            unignored: BTreeSet::from([unblocked]),
            ..Default::default()
        };

        base.merge(newer);

        assert!(
            base.ignored.contains(&blocked),
            "untouched sender stays ignored"
        );
        assert!(!base.ignored.contains(&unblocked));
        assert!(base.unignored.contains(&unblocked));
    }

    fn sent_key() -> SentContactRequestKey {
        SentContactRequestKey {
            owner_id: Identifier::from([1u8; 32]),
            recipient_id: Identifier::from([2u8; 32]),
        }
    }

    fn sent_entry() -> ContactRequestEntry {
        ContactRequestEntry {
            request: ContactRequest::new(
                Identifier::from([1u8; 32]),
                Identifier::from([2u8; 32]),
                0,
                0,
                0,
                vec![0u8; 96],
                100_000,
                0,
            ),
        }
    }

    /// remove-sent → re-send for the same key resolves to exactly "sent": the
    /// newer insert cancels the older tombstone, so the re-send survives apply
    /// (which runs inserts before removes).
    #[test]
    fn contact_merge_remove_sent_then_resend_last_write_wins() {
        let key = sent_key();

        let mut base = ContactChangeSet {
            removed_sent: BTreeSet::from([key]),
            ..Default::default()
        };
        let mut newer = ContactChangeSet::default();
        newer.sent_requests.insert(key, sent_entry());

        base.merge(newer);

        assert!(
            base.sent_requests.contains_key(&key),
            "the newer re-send must win over the older tombstone"
        );
        assert!(
            !base.removed_sent.contains(&key),
            "the newer re-send must clear the older tombstone"
        );
    }

    /// send → remove-sent for the same key resolves to exactly "removed": the
    /// newer tombstone cancels the older insert.
    #[test]
    fn contact_merge_send_then_remove_sent_last_write_wins() {
        let key = sent_key();

        let mut base = ContactChangeSet::default();
        base.sent_requests.insert(key, sent_entry());
        let newer = ContactChangeSet {
            removed_sent: BTreeSet::from([key]),
            ..Default::default()
        };

        base.merge(newer);

        assert!(
            !base.sent_requests.contains_key(&key),
            "the newer tombstone must clear the older insert"
        );
        assert!(base.removed_sent.contains(&key));
    }

    /// Same last-write-wins reconciliation for the incoming pair
    /// (`incoming_requests` vs `removed_incoming`).
    #[test]
    fn contact_merge_incoming_insert_vs_tombstone_last_write_wins() {
        let key = ReceivedContactRequestKey {
            owner_id: Identifier::from([2u8; 32]),
            sender_id: Identifier::from([1u8; 32]),
        };
        let entry = ContactRequestEntry {
            request: ContactRequest::new(
                Identifier::from([1u8; 32]),
                Identifier::from([2u8; 32]),
                0,
                0,
                0,
                vec![0u8; 96],
                100_000,
                0,
            ),
        };

        // tombstone then re-insert → insert wins.
        let mut base = ContactChangeSet {
            removed_incoming: BTreeSet::from([key]),
            ..Default::default()
        };
        let mut newer = ContactChangeSet::default();
        newer.incoming_requests.insert(key, entry.clone());
        base.merge(newer);
        assert!(base.incoming_requests.contains_key(&key));
        assert!(!base.removed_incoming.contains(&key));

        // insert then tombstone → tombstone wins.
        let mut base = ContactChangeSet::default();
        base.incoming_requests.insert(key, entry);
        let newer = ContactChangeSet {
            removed_incoming: BTreeSet::from([key]),
            ..Default::default()
        };
        base.merge(newer);
        assert!(!base.incoming_requests.contains_key(&key));
        assert!(base.removed_incoming.contains(&key));
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

    /// Compressed encoding of the secp256k1 generator point — a
    /// well-known valid public key for stubbing `AddressInfo`s.
    const TEST_PUBKEY_G: [u8; 33] = [
        0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87,
        0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16,
        0xf8, 0x17, 0x98,
    ];

    /// Stub a marked-used entry at `(account_type, pool_type, index)`.
    /// Tests only exercise the dedup key, not address↔pubkey
    /// consistency.
    fn stub_marked_used(
        account_type: AccountType,
        pool_type: AddressPoolType,
        index: u32,
    ) -> key_wallet::transaction_checking::DerivedAddressInfo {
        use key_wallet::bip32::{ChildNumber, DerivationPath};
        use key_wallet::managed_account::address_pool::{AddressInfo, PublicKeyType};

        let pubkey =
            dashcore::PublicKey::from_slice(&TEST_PUBKEY_G).expect("generator point is valid");
        let address = dashcore::Address::p2pkh(&pubkey, Network::Testnet);
        let script_pubkey = address.script_pubkey();
        let path = DerivationPath::from(vec![
            ChildNumber::from_normal_idx(0).expect("valid child number"),
            ChildNumber::from_normal_idx(index).expect("valid child number"),
        ]);
        key_wallet::transaction_checking::DerivedAddressInfo {
            account_type,
            pool_type,
            info: AddressInfo {
                address,
                script_pubkey,
                public_key: Some(PublicKeyType::ECDSA(TEST_PUBKEY_G.to_vec())),
                index,
                path,
                used: true,
                generated_at: 0,
                used_at: None,
                tx_count: 0,
                total_received: 0,
                total_sent: 0,
                balance: 0,
                label: None,
                metadata: BTreeMap::new(),
            },
        }
    }

    fn bip44_account_0() -> AccountType {
        AccountType::Standard {
            index: 0,
            standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
        }
    }

    /// A marked-used delta (or a highest-used watermark) alone must
    /// mark the core changeset non-empty, or the event adapter drops
    /// the persist round and the used flip never reaches any store —
    /// the exact bug this delta exists to fix.
    #[test]
    fn marked_used_and_highest_used_mark_core_changeset_non_empty() {
        let mut cs = CoreChangeSet::default();
        assert!(cs.is_empty());
        cs.addresses_marked_used = vec![stub_marked_used(
            bip44_account_0(),
            AddressPoolType::External,
            0,
        )];
        assert!(!cs.is_empty(), "marked-used delta must be persisted");

        let mut cs = CoreChangeSet::default();
        cs.account_highest_used.insert(
            bip44_account_0(),
            HighestUsedIndexes {
                external: Some(0),
                internal: None,
            },
        );
        assert!(!cs.is_empty(), "highest-used watermark must be persisted");
    }

    /// Merge dedups marked-used entries on `(account_type, pool_type,
    /// index)` — same discipline as `addresses_derived` — and keeps
    /// distinct indices / pools apart.
    #[test]
    fn merge_dedups_marked_used_entries() {
        let acct = bip44_account_0();
        let mut cs = CoreChangeSet {
            addresses_marked_used: vec![stub_marked_used(acct, AddressPoolType::External, 5)],
            ..CoreChangeSet::default()
        };
        cs.merge(CoreChangeSet {
            addresses_marked_used: vec![
                // duplicate of the existing entry — dropped
                stub_marked_used(acct, AddressPoolType::External, 5),
                // same index, different pool — kept
                stub_marked_used(acct, AddressPoolType::Internal, 5),
                // same pool, different index — kept
                stub_marked_used(acct, AddressPoolType::External, 6),
            ],
            ..CoreChangeSet::default()
        });
        assert_eq!(cs.addresses_marked_used.len(), 3);
    }

    /// Highest-used watermarks merge monotonic-max per account per
    /// pool slot: a later batch can only advance a slot, and `None`
    /// never erases a prior `Some`.
    #[test]
    fn merge_highest_used_is_monotonic_max_per_slot() {
        let acct = bip44_account_0();
        let mut cs = CoreChangeSet::default();
        cs.account_highest_used.insert(
            acct,
            HighestUsedIndexes {
                external: Some(5),
                internal: None,
            },
        );
        cs.merge(CoreChangeSet {
            account_highest_used: {
                let mut m = BTreeMap::new();
                m.insert(
                    acct,
                    HighestUsedIndexes {
                        external: Some(2), // lower — must not regress
                        internal: Some(1), // fills the empty slot
                    },
                );
                m
            },
            ..CoreChangeSet::default()
        });
        let merged = cs.account_highest_used[&acct];
        assert_eq!(merged.external, Some(5));
        assert_eq!(merged.internal, Some(1));
    }
}
