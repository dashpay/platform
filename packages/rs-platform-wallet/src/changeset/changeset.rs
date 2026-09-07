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
//! key-wallet exposes core wallet changes as an event bus rather than a
//! changeset type of its own. Platform-wallet subscribes to that bus,
//! projects each event into a `CoreChangeSet`, and routes it through this
//! changeset's `core` slot — so every domain, core included, shares one
//! merge / apply shape downstream consumers can rely on.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::blockdata::transaction::{OutPoint, Transaction};
use dashcore::ephemerealdata::chain_lock::ChainLock;
use dashcore::ephemerealdata::instant_lock::InstantLock;
use dashcore::Txid;

use dash_sdk::platform::address_sync::AddressFunds;
use dpp::prelude::AssetLockProof;
use key_wallet::account::AccountType;
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType};
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::{AddressInfo, Network, PlatformP2PKHAddress, Utxo};

use crate::changeset::identity_scan_state::IdentityScanStateEntry;
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
/// emitted by `WalletManager`. Every field is additive except
/// [`Self::sweeps`]. The merge implementation coalesces the record vecs
/// newest-wins (by txid for the wallet-level `records`, by
/// `(txid, account)` for `account_records` — see
/// [`fold_same_txid_records`]), uses monotonic-max for the height
/// watermarks, `extend` for the utxo vecs and for `sweeps` (in emission
/// order — see the field), and last-write-wins for the IS-lock map.
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
    /// Transaction records produced by this batch — one WALLET-LEVEL
    /// record per txid.
    ///
    /// Includes records first stored (`TransactionDetected`,
    /// `BlockProcessed.inserted`), records whose context advanced
    /// (`BlockProcessed.updated` — e.g. a mempool tx that just confirmed),
    /// and coinbase records that crossed the maturity threshold
    /// (`BlockProcessed.matured`). The event bridge folds a
    /// transaction's per-account slices into a single record whose
    /// `net_amount` / details describe the wallet (see
    /// [`fold_same_txid_records`]), so each record here is a complete
    /// snapshot of its transaction at one observation — merge coalesces
    /// same-txid records newest-wins rather than combining them. All
    /// persisted; the persister's `txid` uniqueness constraint handles
    /// dedup on replay.
    pub records: Vec<TransactionRecord>,

    /// The per-account record SLICES behind [`Self::records`], exactly
    /// as upstream emitted them (one record per matched account,
    /// contact-watch-only slices filtered out).
    ///
    /// The wallet-level fold above is right for the txid-keyed
    /// `transactions` row but destroys account attribution: a sibling
    /// account's `Change` output rides a record whose `account_type`
    /// names the funding account, and `OutputDetail` carries no owning
    /// account. Persisters that route per-account state read the
    /// slices from here instead: the FFI projection buckets
    /// `utxos_added` / `utxos_spent` by each slice's account so
    /// Swift/Kotlin store each TXO under its owning account, and it
    /// emits the folded transaction row into EVERY slice-owning
    /// account's bucket so the per-account transaction callback still
    /// writes the tx↔account involvement join for payload-only
    /// matches (provider owner/voting keys) that restart restoration
    /// depends on. Persisters that resolve accounts another way
    /// (SQLite looks the address up in `core_derived_addresses`) can
    /// ignore this field.
    ///
    /// Merge coalesces by `(txid, account_type)` newest-wins, mirroring
    /// the wallet-level coalesce on `records`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub account_records: Vec<TransactionRecord>,

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

    /// Sweeps this batch carries, in the order the wallet emitted them.
    ///
    /// The one subtractive part of this type. Every other field is additive,
    /// which is exactly why this one has to exist: a persister that only ever
    /// appends keeps the dead rows and replays them on the next load,
    /// re-creating a balance the wallet has already corrected.
    ///
    /// Kept as ordered batches rather than folded into one removal list plus
    /// one release set. Each sweep describes the wallet at the moment it
    /// fired, and those descriptions can disagree: an early sweep frees a
    /// coin, something later spends it, and a later sweep removes that
    /// spender while keeping the coin spent because its own winner took it.
    /// Union the release sets and the first answer outlives the last one that
    /// is actually true. Applied in order, each batch corrects the one before
    /// it, which is what the wallet itself did.
    /// `serde(default)` so a payload written before this field existed still
    /// reads, as an empty vec — the exact backward-compatible meaning, since
    /// a changeset from then could not have carried a sweep.
    ///
    /// Scope of that claim: it holds for SELF-DESCRIBING encodings (JSON and
    /// friends), where a missing field is a fact the decoder can see. It does
    /// NOT hold for a non-self-describing one — bincode, which is what this
    /// workspace persists every stored blob with — where appending a field is
    /// a wire break `default` cannot absorb. That is not a live hazard today:
    /// nothing in-tree serializes a changeset at all (the derive is behind the
    /// optional `serde` feature for out-of-tree consumers), and this note
    /// exists so nobody starts persisting one with bincode believing the
    /// attribute makes it upgrade-safe.
    #[cfg_attr(feature = "serde", serde(default))]
    pub sweeps: Vec<SweepBatch>,
}

/// One `TransactionsSwept` event: the transactions it removed, the
/// transaction that beat them, and the coins its removal actually freed.
///
/// The grouping is what makes ordering expressible. `released_outpoints` is
/// only true relative to the wallet as this event saw it, so it belongs with
/// the removals it came from rather than in a set shared with every other
/// sweep in the batch.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SweepBatch {
    /// The removed transactions. Their rows and every UTXO they created go.
    pub txids: Vec<Txid>,
    /// The transaction whose arrival settled the inputs — final, and
    /// therefore the reason the removed ones can never confirm. Not
    /// necessarily wallet-relevant: it can pay entirely to outside addresses
    /// and still sweep, which is why it cannot be looked up to work out what
    /// it took.
    pub superseded_by: Txid,
    /// Mined height of `superseded_by` when the sweep was triggered by its
    /// arrival in a block; `None` when it was triggered by an
    /// InstantSend-locked winner still waiting to be mined (upstream's only
    /// two triggers — an unlocked mempool arrival never sweeps).
    ///
    /// This is the winner's finality context, straight from the event: the
    /// winner need not be wallet-relevant, so no persister can look its
    /// height up in its own records. A held-but-unfunded input is mirrored
    /// as a durable placeholder in EITHER case; this field decides the
    /// placeholder's lifetime. `Some` stamps the winner's own block height
    /// — the projection of upstream's `observed_spent_outpoints` — and the
    /// placeholder is collectible once `min(chainlock_height,
    /// synced_height)` reaches it, exactly upstream's
    /// `prune_finalized_observed_spends` boundary. `None` (IS-locked
    /// winner, unmined) leaves the placeholder UNSTAMPED and never
    /// collectible: under DIP-10 the lock alone settles the input —
    /// upstream retains it in the account's `spent_outpoints`, a hold with
    /// no height that no record survives to rebuild — and an IS-locked
    /// winner has no mining deadline, so no watermark can ever prove the
    /// funding output delivered-or-never. An unstamped placeholder
    /// resolves only through proof: funding materialisation, a later
    /// block-context sweep's re-stamp, or a release.
    ///
    /// `serde(default)`: a journaled payload written before this field
    /// existed reads back as `None` — the conservative reading (no new
    /// placeholder, existing stamps kept).
    #[cfg_attr(feature = "serde", serde(default))]
    pub winner_mined_height: Option<u32>,
    /// Of the inputs those removed transactions claimed, the ones that came
    /// free — no surviving transaction spends them too. Everything else they
    /// claimed was taken by `superseded_by` and stays spent.
    pub released_outpoints: Vec<OutPoint>,
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

/// Fold same-txid [`TransactionRecord`]s into ONE wallet-level record
/// at the batch seam.
///
/// Upstream `check_core_transaction` emits one record PER MATCHED ACCOUNT
/// for a single transaction, each carrying only its account's slice
/// (`net_amount` is documented "Net amount for this account"). The
/// persisted `transactions` row is keyed by txid alone, so without this
/// fold whichever record drained last defined the row — a multi-account
/// sweep persisted one slice as the whole wallet's net (S22 field case:
/// −0.005 stored for a −2.61920199 spend).
///
/// The fold, per txid group of 2+ records:
/// - `net_amount` — the SUM of the slices: each account's
///   `received − spent` over disjoint detail sets, so the sum is the
///   wallet's `Σreceived − Σspent` by construction.
/// - `input_details` / `output_details` — the union (deduped by input
///   index / output index): the slices are disjoint per account, and the
///   union is exactly the wallet-relevant view downstream consumers
///   (`derive_new_utxos`, usage sweeps) expect of a single record.
/// - `fee` — the first `Some` (only the funding account's record carries
///   one, and disjoint accounts cannot disagree); left `None` when no
///   record knew it.
/// - `direction` — recomputed over the MERGED details with the same rule
///   upstream applies per account (`record_transaction`): `CoinJoin`
///   transaction type wins outright; otherwise no `Sent` output + our
///   inputs + our outputs → `Internal` (a cross-account move whose
///   account-local slices said `Outgoing`/`Incoming` is, wallet-level, a
///   self-transfer); otherwise our inputs → `Outgoing`, else `Incoming`.
///   Deriving from the net's sign instead erased `Internal` and
///   `CoinJoin`: an internal transfer nets −fee and would relabel
///   `Outgoing`.
/// - `context` — the most advanced in the group (`Mempool` <
///   `InstantSend` < `InBlock` < `InChainLockedBlock`), so a group mixing
///   a stale mempool observation with a confirmed one keeps the
///   confirmation.
/// - identity fields (`transaction`, `txid`, `transaction_type`,
///   `label`, `account_type`) — from the FUNDING record (the one with
///   input details) so the row's account attribution names the spender,
///   else the first record.
///
/// Order-preserving for untouched records; a fold lands at the group's
/// FIRST position (`group[0]`) regardless of which record supplied the
/// funding metadata, so unrelated records between two slices never move
/// ahead of the folded transaction. Contact-watch-only records never
/// reach here (filtered at projection — see
/// `core_bridge::is_contact_watch_only`).
pub(crate) fn fold_same_txid_records(records: &mut Vec<TransactionRecord>) {
    use key_wallet::managed_account::transaction_record::{OutputRole, TransactionDirection};
    use key_wallet::transaction_checking::transaction_router::TransactionType;

    if records.len() < 2 {
        return;
    }
    let mut by_txid: BTreeMap<Txid, Vec<usize>> = BTreeMap::new();
    for (i, r) in records.iter().enumerate() {
        by_txid.entry(r.txid).or_default().push(i);
    }
    if by_txid.values().all(|g| g.len() < 2) {
        return;
    }

    let mut drop_idx: BTreeSet<usize> = BTreeSet::new();
    let mut folded: BTreeMap<usize, TransactionRecord> = BTreeMap::new();
    for group in by_txid.values().filter(|g| g.len() >= 2) {
        // Base: the funding record (has input details), else the first.
        let base_pos = group
            .iter()
            .copied()
            .find(|&i| !records[i].input_details.is_empty())
            .unwrap_or(group[0]);
        let mut merged = records[base_pos].clone();
        let mut net: i64 = 0;
        let mut seen_inputs: BTreeSet<u32> = merged.input_details.iter().map(|d| d.index).collect();
        let mut seen_outputs: BTreeSet<u32> =
            merged.output_details.iter().map(|d| d.index).collect();
        for &i in group {
            let r = &records[i];
            net = net.saturating_add(r.net_amount);
            if merged.fee.is_none() {
                merged.fee = r.fee;
            }
            if i != base_pos {
                for d in &r.input_details {
                    if seen_inputs.insert(d.index) {
                        merged.input_details.push(d.clone());
                    }
                }
                for d in &r.output_details {
                    if seen_outputs.insert(d.index) {
                        merged.output_details.push(d.clone());
                    } else if matches!(d.role, OutputRole::Received | OutputRole::Change) {
                        // Index collision across account slices: the slices
                        // are only detail-disjoint for details the accounts
                        // AGREE on. An output owned by account B appears in
                        // funding account A's slice too — as `Sent`, because
                        // A's account-local view cannot attribute B's
                        // address. Keeping the base's entry on collision let
                        // that `Sent` win, and every consumer deriving UTXOs
                        // from the folded record (record_new_utxos_ffi,
                        // derive_new_utxos filter on Received|Change) then
                        // silently dropped the owned output — the store lost
                        // the wallet's own change while the folded net_amount
                        // stayed correct (2026-08-19 device run: records
                        // landed corrected, TXOs never arrived, the reconcile
                        // tripwire healed 4). Ownership is account-scoped
                        // knowledge: exactly one slice can carry
                        // Received/Change for an index, so on collision the
                        // owned role wins unconditionally.
                        if let Some(existing) = merged
                            .output_details
                            .iter_mut()
                            .find(|o| o.index == d.index)
                        {
                            if !matches!(existing.role, OutputRole::Received | OutputRole::Change) {
                                *existing = d.clone();
                            }
                        }
                    }
                }
                drop_idx.insert(i);
            }
            // Context: keep the most advanced observation in the group.
            // The funding slice is not necessarily the newest one — a
            // group can pair a stale `Mempool` sighting with the
            // confirmed snapshot of the same transaction.
            if context_rank(&r.context) > context_rank(&merged.context) {
                merged.context = r.context.clone();
            }
        }
        // The base record's own index was skipped by the `i != base_pos`
        // guard above; drop every group member except the fold's output
        // position (the group's FIRST slot, which the reassembly below
        // fills with the merged record).
        drop_idx.insert(base_pos);
        let first_pos = group[0];
        drop_idx.remove(&first_pos);
        merged.net_amount = net;
        // Wallet-level direction over the merged details — same rule
        // upstream applies per account (see the doc comment). The sign
        // of the net cannot express `Internal` or `CoinJoin`.
        merged.direction = if merged.transaction_type == TransactionType::CoinJoin {
            TransactionDirection::CoinJoin
        } else {
            let has_inputs = !merged.input_details.is_empty();
            let has_sent = merged
                .output_details
                .iter()
                .any(|d| d.role == OutputRole::Sent);
            let has_our_outputs = merged
                .output_details
                .iter()
                .any(|d| matches!(d.role, OutputRole::Received | OutputRole::Change));
            if !has_sent && has_inputs && has_our_outputs {
                TransactionDirection::Internal
            } else if has_inputs {
                TransactionDirection::Outgoing
            } else {
                TransactionDirection::Incoming
            }
        };
        folded.insert(first_pos, merged);
    }

    let old = std::mem::take(records);
    for (i, r) in old.into_iter().enumerate() {
        if let Some(merged) = folded.remove(&i) {
            records.push(merged);
        } else if !drop_idx.contains(&i) {
            records.push(r);
        }
    }
}

/// Replace-or-append fold shared by the record vecs in
/// [`CoreChangeSet`]'s merge: each incoming record either SUPERSEDES the
/// existing record with the same key (in place, keeping the earlier
/// record's position so unrelated records never reorder) or appends.
/// `other` is by the `Merge` contract the later changeset, so incoming
/// records are the newer observations.
///
/// One linear pass over each side per merge — the adapter's drain calls
/// merge once per buffered event, so this deliberately avoids the
/// full-vec re-fold a `fold_same_txid_records` call here would cost.
fn coalesce_newest_wins<K: std::hash::Hash + Eq>(
    existing: &mut Vec<TransactionRecord>,
    incoming: Vec<TransactionRecord>,
    key: impl Fn(&TransactionRecord) -> K,
) {
    use std::collections::hash_map::Entry;
    use std::collections::HashMap;

    if incoming.is_empty() {
        return;
    }
    if existing.is_empty() {
        *existing = incoming;
        return;
    }
    let mut index: HashMap<K, usize> = existing
        .iter()
        .enumerate()
        .map(|(i, r)| (key(r), i))
        .collect();
    for r in incoming {
        match index.entry(key(&r)) {
            Entry::Occupied(slot) => existing[*slot.get()] = r,
            Entry::Vacant(slot) => {
                slot.insert(existing.len());
                existing.push(r);
            }
        }
    }
}

/// Rank a [`TransactionContext`](key_wallet::transaction_checking::TransactionContext)
/// by how far along the confirmation lifecycle the observation is.
/// Used by [`fold_same_txid_records`] so a fold never regresses a
/// confirmed context to a stale mempool one.
fn context_rank(context: &key_wallet::transaction_checking::TransactionContext) -> u8 {
    use key_wallet::transaction_checking::TransactionContext;
    match context {
        TransactionContext::Mempool => 0,
        TransactionContext::InstantSend(_) => 1,
        TransactionContext::InBlock(_) => 2,
        TransactionContext::InChainLockedBlock(_) => 3,
    }
}

impl Merge for CoreChangeSet {
    fn merge(&mut self, other: Self) {
        // Records: coalesce by txid, NEWEST-WINS.
        //
        // The event bridge already folded each event's per-account
        // slices into one wallet-level record per txid (see
        // `fold_same_txid_records` and the `TransactionDetected`
        // rebuild in `core_bridge::build_core_changeset`), so two
        // same-txid records meeting here are the same transaction at
        // two OBSERVATIONS — e.g. a `TransactionDetected` mempool
        // snapshot and its `BlockProcessed.updated` confirmation.
        // Summing those doubled the persisted net (−100 detected +
        // −100 confirmed = −200) and could keep the stale mempool
        // context; the later snapshot simply supersedes the earlier
        // one, at the earlier record's position so unrelated records
        // never reorder around it.
        coalesce_newest_wins(&mut self.records, other.records, |r| r.txid);
        // Account slices: same discipline, keyed by (txid, account) —
        // a slice supersedes the previous observation of the SAME
        // account's slice, while slices of sibling accounts coexist.
        coalesce_newest_wins(&mut self.account_records, other.account_records, |r| {
            (r.txid, r.account_type)
        });
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

        // Sweeps: appended, never folded. Order is the whole point — a later
        // batch's decision to keep a coin spent has to survive an earlier
        // batch's decision to free it, and only replaying them in sequence
        // preserves that.
        self.sweeps.extend(other.sweeps);
    }

    fn is_empty(&self) -> bool {
        self.records.is_empty()
            && self.account_records.is_empty()
            && self.sweeps.is_empty()
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
                    // DPNS names: last-write-wins wholesale, same policy
                    // as `contested_dpns_names` below. Every emitter
                    // snapshots the complete current list via
                    // `from_managed`, and a sold/transferred name must be
                    // able to LEAVE the list — the previous append-only-
                    // by-label merge made departure impossible.
                    existing.dpns_names = entry.dpns_names.clone();
                    // The contested-name sync emits the complete canonical
                    // snapshot. Last-write-wins is therefore required so
                    // resolved contests disappear, including when the latest
                    // snapshot is empty.
                    existing.contested_dpns_names = entry.contested_dpns_names.clone();
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
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::changeset::serde_adapters::optional_asset_lock_proof")
    )]
    pub proof: Option<AssetLockProof>,
}

impl Merge for AssetLockChangeSet {
    fn merge(&mut self, other: Self) {
        // Last write wins, with ONE lifecycle exception: `Consumed` is
        // the terminal state, so a non-Consumed snapshot never replaces
        // a Consumed one. Writers race here — the wallet-event
        // adapter's batched drain can fold (or persist) a stale
        // reconstruction/enrichment snapshot AFTER the live flow's
        // synchronous consumption write — and every non-terminal
        // transition is legitimately bidirectional (a live advance
        // overwrites `RecoveredFromChain`, a defensive resume
        // re-enters `Broadcast`), so terminality is the only ordering
        // the merge can enforce without vetoing real transitions. The
        // durable stores apply the same rule (sqlite upsert guard,
        // swift-sdk `persistAssetLocks`), making the store order of
        // racing snapshots immaterial.
        for (out_point, entry) in other.asset_locks {
            if entry.status == AssetLockStatus::Consumed {
                // A Consumed write supersedes any earlier-folded
                // tombstone for the outpoint — Consumed rows are
                // deliberately retained for historical lookup (see the
                // variant doc), so the terminal write wins over a stale
                // removal exactly as it wins over a stale status.
                self.removed.remove(&out_point);
            } else if let Some(existing) = self.asset_locks.get(&out_point) {
                if existing.status == AssetLockStatus::Consumed {
                    continue;
                }
            }
            self.asset_locks.insert(out_point, entry);
        }
        // Tombstones folded after a Consumed upsert are dropped for the
        // same reason. The only removal emitter (`untrack_asset_lock`)
        // fires exclusively for Built rows whose broadcast was
        // definitively rejected, so a Consumed/removed pair for one
        // outpoint has no legitimate producer — this is defense in
        // depth matching the upsert guard.
        for out_point in other.removed {
            let consumed = self
                .asset_locks
                .get(&out_point)
                .is_some_and(|entry| entry.status == AssetLockStatus::Consumed);
            if !consumed {
                self.removed.insert(out_point);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.asset_locks.is_empty() && self.removed.is_empty()
    }
}

// ---------------------------------------------------------------------------
// DashPay Invitations (DIP-13)
// ---------------------------------------------------------------------------

/// Lifecycle status of an inviter-side invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InvitationStatus {
    /// Created and shared; the funding asset lock is unspent.
    Created,
    /// The voucher was consumed — an identity was registered from it.
    Claimed,
    /// The inviter reclaimed the unspent voucher back into their wallet.
    Reclaimed,
}

/// A single inviter-side invitation record (DIP-13).
///
/// **No secret is stored.** The one-time voucher private key is HD-derived and
/// re-derivable from `funding_index` on demand (for re-packaging or reclaiming an
/// unclaimed invitation); it is never persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InvitationEntry {
    /// The funding asset lock's outpoint (this record's identity).
    pub out_point: OutPoint,
    /// DIP-13 invitation funding index (`m/9'/coin'/5'/3'/<funding_index>'`);
    /// re-derives the voucher key.
    pub funding_index: u32,
    /// Amount locked in the voucher (duffs).
    pub amount_duffs: u64,
    /// Advisory expiry (unix seconds).
    pub expiry_unix: u32,
    /// Unix seconds when the invitation was created.
    pub created_at_secs: u32,
    /// Whether the inviter opted into the contact-bootstrap ("send a request
    /// back to me").
    pub has_inviter: bool,
    /// Current lifecycle status.
    pub status: InvitationStatus,
}

/// Inviter-side invitation records emitted by `create_invitation` (and, later,
/// reclaim + a status sync that flips `Created → Claimed`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InvitationChangeSet {
    /// Invitation records keyed by funding outpoint. Last write wins on merge.
    pub invitations: BTreeMap<OutPoint, InvitationEntry>,
    /// Invitations removed from tracking.
    pub removed: BTreeSet<OutPoint>,
}

impl Merge for InvitationChangeSet {
    fn merge(&mut self, other: Self) {
        // Last write wins — later status is higher finality. `invitations` and
        // `removed` merge independently with no per-key reconciliation, and the
        // sqlite writer applies inserts before deletes, so an outpoint present
        // in both a merged round's insert and remove sets resolves to "removed"
        // (same hazard/mitigation as `IdentityChangeSet`: emit at most one
        // action per key per mutation). The only current emitter,
        // `create_invitation`, is insert-only, so this is latent until reclaim /
        // status-sync emitters land.
        self.invitations.extend(other.invitations);
        self.removed.extend(other.removed);
    }

    fn is_empty(&self) -> bool {
        self.invitations.is_empty() && self.removed.is_empty()
    }
}

// ---------------------------------------------------------------------------
// DPNS name states (username marketplace)
// ---------------------------------------------------------------------------

/// Where a tracked DPNS name currently stands relative to the wallet
/// identity that owned it.
///
/// `Sold` / `Transferred` rows are retained (not deleted) so the host can
/// surface "your name was sold" affordances; hard removal goes through
/// [`DpnsNameStateChangeSet::removed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DpnsNameSaleStatus {
    /// The wallet identity is the document's `$ownerId`.
    Owned,
    /// The name left the identity through a purchase; `to` is the buyer.
    Sold { to: Identifier },
    /// The name left the identity through a plain transfer (gift /
    /// off-market handover); `to` is the recipient.
    Transferred { to: Identifier },
}

/// One tracked DPNS `domain` document belonging to (or recently departed
/// from) a wallet identity, **with sale state** — the marketplace-facing
/// superset of the label-only `DpnsNameInfo` list.
///
/// Deliberately a separate store rather than new fields on
/// [`IdentityEntry`]: the identity `entry_blob` is unversioned positional
/// bincode, so growing `DpnsNameInfo` would break decoding of existing
/// rows. Keyed by the domain document id, which is stable across ownership
/// changes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DpnsNameStateEntry {
    /// The DPNS `domain` document id (this record's identity; stable
    /// across transfers and purchases).
    pub document_id: Identifier,
    /// The wallet identity this row is tracked for. For `Owned` rows this
    /// equals the document's `$ownerId`; for `Sold`/`Transferred` rows it
    /// is the previous owner (ours).
    pub wallet_identity_id: Identifier,
    /// Display label (e.g. "Alice").
    pub label: String,
    /// Homograph-normalized label (e.g. "a11ce").
    pub normalized_label: String,
    /// Normalized parent domain (today always "dash").
    pub normalized_parent_domain_name: String,
    /// Listed sale price in credits (`$price`). `None` = not for sale.
    pub price: Option<Credits>,
    /// Ownership status relative to `wallet_identity_id`.
    pub status: DpnsNameSaleStatus,
    /// Document `$createdAt` (ms since epoch) when the document carries it.
    pub created_at_ms: Option<u64>,
    /// Document `$updatedAt` (ms) — bumps on price changes.
    pub updated_at_ms: Option<u64>,
    /// Document `$transferredAt` (ms) — set on purchase/transfer.
    pub transferred_at_ms: Option<u64>,
    /// Wall-clock ms of the sync pass / confirmed transition that wrote
    /// this row.
    pub last_synced_at_ms: u64,
}

/// DPNS name-state records emitted by the marketplace sync pass and by the
/// set-price / delist / purchase / transfer orchestration ops.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DpnsNameStateChangeSet {
    /// Name states keyed by domain document id. Last write wins on merge —
    /// every emitter writes a complete row read from Platform or from a
    /// confirmed transition, so later rows are strictly fresher.
    pub names: BTreeMap<Identifier, DpnsNameStateEntry>,
    /// Document ids removed from tracking entirely.
    pub removed: BTreeSet<Identifier>,
}

impl Merge for DpnsNameStateChangeSet {
    fn merge(&mut self, other: Self) {
        // Last OPERATION wins per document id, not merely last write.
        //
        // The sqlite writer applies inserts before deletes, so a key
        // landing in both sets resolves to "removed" no matter which
        // operation came first — a stale tombstone would silently
        // swallow a newer upsert. Each side therefore evicts the key
        // from the other as it merges, so the operation that arrived
        // later is the one that survives.
        //
        // Deliberately stricter than `InvitationChangeSet`'s
        // insert-XOR-tombstone convention: a marketplace row can
        // legitimately come back after removal (a name re-acquired
        // later), so the ordering hazard is reachable here rather than
        // latent.
        for document_id in other.names.keys() {
            self.removed.remove(document_id);
        }
        for document_id in &other.removed {
            self.names.remove(document_id);
        }
        self.names.extend(other.names);
        self.removed.extend(other.removed);
    }

    fn is_empty(&self) -> bool {
        self.names.is_empty() && self.removed.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Token Balances
// ---------------------------------------------------------------------------

/// Per-(identity, token) balance changes emitted by
/// [`crate::manager::identity_sync::IdentitySyncManager::sync_now`].
///
/// The watch list itself is not changeset-replicated — it lives
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
///
/// Produced by [`derive_platform_node_public_keys`](crate::wallet::provider_key_at_index::derive_platform_node_public_keys)
/// and fed straight into the managed platform-node pool at registration
/// via [`populate_platform_node_pool`](crate::wallet::provider_key_at_index::populate_platform_node_pool),
/// from which the keys persist as ordinary typed core-address rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProviderPlatformNodePubKey {
    /// Hardened key index within the platform-node pool (`#0..`).
    pub index: u32,
    /// Raw 32-byte Ed25519 public key at this index.
    pub public_key: [u8; 32],
    /// The 20-byte platform node id — `SHA256(ed25519 pubkey)[..20]`
    /// (Tenderdash convention, rust-dashcore #884) of the Ed25519 public
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

/// Snapshot the non-empty address pools of one account into
/// [`AccountAddressPoolEntry`] rows.
///
/// Pool snapshots are whole-pool and last-write-wins on the persistence
/// side, so each emission carries the full pool state; empty pools are
/// dropped so the FFI receiver keeps its "skip empty pools" semantics.
/// Callers pass `account.managed_account_type().address_pools()` — this
/// works for any account shape (`ManagedCoreFundsAccount`,
/// `ManagedAccountRef`, …) since only the resolved pools are needed.
/// Shared by wallet registration, the DashPay registration/payment-rotation
/// path, the identity-top-up account deriver, and the asset-lock
/// funding-index persistence.
pub(crate) fn account_address_pool_entries<'a>(
    account_type: AccountType,
    pools: impl IntoIterator<Item = &'a AddressPool>,
) -> Vec<AccountAddressPoolEntry> {
    pools
        .into_iter()
        .filter_map(|pool| {
            let addresses: Vec<AddressInfo> = pool.addresses.values().cloned().collect();
            if addresses.is_empty() {
                return None;
            }
            Some(AccountAddressPoolEntry {
                account_type,
                pool_type: pool.pool_type,
                addresses,
            })
        })
        .collect()
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
    /// DashPay invitation (DIP-13) records — inviter-side create/reclaim.
    pub invitations: Option<InvitationChangeSet>,
    /// DPNS name states with sale price (username marketplace) — emitted
    /// by the marketplace sync pass and the trade orchestration ops.
    pub dpns_name_states: Option<DpnsNameStateChangeSet>,
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
    /// Verdict of the most recent gap-limit identity scan. Emitted by
    /// discovery and by the startup sequence when it abandons a scan; read on
    /// the next launch to decide whether the warm-launch shortcut may skip
    /// discovery. See [`IdentityScanStateEntry`].
    ///
    /// Durability caveat, the same one [`Self::pending_contact_crypto_added`]
    /// carries: no persister vtable has a slot for this field yet, so on
    /// hosts that have not adopted it the verdict is process-lifetime only.
    /// Within a process it still redirects a second bring-up, and a partial
    /// scan is retried inside its own launch — but honouring the verdict
    /// across launches needs the host slot.
    pub identity_scan_state: Option<IdentityScanStateEntry>,
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
    ///
    /// Durability caveat: the FFI persister vtable (iOS/Android hosts) has
    /// no slot for this field yet, so on those hosts the queue is
    /// process-lifetime only — a restart before a signer-backed drain
    /// loses the entries until the recurring sweep re-discovers and
    /// re-enqueues them (self-healing, but not immediate).
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

impl From<DpnsNameStateChangeSet> for PlatformWalletChangeSet {
    fn from(cs: DpnsNameStateChangeSet) -> Self {
        Self {
            dpns_name_states: Some(cs),
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
        self.invitations.merge(other.invitations);
        self.dpns_name_states.merge(other.dpns_name_states);
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
        // Identity-scan verdict: the later scan's verdict folded over the
        // earlier one, on the rule the manager applies — see
        // `IdentityScanStateEntry::superseding`. Overwriting instead would let
        // a scan batched into the same persist round clear a gap it never
        // probed, which is the whole reason the verdict is recorded.
        if let Some(scan) = other.identity_scan_state {
            self.identity_scan_state = Some(match self.identity_scan_state.take() {
                Some(previous) => scan.superseding(&previous),
                None => scan,
            });
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
            && self.invitations.is_empty()
            && self.dpns_name_states.is_empty()
            && self.token_balances.is_empty()
            && self.dashpay_profiles.as_ref().is_none_or(|m| m.is_empty())
            && self
                .dashpay_payments_overlay
                .as_ref()
                .is_none_or(|m| m.is_empty())
            && self.wallet_metadata.is_none()
            && self.identity_scan_state.is_none()
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

#[cfg(all(test, feature = "serde"))]
mod serde_compat_tests {
    use super::*;

    /// A changeset serialized before `sweeps` existed must still load. The
    /// field postdates the representation, so an older payload simply omits
    /// it — and an empty vec is the exact reading, since nothing back then
    /// could have carried a sweep. Without `serde(default)` the whole
    /// deserialization fails and every pre-sweep payload becomes unreadable.
    #[test]
    fn a_pre_sweep_payload_deserializes_with_no_sweeps() {
        let json = r#"{
            "records": [],
            "spent_utxos": [],
            "new_utxos": [],
            "instant_locks_for_non_final_records": {},
            "last_processed_height": 1000,
            "synced_height": 900,
            "account_highest_used": {},
            "last_applied_chain_lock": null
        }"#;

        let cs: CoreChangeSet =
            serde_json::from_str(json).expect("a pre-sweep payload must still deserialize");
        assert!(cs.sweeps.is_empty());
        assert_eq!(cs.last_processed_height, Some(1000));
        assert_eq!(cs.synced_height, Some(900));
    }

    /// The compat test above only proves a MISSING `sweeps` reads as empty.
    /// This one proves a present one survives the trip at all: `SweepBatch`
    /// carries `Txid` and `OutPoint` from `dashcore`, whose `Serialize` /
    /// `Deserialize` arrive through that crate's own feature wiring — if
    /// that wiring were wrong or absent, every sweep-carrying changeset
    /// would silently fail to round-trip and nothing else here would catch
    /// it.
    #[test]
    fn a_populated_sweep_batch_round_trips() {
        use dashcore::hashes::Hash;

        let loser = Txid::from_byte_array([0x11; 32]);
        let winner = Txid::from_byte_array([0x22; 32]);
        let released = OutPoint::new(Txid::from_byte_array([0x33; 32]), 7);
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser],
                superseded_by: winner,
                winner_mined_height: Some(4_242),
                released_outpoints: vec![released],
            }],
            ..Default::default()
        };

        let encoded = serde_json::to_string(&cs).expect("a sweep-carrying changeset serializes");
        let decoded: CoreChangeSet =
            serde_json::from_str(&encoded).expect("and reads back identically");

        assert_eq!(decoded.sweeps.len(), 1);
        let batch = &decoded.sweeps[0];
        assert_eq!(batch.txids, vec![loser]);
        assert_eq!(batch.superseded_by, winner);
        assert_eq!(batch.winner_mined_height, Some(4_242));
        assert_eq!(batch.released_outpoints, vec![released]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_entry_with_contested(id: Identifier, labels: &[&str]) -> IdentityEntry {
        IdentityEntry {
            id,
            balance: 0,
            revision: 0,
            identity_index: None,
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: Vec::new(),
            contested_dpns_names: labels.iter().map(|label| (*label).to_owned()).collect(),
            status: IdentityStatus::Unknown,
            wallet_id: None,
            dashpay_profile: None,
            dashpay_payments: BTreeMap::new(),
            contact_profiles: BTreeMap::new(),
            ignored_senders: BTreeSet::new(),
        }
    }

    #[test]
    fn test_empty_changeset() {
        let cs = PlatformWalletChangeSet::default();
        assert!(cs.is_empty());
    }

    /// Asset-lock merge is last-write-wins EXCEPT for the Consumed
    /// terminal: when the wallet-event adapter's batched drain folds a
    /// stale reconstruction/enrichment snapshot after (or before) the
    /// live flow's consumption write, the fold must never regress
    /// Consumed — while Consumed itself must still land over anything.
    #[test]
    fn asset_lock_merge_never_regresses_consumed() {
        use dashcore::hashes::Hash;
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        let outpoint = OutPoint {
            txid: Txid::from_byte_array([0x61; 32]),
            vout: 0,
        };
        let entry_with = |status: AssetLockStatus| AssetLockEntry {
            out_point: outpoint,
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
            amount_duffs: 1,
            status,
            proof: None,
        };
        let cs_with = |status: AssetLockStatus| {
            let mut cs = AssetLockChangeSet::default();
            cs.asset_locks.insert(outpoint, entry_with(status));
            cs
        };

        // Stale recovery snapshot folded AFTER the consumption write.
        let mut folded = cs_with(AssetLockStatus::Consumed);
        folded.merge(cs_with(AssetLockStatus::RecoveredFromChain));
        assert_eq!(
            folded.asset_locks[&outpoint].status,
            AssetLockStatus::Consumed,
            "a non-Consumed snapshot must not replace the Consumed terminal"
        );

        // The legitimate direction still lands.
        let mut folded = cs_with(AssetLockStatus::RecoveredFromChain);
        folded.merge(cs_with(AssetLockStatus::Consumed));
        assert_eq!(
            folded.asset_locks[&outpoint].status,
            AssetLockStatus::Consumed
        );

        // Non-terminal transitions stay last-write-wins in both
        // directions (live advances overwrite RecoveredFromChain, and
        // enrichment overwrites Broadcast).
        let mut folded = cs_with(AssetLockStatus::RecoveredFromChain);
        folded.merge(cs_with(AssetLockStatus::ChainLocked));
        assert_eq!(
            folded.asset_locks[&outpoint].status,
            AssetLockStatus::ChainLocked
        );
        let mut folded = cs_with(AssetLockStatus::Broadcast);
        folded.merge(cs_with(AssetLockStatus::RecoveredFromChain));
        assert_eq!(
            folded.asset_locks[&outpoint].status,
            AssetLockStatus::RecoveredFromChain
        );

        // Tombstones obey the same terminal rule. A removal folded
        // after a Consumed entry is dropped…
        let removal = || {
            let mut cs = AssetLockChangeSet::default();
            cs.removed.insert(outpoint);
            cs
        };
        let mut folded = cs_with(AssetLockStatus::Consumed);
        folded.merge(removal());
        assert!(
            folded.removed.is_empty(),
            "a tombstone must not survive over a Consumed entry"
        );
        // …a Consumed entry folded after a tombstone clears it…
        let mut folded = removal();
        folded.merge(cs_with(AssetLockStatus::Consumed));
        assert!(folded.removed.is_empty());
        assert_eq!(
            folded.asset_locks[&outpoint].status,
            AssetLockStatus::Consumed
        );
        // …and a legitimate removal (rejected Built row) still folds.
        let mut folded = cs_with(AssetLockStatus::Built);
        folded.merge(removal());
        assert!(folded.removed.contains(&outpoint));
    }

    #[test]
    fn contested_dpns_merge_replaces_canonical_snapshot_and_allows_empty() {
        let id = Identifier::from([0x51; 32]);
        let mut changes = IdentityChangeSet::default();
        changes
            .identities
            .insert(id, identity_entry_with_contested(id, &["old", "retained"]));

        let mut refreshed = IdentityChangeSet::default();
        refreshed
            .identities
            .insert(id, identity_entry_with_contested(id, &["retained", "new"]));
        changes.merge(refreshed);
        assert_eq!(
            changes.identities[&id].contested_dpns_names,
            ["retained", "new"]
        );

        let mut resolved = IdentityChangeSet::default();
        resolved
            .identities
            .insert(id, identity_entry_with_contested(id, &[]));
        changes.merge(resolved);
        assert!(changes.identities[&id].contested_dpns_names.is_empty());
    }

    fn identity_entry_with_names(id: Identifier, labels: &[&str]) -> IdentityEntry {
        let mut entry = identity_entry_with_contested(id, &[]);
        entry.dpns_names = labels
            .iter()
            .map(|label| DpnsNameInfo {
                label: (*label).to_owned(),
                acquired_at: None,
            })
            .collect();
        entry
    }

    /// DPNS names merge last-write-wins wholesale (same policy as
    /// contested names): a sold/transferred name must be able to LEAVE
    /// the list, including via an empty snapshot. Guards the 2026-08
    /// change away from append-only-by-label, which made departure
    /// impossible.
    #[test]
    fn dpns_names_merge_replaces_canonical_snapshot_and_allows_empty() {
        let id = Identifier::from([0x52; 32]);
        let mut changes = IdentityChangeSet::default();
        changes
            .identities
            .insert(id, identity_entry_with_names(id, &["sold", "kept"]));

        let mut refreshed = IdentityChangeSet::default();
        refreshed
            .identities
            .insert(id, identity_entry_with_names(id, &["kept", "bought"]));
        changes.merge(refreshed);
        let labels: Vec<&str> = changes.identities[&id]
            .dpns_names
            .iter()
            .map(|n| n.label.as_str())
            .collect();
        assert_eq!(labels, ["kept", "bought"]);

        let mut emptied = IdentityChangeSet::default();
        emptied
            .identities
            .insert(id, identity_entry_with_names(id, &[]));
        changes.merge(emptied);
        assert!(changes.identities[&id].dpns_names.is_empty());
    }

    /// Marketplace name-state rows merge LWW per document id, with
    /// tombstones accumulating independently (insert-XOR-tombstone per
    /// mutation round, applied inserts-then-deletes downstream).
    #[test]
    fn dpns_name_state_merge_is_lww_per_document_with_tombstones() {
        let doc = Identifier::from([0x61; 32]);
        let other_doc = Identifier::from([0x62; 32]);
        let identity = Identifier::from([0x63; 32]);
        let buyer = Identifier::from([0x64; 32]);
        let entry = |price: Option<Credits>, status: DpnsNameSaleStatus| DpnsNameStateEntry {
            document_id: doc,
            wallet_identity_id: identity,
            label: "Alice".into(),
            normalized_label: "a11ce".into(),
            normalized_parent_domain_name: "dash".into(),
            price,
            status,
            created_at_ms: Some(1),
            updated_at_ms: None,
            transferred_at_ms: None,
            last_synced_at_ms: 2,
        };

        let mut cs = DpnsNameStateChangeSet::default();
        assert!(cs.is_empty());
        cs.names
            .insert(doc, entry(Some(5_000), DpnsNameSaleStatus::Owned));

        let mut sold = DpnsNameStateChangeSet::default();
        sold.names
            .insert(doc, entry(None, DpnsNameSaleStatus::Sold { to: buyer }));
        sold.removed.insert(other_doc);
        cs.merge(sold);

        assert_eq!(cs.names[&doc].price, None);
        assert_eq!(
            cs.names[&doc].status,
            DpnsNameSaleStatus::Sold { to: buyer }
        );
        assert!(cs.removed.contains(&other_doc));
        assert!(!cs.is_empty());

        // Tombstone AFTER an upsert: the later remove wins and the
        // superseded upsert does not linger in `names`.
        let mut upsert_then_remove = DpnsNameStateChangeSet::default();
        upsert_then_remove
            .names
            .insert(doc, entry(Some(1), DpnsNameSaleStatus::Owned));
        let mut tombstone = DpnsNameStateChangeSet::default();
        tombstone.removed.insert(doc);
        upsert_then_remove.merge(tombstone);
        assert!(!upsert_then_remove.names.contains_key(&doc));
        assert!(upsert_then_remove.removed.contains(&doc));

        // Upsert AFTER a tombstone (a name re-acquired later): the newer
        // upsert wins and the stale tombstone is dropped. Without the
        // eviction this row would be silently deleted, because the
        // sqlite writer applies inserts before deletes.
        let mut remove_then_upsert = DpnsNameStateChangeSet::default();
        remove_then_upsert.removed.insert(doc);
        let mut reacquired = DpnsNameStateChangeSet::default();
        reacquired
            .names
            .insert(doc, entry(Some(7), DpnsNameSaleStatus::Owned));
        remove_then_upsert.merge(reacquired);
        assert!(!remove_then_upsert.removed.contains(&doc));
        assert_eq!(remove_then_upsert.names[&doc].price, Some(7));

        // Replaying the same round is idempotent.
        let mut replayed = remove_then_upsert.clone();
        replayed.merge(remove_then_upsert.clone());
        assert_eq!(replayed.names, remove_then_upsert.names);
        assert_eq!(replayed.removed, remove_then_upsert.removed);
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
        use key_wallet::managed_account::address_pool::{AddressInfo, AddressState, PublicKeyType};

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
                state: AddressState::Used,
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
