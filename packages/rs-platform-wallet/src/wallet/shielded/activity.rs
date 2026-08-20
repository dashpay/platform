//! Derived shielded-activity log: a user-facing history of shielded
//! operations.
//!
//! The wallet already persists the *raw* shielded materials — own
//! received notes ([`ShieldedNote`]) and OVK-recovered sends
//! ([`ShieldedOutgoingNote`]) — but nothing that reads as an
//! operation-level activity feed ("you sent 0.5 DASH", "you created an
//! identity"). This module derives that feed:
//!
//! - **Live recorder** (the rich path): each of the six shielded
//!   operation call sites records a [`ShieldedActivityEntry`] at
//!   execution time, when the operation type, amounts, fee, recipient,
//!   memo, spent notes, and created-identity id are all known exactly.
//!   See `operations.rs` / `fund_from_asset_lock.rs`.
//! - **Scan deriver** (the restore path): on a wallet that upgraded
//!   with existing note/outgoing-note state (or whose live entries were
//!   lost), [`derive_activity_from_scan_data`] reconstructs entries
//!   best-effort from the data the wallet already holds — clustering
//!   own notes, OVK sends, and spends by block height. Classification is
//!   100% client-side (Option B): it queries nothing new (no node/DAPI,
//!   no note→transition index).
//!
//! # The dedupe contract (READ THIS BEFORE TOUCHING [`ShieldedActivityEntry::id`])
//!
//! Both paths key an entry by [`ShieldedActivityEntry::id`] =
//! `sha256(sorted visible output cmxs)`. "Visible output cmxs" means the
//! commitments of the outputs the WALLET CAN SEE for that bundle:
//! - the wallet's own received notes ([`ShieldedNote::cmx`]), AND
//! - the wallet's OVK-recovered sends ([`ShieldedOutgoingNote::cmx`]).
//!
//! Dummy / unrecoverable anonymity-set fillers are excluded (the wallet
//! can't see their cmx as a note or an outgoing note). The live recorder
//! computes the id over exactly this same set (own note cmxs of the
//! built bundle + the recovered/known output cmxs), so when a later
//! rescan re-derives the same cluster it produces the *same* id and the
//! persister upsert collapses them into one row — the live entry's rich
//! classification wins, the scan entry is dropped.
//!
//! Compute the id ONLY via [`compute_activity_id`] on both sides so the
//! sort-then-hash stays identical.

use std::collections::BTreeMap;

use grovedb_commitment_tree::{IncomingViewingKey, PaymentAddress};

use crate::wallet::shielded::store::{ShieldedNote, ShieldedOutgoingNote};

/// How an entry's net direction reads relative to the wallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ShieldedDirection {
    /// Value entered the wallet's shielded pool (Shield / Received).
    In,
    /// Value left the wallet's shielded pool to another party
    /// (Sent / Unshield / Withdrawal / IdentityCreate).
    Out,
    /// A wallet-internal move with no net party change (e.g. a residual
    /// spend whose outputs are all self-change).
    SelfTransfer,
}

/// Confirmation status of a recorded activity entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ShieldedActivityStatus {
    /// Built + broadcast, not yet confirmed on chain.
    Pending,
    /// Observed confirmed (the live flip after `broadcast_and_wait`, or
    /// a scan that matched the cluster).
    Confirmed,
    /// The operation definitively failed (broadcast rejected on merits).
    Failed,
}

/// The classified kind of a shielded operation.
///
/// Designed to be **upgradeable in place**: a rescan or a later
/// correlation pass may refine [`ShieldedActivityKind::ShieldedSpend`]
/// (the unclassified residual) into a specific kind. The persister
/// upserts by [`ShieldedActivityEntry::id`], so re-emitting the same id
/// with a sharper `kind` updates the existing row rather than
/// duplicating it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ShieldedActivityKind {
    /// Type 15: transparent platform addresses → shielded pool.
    Shield,
    /// Type 18: Core L1 asset lock → shielded pool.
    ShieldFromAssetLock,
    /// Inbound shielded note(s) with no own spend in the cluster. On the
    /// restore path a Shield is indistinguishable from a third-party
    /// receive, so both surface as `Received` (honest by construction).
    Received,
    /// Type 16: shielded pool → another Orchard address (a private send).
    Sent,
    /// Type 17: shielded pool → transparent platform address.
    Unshield,
    /// Type 19: shielded pool → Core L1 address.
    Withdrawal,
    /// Type 20: shielded pool → a brand-new Platform identity.
    IdentityCreate {
        /// The created identity's id (32 bytes).
        identity_id: [u8; 32],
    },
    /// Own spend whose outputs are all self-change and which no
    /// correlation arm could refine. The honest residual on the restore
    /// path. Carries `fee: None` because the exact fee is underivable
    /// from scan data alone.
    ShieldedSpend,
}

impl ShieldedActivityKind {
    /// A stable discriminant byte for the FFI / persistence layer.
    ///
    /// Kept explicit (rather than relying on `as u8`) so adding a
    /// variant in the middle can't silently renumber a persisted kind.
    pub fn tag(&self) -> u8 {
        match self {
            ShieldedActivityKind::Shield => 0,
            ShieldedActivityKind::ShieldFromAssetLock => 1,
            ShieldedActivityKind::Received => 2,
            ShieldedActivityKind::Sent => 3,
            ShieldedActivityKind::Unshield => 4,
            ShieldedActivityKind::Withdrawal => 5,
            ShieldedActivityKind::IdentityCreate { .. } => 6,
            ShieldedActivityKind::ShieldedSpend => 7,
        }
    }
}

/// One shielded operation, as a user-facing activity record.
///
/// See the module docs for the [`Self::id`] dedupe contract and the
/// live-vs-scan recording split.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShieldedActivityEntry {
    /// `sha256(sorted visible output cmxs)` — the dedupe key shared by
    /// the live recorder and the scan deriver. See the module docs.
    pub id: [u8; 32],
    /// The classified operation kind. Upgradeable in place (see
    /// [`ShieldedActivityKind`]).
    pub kind: ShieldedActivityKind,
    /// Net direction relative to the wallet.
    pub direction: ShieldedDirection,
    /// Display amount in credits: the operation principal, excluding
    /// self-change outputs and zero-value fillers. For a send this is the
    /// value paid to the counterparty; for a receive/shield the value
    /// received.
    pub amount: u64,
    /// Exact fee in credits when derivable (live entries:
    /// `spent − outputs − change`); `None` when underivable (restored
    /// `ShieldedSpend`).
    pub fee: Option<u64>,
    /// Counterparty bytes, kind-dependent:
    /// - `Sent`: 43-byte raw Orchard address.
    /// - `Unshield`: 21-byte serialized `PlatformAddress`.
    /// - `Withdrawal`: Core output script / address bytes.
    /// `None` for self / receive / shield kinds.
    pub counterparty: Option<Vec<u8>>,
    /// 36-byte `DashMemo` when present and non-zero; `None` otherwise.
    pub memo: Option<Vec<u8>>,
    /// Block height the operation confirmed at. `None` while pending,
    /// when the live confirm couldn't read it from the proof metadata,
    /// or — permanently — on scan-derived (restored) entries: the
    /// note-fetch proof carries no per-note inclusion height, so the
    /// real height is unknowable client-side and is modeled as absent
    /// rather than stamped with the discovering batch's proof-anchor
    /// (scan-tip) height, which two devices would disagree on.
    /// **Canonical sort key** (desc) — see [`sort_activity_for_display`]
    /// for how the `None` bands order.
    pub block_height: Option<u64>,
    /// Confirmation status.
    pub status: ShieldedActivityStatus,
    /// `SystemTime` (ms since epoch) at record time for live-recorded
    /// entries. **`0` = unknown**: scan-derived (restored) entries have
    /// no wall-clock provenance — the chain data carries no per-note
    /// block time — so they carry the honest sentinel instead of the
    /// scan moment (which dated weeks-old restored history "today" and
    /// differed between devices). Display-only and a sort tiebreak;
    /// never the primary sort key. Hosts must render `0` as an unknown
    /// date, never as the epoch.
    pub created_at_ms: u64,
    /// Chain-order key for scan-derived (restored) entries whose date
    /// and height are unknowable: the smallest commitment-tree position
    /// among the entry's own received notes. Tree positions are
    /// append-only chain order, so sorting by this reproduces the exact
    /// on-chain sequence of otherwise-undatable history — identically
    /// on every device. `None` on live-recorded entries (which carry a
    /// real record time instead) and on the rare outgoing-only cluster
    /// (OVK-recovered sends don't persist a position).
    #[cfg_attr(feature = "serde", serde(default))]
    pub min_note_position: Option<u64>,
    /// cmxs of the visible outputs that fed [`Self::id`] (own notes +
    /// recovered sends). Linkage for status confirmation and dedupe.
    /// Stored as `Vec<[u8;32]>` (serde derive covers `[u8;32]`).
    pub note_cmxs: Vec<[u8; 32]>,
    /// Nullifiers of the notes this operation spent (empty for inbound
    /// kinds). Linkage for confirmation and dedupe.
    pub spent_nullifiers: Vec<[u8; 32]>,
}

impl ShieldedActivityEntry {
    /// Current wall-clock time in milliseconds since the Unix epoch.
    ///
    /// Display-only (`created_at_ms`); a clock that's briefly behind
    /// only perturbs the sort tiebreak, never the `block_height`-first
    /// ordering, so a non-monotonic read is harmless here.
    pub fn now_ms() -> u64 {
        crate::util::now_ms()
    }
}

/// Compute the dedupe id over a set of visible output cmxs.
///
/// **The single source of truth for [`ShieldedActivityEntry::id`].** Both
/// the live recorder and the scan deriver MUST call this so the
/// sort-then-hash is byte-identical and a later rescan dedupes against
/// the live entry. Duplicate cmxs are de-duplicated before hashing so a
/// note that appears as both a received note and (defensively) an
/// outgoing note can't perturb the id.
///
/// An empty set hashes to `sha256("")` — a degenerate but stable id;
/// callers that could produce an empty visible set (a fully-dummy
/// bundle) should not record an entry at all.
pub fn compute_activity_id(cmxs: &[[u8; 32]]) -> [u8; 32] {
    use dpp::dashcore::hashes::{sha256, Hash, HashEngine};

    let mut sorted: Vec<[u8; 32]> = cmxs.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    let mut engine = sha256::Hash::engine();
    for cmx in &sorted {
        engine.input(cmx);
    }
    sha256::Hash::from_engine(engine).to_byte_array()
}

/// The set of events the scan deriver clusters by block height for one
/// subwallet, gathered from the data the wallet already persists.
///
/// Pure inputs — built from [`ShieldedStore::get_all_notes`] and
/// [`ShieldedStore::get_outgoing_notes`] by the caller — so the
/// clustering + classification logic stays unit-testable without a live
/// store or network.
///
/// [`ShieldedStore::get_all_notes`]: crate::wallet::shielded::ShieldedStore::get_all_notes
/// [`ShieldedStore::get_outgoing_notes`]: crate::wallet::shielded::ShieldedStore::get_outgoing_notes
#[derive(Debug, Clone, Default)]
pub struct ScanDeriveInput {
    /// All known notes (spent + unspent) for the subwallet.
    pub notes: Vec<ShieldedNote>,
    /// OVK-recovered outgoing (sent) notes for the subwallet.
    pub outgoing: Vec<ShieldedOutgoingNote>,
    /// The wallet's own Orchard addresses (43-byte raw bytes), used to
    /// tell "sent to someone else" from "sent to self / change". Pass
    /// every diversified address the subwallet can recognize; an
    /// outgoing note whose recipient is in this set is self-change.
    pub own_addresses: Vec<Vec<u8>>,
}

/// Output of [`derive_activity_from_scan_data`].
#[derive(Debug, Clone, Default)]
pub struct DerivedActivity {
    /// Entries for clusters with no existing row — new history.
    pub new_entries: Vec<ShieldedActivityEntry>,
    /// `(entry_id, observed_at_height)` sightings for clusters whose id
    /// already has a row: the cluster was observed on-chain **at or
    /// before** that height (the discovering batch's proof-anchor
    /// height — an upper bound, not the inclusion height, which the
    /// note-fetch proof doesn't carry). The caller upgrades a
    /// still-`Pending` (or height-less) stored row to `Confirmed` at
    /// the height — preserving the live entry's richer fields — and
    /// ignores sightings for rows that are already confirmed. For the
    /// live flows this band serves, the pass runs near-tip so the bound
    /// is within a few blocks of inclusion; scan-derived NEW entries
    /// never carry it (see [`derive_activity_from_scan_data`]).
    pub confirmations: Vec<([u8; 32], u64)>,
}

/// One height-keyed cluster of a subwallet's shielded events.
///
/// The key is NOT a per-note mined height — the proven note data carries
/// none. It is the fetch chunk's proof anchor height (the chain tip the
/// response was proven at), stamped per-batch on incoming and outgoing
/// notes alike, so "same height" really means "surfaced by the same
/// fetch batch".
///
/// Documented limitation (acceptable for v1): events that share a batch
/// merge into one cluster — the anchor height is the only join key
/// available client-side without a note→transition index (which Option B
/// forbids). For live syncing this approximates "same block" well (a
/// near-tip pass fetches few new notes per chunk), but on a cold restore
/// one chunk can span many historical blocks of unrelated operations,
/// which then merge into a single synthetic entry with combined
/// amount/kind/memo. Live-recorded entries are protected from this by
/// the cmx-overlap dedupe; only restore-derived history aggregates. The
/// real fix needs a per-note mined height in the note-fetch proof
/// (node-side change).
#[derive(Debug, Clone, Default)]
struct HeightCluster {
    /// Own new notes that first appeared at this height.
    received: Vec<ShieldedNote>,
    /// OVK-recovered sends at this height.
    outgoing: Vec<ShieldedOutgoingNote>,
    // NOTE: no `spent` field. A note's stored `block_height` is its
    // *receipt* height; scan-based spend detection flips `is_spent`
    // without recording the spend's height (Option B forbids querying
    // it), so spends can't be height-clustered from note data alone.
    // See [`cluster_events`] for how that bounds classification.
}

/// Whether an outgoing note's recipient is one of the wallet's own
/// addresses (i.e. self-change rather than a payment out).
fn is_own_recipient(recipient: &[u8], own_addresses: &[Vec<u8>]) -> bool {
    own_addresses.iter().any(|a| a.as_slice() == recipient)
}

/// Extract a note's `rho` from its serialized `note_data`
/// (`recipient(43) || value(8 LE) || rho(32) || rseed(32)`, 115 bytes —
/// the layout documented on [`ShieldedNote::note_data`]).
///
/// `rho` is the nullifier of the ACTION that created the note: when one
/// of our new notes has `rho` equal to the nullifier of one of our own
/// (now spent) notes, that new note is provably the change of our own
/// spend — the strongest classification signal available client-side.
///
/// The link is zero-false-positive but PROBABILISTIC in recall: Orchard's
/// builder shuffles spends and outputs independently before pairing them
/// into actions (`indexed_spends.shuffle` / `indexed_outputs.shuffle` in
/// `orchard::builder`), so in a 2-action bundle the change output sits in
/// the same action as the real spend only ~half the time. A missed link
/// degrades to the honest `ShieldedSpend`/self-transfer arm — never to a
/// wrong `Received`.
fn note_rho(note_data: &[u8]) -> Option<[u8; 32]> {
    if note_data.len() != 115 {
        return None;
    }
    let mut rho = [0u8; 32];
    rho.copy_from_slice(&note_data[51..83]);
    Some(rho)
}

/// Cluster a subwallet's events by stored `block_height`.
///
/// Receipts and sends carry a `block_height` from the scan — the fetch
/// chunk's proof ANCHOR height, not a per-note mined height (none
/// exists in the proven data), stamped per-batch on BOTH sides (see
/// `ShieldedNote::block_height`) so a bundle's incoming change and
/// OVK-recovered send agree on the key and cluster together. "Same
/// height" therefore means "same fetch batch"; see [`HeightCluster`]
/// for the cold-restore aggregation this implies. Spends are the hard
/// case: a note's stored
/// `block_height` is its *receipt* height, and scan-based spend
/// detection flips `is_spent` without recording *when* it was spent
/// (the spend height isn't persisted anywhere the wallet layer can read
/// — Option B forbids querying it). So a spent own note can only be
/// attributed to its own receipt-height cluster, which is wrong for the
/// spend. We therefore DO NOT try to height-attribute spends here; the
/// classifier uses the per-cluster receive/send shape plus the global
/// "did this subwallet spend anything" signal. This is the v1
/// limitation called out in the task: spend-only residuals can't be
/// height-correlated client-side, so they fall to `ShieldedSpend`.
fn cluster_events(input: &ScanDeriveInput) -> BTreeMap<u64, HeightCluster> {
    let mut clusters: BTreeMap<u64, HeightCluster> = BTreeMap::new();
    for note in &input.notes {
        clusters
            .entry(note.block_height)
            .or_default()
            .received
            .push(note.clone());
    }
    for out in &input.outgoing {
        clusters
            .entry(out.block_height)
            .or_default()
            .outgoing
            .push(out.clone());
    }
    clusters
}

/// Reconstruct activity entries best-effort from a subwallet's persisted
/// scan data. The restore-path counterpart to the live recorder.
///
/// Classification (client-side only — Option B), in priority order:
/// - cluster has OVK outgoing note(s) to a NON-own address → [`Sent`]
///   (counterparty / value / memo from the outgoing notes; own receipts
///   are the change and excluded from the amount). The amount is the SUM
///   over every external output, so a multi-output transfer yields one
///   aggregate row; its `counterparty` / `memo` are filled only when all
///   those outputs agree (see [`unanimous_bytes`]), matching the live
///   `transfer_multi` recorder. When the rho linkage (see [`note_rho`])
///   identifies the consumed note(s), the exact fee (spent − sent −
///   change) is recovered too.
/// - rho-linked cluster with no external recipient → [`ShieldedSpend`]
///   with direction `Out` and the exact amount that left the pool
///   (spent − change). This is provably our own spend (unshield /
///   withdrawal / identity-create — the type itself isn't recoverable
///   client-side).
/// - own new note(s) with NO OVK pairing at all → [`Received`]. A true
///   third-party receive has exactly this shape (we never hold the
///   sender's OVK), so this arm is reliable.
/// - self-pay cluster without a rho link → [`ShieldedSpend`] with
///   direction `SelfTransfer`: a shield-to-self and an unlinked change
///   (the ~50% shuffle miss) are indistinguishable, and labeling either
///   `Received` would misrepresent a spend as money arriving.
///
/// **Correlation arms a/b/c are NOT implemented here** — see the crate
/// report. The data they'd need (per-identity registration height,
/// per-height platform-address credits, locally-indexed withdrawal docs)
/// is not reachable in `rs-platform-wallet` today, and Option B forbids
/// adding node queries to get it. Those arms remain TODOs; the residual
/// stays `ShieldedSpend`, which a later live entry (or a future
/// correlation pass) can upgrade in place via the shared id.
///
/// `existing_cmxs` maps every stored entry's visible output cmx to the
/// id of the entry that owns it (live entries win). Dedupe is by cmx
/// OVERLAP, not exact-id equality: a cluster whose visible cmxs intersect
/// any stored entry's cmx set produces no NEW entry (so a live
/// `IdentityCreate` / `Unshield` / `Withdrawal` is never clobbered by a
/// coarser scan-derived `Sent` / `ShieldedSpend`), and instead emits a
/// [`DerivedActivity::confirmations`] observation for EACH overlapped id.
/// The ambiguous post-broadcast paths leave their live row `Pending` with
/// the explicit promise that a later scan finding the cluster on-chain
/// flips it to `Confirmed` at the observed height (the caller performs
/// that upgrade against the stored row so the live entry's richer fields
/// survive).
///
/// Overlap subsumes the exact-id case AND handles the same-block merge
/// hazard: when one cluster merges two live ops (cmx sets A and B) its
/// computed id `H(A∪B)` matches neither live id, but the overlap catches
/// both and forgoes synthesizing a spurious aggregate row. The
/// conservative trade: a same-block mix of an owned-op and an unrelated
/// receive forgoes the receive's own row (it folds into the confirmation
/// for the owned op) — acceptable to avoid both clobber and aggregates.
///
/// [`Sent`]: ShieldedActivityKind::Sent
/// [`Received`]: ShieldedActivityKind::Received
/// [`ShieldedSpend`]: ShieldedActivityKind::ShieldedSpend
pub fn derive_activity_from_scan_data(
    input: &ScanDeriveInput,
    existing_cmxs: &std::collections::BTreeMap<[u8; 32], [u8; 32]>,
) -> DerivedActivity {
    let clusters = cluster_events(input);
    let mut out = DerivedActivity::default();
    // Scan-derived entries deliberately carry NO height and NO
    // timestamp (dashwallet-ios restored-history acceptance criteria):
    //
    // - `block_height: None` — the cluster key is the discovering
    //   batch's proof-anchor height (the tip the response was proven
    //   at), NOT the operation's inclusion height. The note-fetch proof
    //   carries no per-note mined height, nullifier items are stored
    //   with empty values, and the on-chain anchors-by-height index is
    //   pruned to a recent window — so for restored history the real
    //   height is unknowable client-side. Stamping the anchor height
    //   dated the same entry at two different heights on two devices
    //   restoring the same wallet; absence is the honest value.
    // - `created_at_ms: 0` (unknown) — chain data carries no per-note
    //   block time either, and stamping the scan clock grouped
    //   weeks-old restored transfers under "today". Live-recorded
    //   entries keep their genuine record time; only this
    //   reconstruction path uses the sentinel.
    //
    // Both values are device-independent, so two restores of the same
    // wallet produce byte-identical entries for the same id. The
    // cluster height is still used ABOVE as the batch grouping key and
    // in `confirmations` (an "observed at-or-before" bound for
    // upgrading live pending rows) — it just never masquerades as an
    // inclusion height on a derived entry.

    // Own-nullifier → (value, cmx) lookup for the rho linkage: a cluster
    // note whose `rho` is one of our own nullifiers is provably the
    // change of our own spend, and the matched note is what we spent.
    let own_nullifiers: BTreeMap<[u8; 32], (u64, [u8; 32])> = input
        .notes
        .iter()
        .map(|n| (n.nullifier, (n.value, n.cmx)))
        .collect();

    // Global "did this subwallet ever spend anything" signal. A self-pay
    // cluster can only be change-from-a-spend if SOME own note was spent;
    // a wallet with zero spent notes can only have produced it by
    // shielding to itself, so the self-pay ambiguity collapses to a
    // receive in that case.
    let wallet_has_any_spend = input.notes.iter().any(|n| n.is_spent);

    for (height, cluster) in clusters {
        // Visible output cmxs for this cluster: own received-note cmxs +
        // recovered-send cmxs. This is exactly the set the live recorder
        // hashes, so the ids line up for dedupe.
        let mut visible_cmxs: Vec<[u8; 32]> = Vec::new();
        for n in &cluster.received {
            visible_cmxs.push(n.cmx);
        }
        for o in &cluster.outgoing {
            // `ShieldedOutgoingNote::cmx` is a fixed `[u8; 32]` (the cmx is
            // always 32 bytes; only the 43-byte recipient/36-byte memo
            // needed the `Vec` serde workaround).
            visible_cmxs.push(o.cmx);
        }
        if visible_cmxs.is_empty() {
            continue;
        }
        let id = compute_activity_id(&visible_cmxs);
        // Exact chain-order key: tree positions are append-only chain
        // order, and the received notes carry theirs. Outgoing-only
        // clusters (no persisted position on OVK-recovered sends)
        // honestly report None.
        let min_note_position = cluster.received.iter().map(|n| n.position).min();
        // Overlap-based dedupe: any stored entry whose visible cmx set
        // intersects this cluster's cmxs already owns (part of) the
        // cluster. `BTreeSet` so each overlapped id is reported once even
        // when several of the cluster's cmxs map to the same entry.
        let overlapping: std::collections::BTreeSet<[u8; 32]> = visible_cmxs
            .iter()
            .filter_map(|c| existing_cmxs.get(c))
            .copied()
            .collect();
        if !overlapping.is_empty() {
            // A live entry (or an earlier scan) already owns this cluster
            // (exactly, or as one of a same-block merge). Don't synthesize
            // a (coarser, or spuriously aggregate) duplicate — but DO
            // report the on-chain sighting for EACH overlapped id so the
            // caller can flip a still-`Pending` row (the ambiguous
            // post-broadcast paths) to Confirmed at this height.
            for entry_id in overlapping {
                out.confirmations.push((entry_id, height));
            }
            continue;
        }

        // Partition outgoing notes into external payments vs self-change.
        let (external, _self_change): (Vec<&ShieldedOutgoingNote>, Vec<&ShieldedOutgoingNote>) =
            cluster
                .outgoing
                .iter()
                .partition(|o| !is_own_recipient(&o.recipient, &input.own_addresses));

        // Rho linkage: a cluster note whose rho is one of OUR nullifiers
        // is provably change from our own spend, and tells us exactly
        // which note (value) was consumed. Exclude self-matches (a note's
        // rho can't be its own nullifier, but a merged cluster could pair
        // a note with a sibling created in the same block — require the
        // matched spent note to be a different cmx).
        let mut linked_spent_total: u64 = 0;
        let mut linked_nullifiers: Vec<[u8; 32]> = Vec::new();
        for n in &cluster.received {
            if let Some(rho) = note_rho(&n.note_data) {
                if let Some((spent_value, spent_cmx)) = own_nullifiers.get(&rho) {
                    if *spent_cmx != n.cmx {
                        linked_spent_total = linked_spent_total.saturating_add(*spent_value);
                        linked_nullifiers.push(rho);
                    }
                }
            }
        }
        let change_total: u64 = cluster.received.iter().map(|n| n.value).sum();

        let entry = if !external.is_empty() {
            // SENT: at least one outgoing note to an address we don't own.
            // Amount = sum of external sends; own receipts in the cluster
            // are the change and are excluded. When the rho linkage
            // identified the consumed note(s), the exact fee falls out:
            // spent − sent − change.
            //
            // A multi-output transfer (Type 16 paying several recipients
            // in one transition) restores as ONE such cluster, so
            // `counterparty` and `memo` go through the shared
            // [`unanimous_bytes`] rule: recorded only when every external
            // output agrees, `None` otherwise. That is the same rule the
            // live `transfer_multi` recorder applies — literally the same
            // function — so a restored row names a recipient exactly when
            // the live row would. Copying `external.first()` here instead
            // would pin the whole aggregate amount on one of several
            // distinct recipients.
            let amount: u64 = external.iter().map(|o| o.value).sum();
            let fee = (!linked_nullifiers.is_empty())
                .then(|| {
                    linked_spent_total
                        .checked_sub(amount)
                        .and_then(|r| r.checked_sub(change_total))
                })
                .flatten();
            ShieldedActivityEntry {
                id,
                kind: ShieldedActivityKind::Sent,
                direction: ShieldedDirection::Out,
                amount,
                fee,
                counterparty: unanimous_bytes(external.iter().map(|o| o.recipient.as_slice())),
                memo: unanimous_bytes(external.iter().map(|o| o.memo.as_slice()))
                    .and_then(|m| non_zero_memo(&m)),
                block_height: None,
                status: ShieldedActivityStatus::Confirmed,
                created_at_ms: 0,
                min_note_position,
                note_cmxs: visible_cmxs,
                spent_nullifiers: linked_nullifiers,
            }
        } else if !linked_nullifiers.is_empty() {
            // LINKED SPEND: no external recipient, but the rho linkage
            // proves our own note(s) were consumed and this cluster's
            // notes are the change. Amount = value that left the pool
            // (spent − change; includes the fee, which can't be split out
            // without knowing the transition type). An unshield /
            // withdrawal / identity-create on a restored wallet surfaces
            // here; a later correlation or live entry can refine the kind.
            ShieldedActivityEntry {
                id,
                kind: ShieldedActivityKind::ShieldedSpend,
                direction: ShieldedDirection::Out,
                amount: linked_spent_total.saturating_sub(change_total),
                fee: None,
                counterparty: None,
                memo: None,
                block_height: None,
                status: ShieldedActivityStatus::Confirmed,
                created_at_ms: 0,
                min_note_position,
                note_cmxs: visible_cmxs,
                spent_nullifiers: linked_nullifiers,
            }
        } else if cluster.outgoing.is_empty() {
            // RECEIVED: own new notes with no OVK pairing at all. A true
            // third-party receive has exactly this shape (we never hold
            // the sender's OVK), so `Received` is reliable here.
            ShieldedActivityEntry {
                id,
                kind: ShieldedActivityKind::Received,
                direction: ShieldedDirection::In,
                amount: change_total,
                fee: None,
                counterparty: None,
                memo: None,
                block_height: None,
                status: ShieldedActivityStatus::Confirmed,
                created_at_ms: 0,
                min_note_position,
                note_cmxs: visible_cmxs,
                spent_nullifiers: Vec::new(),
            }
        } else if !wallet_has_any_spend {
            // SELF-PAY in a subwallet that has NEVER spent a note: change
            // from a spend is impossible, so this is necessarily a shield
            // to self — surface it as the inbound value it is.
            ShieldedActivityEntry {
                id,
                kind: ShieldedActivityKind::Received,
                direction: ShieldedDirection::In,
                amount: change_total,
                fee: None,
                counterparty: None,
                memo: None,
                block_height: None,
                status: ShieldedActivityStatus::Confirmed,
                created_at_ms: 0,
                min_note_position,
                note_cmxs: visible_cmxs,
                spent_nullifiers: Vec::new(),
            }
        } else {
            // SELF-PAY, UNLINKED, in a wallet that HAS spends: two
            // indistinguishable possibilities — a shield to self (Type
            // 15/18) or change from our own spend whose output landed in a
            // different action than the real spend (the ~50% shuffle miss;
            // see `note_rho`). Calling it `Received` would show a spend as
            // money arriving, so this is surfaced as a self-transfer
            // instead — honest for both readings and upgradeable in place
            // by a live entry.
            ShieldedActivityEntry {
                id,
                kind: ShieldedActivityKind::ShieldedSpend,
                direction: ShieldedDirection::SelfTransfer,
                amount: change_total,
                fee: None,
                counterparty: None,
                memo: None,
                block_height: None,
                status: ShieldedActivityStatus::Confirmed,
                created_at_ms: 0,
                min_note_position,
                note_cmxs: visible_cmxs,
                spent_nullifiers: Vec::new(),
            }
        };
        out.new_entries.push(entry);
    }

    out
}

/// Collapse a transfer's per-output byte values into the ONE value that
/// describes the whole transfer, or `None` when the outputs disagree.
///
/// A shielded transfer publishes exactly one activity row per
/// transition — the row's id is `sha256(sorted visible output cmxs)`
/// over the WHOLE cluster, so per-recipient rows could never dedupe
/// against the live row and a rescan would double-count. That single
/// row's scalar fields (`counterparty`, `memo`) are therefore only
/// meaningful when EVERY external output agrees on them: the
/// fund-one-address-with-N-notes shape. When they disagree there is no
/// honest single answer, and picking one output's value would attribute
/// the full aggregate amount to one of several distinct recipients.
/// "The first one" is not even a stable choice — Orchard's builder
/// shuffles outputs before pairing them into actions, so the winner
/// varies per build (and, on the restore path, per scan order).
///
/// Both the live recorder (`transfer_multi`) and the cold-restore
/// deriver ([`derive_activity_from_scan_data`]) route their
/// `counterparty` through this one function over the same canonical
/// 43-byte raw address encoding, so the two paths reach an identical
/// verdict for a given transition by construction rather than by two
/// copies of the rule agreeing.
///
/// Returns `None` for an empty iterator (no outputs, nothing to name).
pub(crate) fn unanimous_bytes<'a>(values: impl IntoIterator<Item = &'a [u8]>) -> Option<Vec<u8>> {
    let mut values = values.into_iter();
    let first = values.next()?;
    values.all(|v| v == first).then(|| first.to_vec())
}

/// Return `Some(memo)` when `memo` is non-empty and not all-zero;
/// `None` otherwise. A zero-filled 36-byte `DashMemo` is the "no memo"
/// sentinel and shouldn't surface as an attached memo.
pub(crate) fn non_zero_memo(memo: &[u8]) -> Option<Vec<u8>> {
    if memo.is_empty() || memo.iter().all(|&b| b == 0) {
        None
    } else {
        Some(memo.to_vec())
    }
}

/// The live `transfer_multi` activity ROW: classify ONE multi-output
/// shielded transfer's requested `(recipient, amount)` outputs into the
/// `(kind, amount, counterparty)` the live recorder writes.
///
/// Partition the outputs into EXTERNAL payments vs WALLET-OWNED
/// receipts by testing each recipient against the account's
/// `IncomingViewingKey::diversifier_index` — Orchard addresses are
/// diversified, so a fixed-address comparison cannot work; this is the
/// same ownership test `coordinator::is_own_orchard_recipient` uses to
/// build the deriver's own-address set — then derive the row from the
/// external subset only:
///
/// - No external output: nothing was paid to anyone, only the fee left
///   the pool → (`ShieldedSpend`, `fee`, no counterparty). Mirrors the
///   deriver's all-own arm, which has no external recipient to
///   aggregate and can never classify the cluster as `Sent`.
/// - Otherwise → (`Sent`, sum of the external amounts, counterparty by
///   the shared [`unanimous_bytes`] rule over the external recipients'
///   canonical 43-byte raw encodings — the exact form the deriver
///   recovers from the OVK-decrypted outgoing notes).
///
/// `operations::transfer_multi` records its live row through THIS
/// function, and the `live_and_restored_agree_*` parity tests below run
/// the same function against [`derive_activity_from_scan_data`] — so
/// the parity those tests prove binds the production code path, not a
/// test-local restatement of it (#4312 review finding 379da4cc0ad0).
pub(crate) fn live_transfer_multi_row(
    ivk: &IncomingViewingKey,
    outputs: &[(PaymentAddress, u64)],
    fee: u64,
) -> (ShieldedActivityKind, u64, Option<Vec<u8>>) {
    let external: Vec<&(PaymentAddress, u64)> = outputs
        .iter()
        .filter(|(recipient, _)| ivk.diversifier_index(recipient).is_none())
        .collect();
    if external.is_empty() {
        (ShieldedActivityKind::ShieldedSpend, fee, None)
    } else {
        let amount = external.iter().map(|(_, amount)| *amount).sum();
        let recipients: Vec<Vec<u8>> = external
            .iter()
            .map(|(recipient, _)| recipient.to_raw_address_bytes().to_vec())
            .collect();
        let counterparty = unanimous_bytes(recipients.iter().map(|r| r.as_slice()));
        (ShieldedActivityKind::Sent, amount, counterparty)
    }
}

/// Sort entries for display, in four bands. Mutates in place.
///
/// 1. `Pending` STATUS rows float to the very top. Pending is a status,
///    not "missing height": the live recorder flips successful ops to
///    `Confirmed` with `block_height: None`, so keying this band on
///    height would misfile the common success shape (the Swift UI
///    partitions by status for the same reason).
/// 2. Height-less rows **with a record time** — live successes whose
///    height the scan hasn't backfilled yet — newest first. They sit
///    above heighted rows because they are by construction the freshest
///    operations.
/// 3. Heighted rows, by `block_height` descending, tiebroken by
///    `created_at_ms` descending.
/// 4. Height-less rows with **no record time** (`created_at_ms == 0`) —
///    scan-derived restored history, whose real height and time are
///    unknowable client-side (see [`ShieldedActivityEntry::block_height`]).
///    Unknown age must not read as "newest", so they sink below every
///    dated/heighted row — ordered by [`min_note_position`] descending
///    (tree positions are exact chain order, so the band reads
///    newest-first like the rest of the list, identically on every
///    device), position-less entries last.
///
/// A final `id` tiebreak makes the whole order total and deterministic.
///
/// [`min_note_position`]: ShieldedActivityEntry::min_note_position
pub fn sort_activity_for_display(entries: &mut [ShieldedActivityEntry]) {
    // Band rank per the doc above; lower sorts first.
    fn band(e: &ShieldedActivityEntry) -> u8 {
        if e.status == ShieldedActivityStatus::Pending {
            0
        } else if e.block_height.is_some() {
            2
        } else if e.created_at_ms > 0 {
            1
        } else {
            3
        }
    }
    entries.sort_by(|a, b| {
        band(a)
            .cmp(&band(b))
            .then_with(|| match (a.block_height, b.block_height) {
                (Some(ah), Some(bh)) => bh.cmp(&ah),
                _ => std::cmp::Ordering::Equal,
            })
            // Tiebreak: more recent record time first.
            .then_with(|| b.created_at_ms.cmp(&a.created_at_ms))
            // Chain order (band 4's primary key; a no-op elsewhere,
            // where heights/record times already decided): later tree
            // position first, position-less entries last.
            .then_with(|| match (a.min_note_position, b.min_note_position) {
                (Some(ap), Some(bp)) => bp.cmp(&ap),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            // Final total order so the sort is deterministic.
            .then_with(|| a.id.cmp(&b.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn own_note(cmx: u8, nullifier: u8, height: u64, value: u64, spent: bool) -> ShieldedNote {
        ShieldedNote {
            position: 0,
            cmx: [cmx; 32],
            nullifier: [nullifier; 32],
            block_height: height,
            is_spent: spent,
            value,
            note_data: vec![0u8; 115],
        }
    }

    fn outgoing(
        cmx: u8,
        recipient: Vec<u8>,
        height: u64,
        value: u64,
        memo: Vec<u8>,
    ) -> ShieldedOutgoingNote {
        ShieldedOutgoingNote {
            cmx: [cmx; 32],
            recipient,
            value,
            memo,
            block_height: height,
        }
    }

    fn addr(b: u8) -> Vec<u8> {
        vec![b; 43]
    }

    // ── id / dedupe ────────────────────────────────────────────────

    #[test]
    fn id_is_order_independent_and_dedupes_duplicates() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let c = [3u8; 32];
        // Same set, different order → same id.
        assert_eq!(
            compute_activity_id(&[a, b, c]),
            compute_activity_id(&[c, a, b])
        );
        // A duplicated cmx doesn't change the id.
        assert_eq!(
            compute_activity_id(&[a, b]),
            compute_activity_id(&[a, b, a])
        );
        // A different set → different id.
        assert_ne!(compute_activity_id(&[a, b]), compute_activity_id(&[a, c]));
    }

    #[test]
    fn live_entry_then_scan_of_same_cluster_yields_one_entry() {
        // Live recorder recorded a Sent for a bundle whose single visible
        // output cmx is `7`. The scan later sees the same cluster (one
        // outgoing note with cmx `7`). Because the cluster's cmx overlaps
        // the stored entry's cmx (mapped to `live_id` in `existing_cmxs`),
        // the scan produces NOTHING for it.
        let cmx = [7u8; 32];
        let live_id = compute_activity_id(&[cmx]);
        let mut existing = BTreeMap::new();
        existing.insert(cmx, live_id);

        let input = ScanDeriveInput {
            notes: vec![],
            outgoing: vec![outgoing(7, addr(0xEE), 100, 500, vec![])],
            own_addresses: vec![addr(0x01)],
        };
        let derived = derive_activity_from_scan_data(&input, &existing);
        assert!(
            derived.new_entries.is_empty(),
            "scan must not re-emit a cluster a live entry already owns"
        );
        assert_eq!(
            derived.confirmations,
            vec![(live_id, 100)],
            "...but it must report the on-chain sighting so a Pending \
             live row can be confirmed at the observed height"
        );

        // With no live entry on file, the same scan DOES produce one entry,
        // and its id equals the live id (the dedupe contract).
        let derived2 = derive_activity_from_scan_data(&input, &BTreeMap::new());
        assert_eq!(derived2.new_entries.len(), 1);
        assert_eq!(derived2.new_entries[0].id, live_id);
        assert!(derived2.confirmations.is_empty());
    }

    #[test]
    fn same_block_merge_of_two_live_ops_confirms_both_without_new_entries() {
        // Two existing live entries own cmxs {A} and {B} respectively. A
        // later scan sees a single same-block cluster whose visible cmxs
        // are {A, B} (the documented same-block merge). The merged
        // cluster's own id `H(A∪B)` matches NEITHER live id, but overlap
        // dedupe catches both: no new (spuriously aggregate) entry, and a
        // confirmation for each overlapped id at the observed height.
        let a = [0xA1u8; 32];
        let b = [0xB2u8; 32];
        let id_a = compute_activity_id(&[a]);
        let id_b = compute_activity_id(&[b]);
        let mut existing = BTreeMap::new();
        existing.insert(a, id_a);
        existing.insert(b, id_b);

        // Cluster at height H carrying both cmxs as own received notes.
        let height = 500u64;
        let input = ScanDeriveInput {
            notes: vec![
                own_note(0xA1, 0x10, height, 1_000, false),
                own_note(0xB2, 0x11, height, 2_000, false),
            ],
            outgoing: vec![],
            own_addresses: vec![addr(0x01)],
        };
        let derived = derive_activity_from_scan_data(&input, &existing);
        assert!(
            derived.new_entries.is_empty(),
            "a same-block merge of two owned ops must not synthesize an aggregate row"
        );
        // `confirmations` carries both overlapped ids at H (BTreeSet order:
        // id_a sorts before id_b since 0xA1.. < 0xB2.. when hashed? — assert
        // membership rather than order to stay robust).
        assert_eq!(derived.confirmations.len(), 2);
        assert!(derived.confirmations.contains(&(id_a, height)));
        assert!(derived.confirmations.contains(&(id_b, height)));
    }

    // ── classification table (scan path) ───────────────────────────

    #[test]
    fn classifies_received_when_only_own_notes() {
        let input = ScanDeriveInput {
            notes: vec![own_note(0x10, 0x20, 50, 1_000, false)],
            outgoing: vec![],
            own_addresses: vec![addr(0x01)],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, ShieldedActivityKind::Received);
        assert_eq!(d[0].direction, ShieldedDirection::In);
        assert_eq!(d[0].amount, 1_000);
        // Scan-derived entries carry NO height and NO record time — the
        // note's stored height is the discovering batch's proof-anchor
        // (scan-tip) height, not an inclusion height, and there is no
        // wall-clock provenance for restored history. Both sentinels
        // are device-independent (restored-history acceptance criteria).
        assert_eq!(d[0].block_height, None);
        assert_eq!(d[0].created_at_ms, 0);
        // What they DO carry is the exact chain-order key: the received
        // note's tree position.
        assert_eq!(d[0].min_note_position, Some(0));
    }

    #[test]
    fn classifies_sent_when_outgoing_to_external_address() {
        let memo = {
            let mut m = vec![0u8; 36];
            m[0] = 1; // kind tag
            m[4] = b'h';
            m
        };
        let input = ScanDeriveInput {
            notes: vec![],
            outgoing: vec![outgoing(0x30, addr(0xEE), 200, 750, memo.clone())],
            own_addresses: vec![addr(0x01)],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, ShieldedActivityKind::Sent);
        assert_eq!(d[0].direction, ShieldedDirection::Out);
        assert_eq!(d[0].amount, 750);
        assert_eq!(d[0].counterparty.as_deref(), Some(addr(0xEE).as_slice()));
        assert_eq!(d[0].memo, Some(memo));
    }

    // ── multi-output transfers on the restore path ─────────────────

    /// The live `transfer_multi` counterparty rule, evaluated exactly as
    /// `operations::transfer_multi` evaluates it: the raw 43-byte
    /// encoding of each `(address, amount)` output, in call order,
    /// through the shared [`unanimous_bytes`] helper. The restore path
    /// must agree with this for the same transition.
    fn live_counterparty(recipients: &[Vec<u8>]) -> Option<Vec<u8>> {
        unanimous_bytes(recipients.iter().map(|r| r.as_slice()))
    }

    /// Real Orchard key material for the live/restored parity tests. The
    /// live row's ownership predicate is the account IVK itself
    /// (`IncomingViewingKey::diversifier_index`, inside
    /// [`live_transfer_multi_row`]), so exercising the production
    /// classifier needs addresses a real IVK does and does not recognize.
    fn parity_keys(seed_byte: u8) -> crate::wallet::shielded::keys::OrchardKeySet {
        crate::wallet::shielded::keys::OrchardKeySet::from_seed(
            &[seed_byte; 64],
            dashcore::Network::Testnet,
            0,
        )
        .expect("a 64-byte seed satisfies the ZIP-32 bounds")
    }

    /// Canonical 43-byte raw encoding of `addr` — the form outgoing notes
    /// store recipients in and the deriver matches own-addresses against.
    fn raw(addr: &PaymentAddress) -> Vec<u8> {
        addr.to_raw_address_bytes().to_vec()
    }

    #[test]
    fn live_and_restored_agree_on_a_mixed_external_and_own_output_set() {
        // The reviewer's mixed case: ONE Type-16 transition paying 10
        // credits to an EXTERNAL address and 20 to one of the account's
        // OWN diversified addresses (the public API accepts both).
        //
        // The live side of this check is the PRODUCTION classifier —
        // [`live_transfer_multi_row`], the function
        // `operations::transfer_multi` records its row through — run with
        // real Orchard key material, so the ownership predicate under test
        // is the real IVK one.
        //
        // Restoration removes the own output before aggregating, so it
        // records a 10-credit send to the external address. The live
        // recorder must reach the same verdict — before the fix it summed
        // every requested output and recorded a 30-credit send with no
        // counterparty (#4312 review finding 379da4cc0ad0).
        let ours = parity_keys(7);
        let theirs = parity_keys(9);
        let external = theirs.default_address;
        // A non-default diversified address: only the IVK predicate can
        // recognize it as ours — a fixed-address comparison against the
        // default address could not (`coordinator::is_own_orchard_recipient`
        // builds the deriver's own-address set from the same predicate).
        let own = ours.address_at(5);
        let fee = 7u64;

        let input = ScanDeriveInput {
            notes: vec![],
            outgoing: vec![
                outgoing(0x70, raw(&external), 800, 10, vec![0u8; 36]),
                outgoing(0x71, raw(&own), 800, 20, vec![0u8; 36]),
            ],
            own_addresses: vec![raw(&own)],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert_eq!(d.len(), 1, "one transition restores as one activity row");

        let (kind, amount, counterparty) = live_transfer_multi_row(
            &ours.incoming_viewing_key,
            &[(external, 10), (own, 20)],
            fee,
        );

        assert_eq!(d[0].kind, kind, "kind must agree");
        assert_eq!(kind, ShieldedActivityKind::Sent);
        assert_eq!(
            d[0].amount, amount,
            "amount must agree, and must count only the external payment"
        );
        assert_eq!(amount, 10, "the own output is change, not a payment");
        assert_eq!(d[0].counterparty, counterparty, "counterparty must agree");
        assert_eq!(
            counterparty,
            Some(raw(&external)),
            "one external recipient names itself"
        );

        // Pin the regression itself: the pre-fix live row was the SUM over
        // every output with no counterparty, which restoration never
        // produces for this transition.
        assert_ne!(amount, 30, "the own output must not inflate the amount");
    }

    #[test]
    fn live_and_restored_agree_that_an_all_own_output_set_is_not_a_send() {
        // Every output lands on an address this account's IVK recognizes,
        // so nothing was paid to anyone. Restoration cannot classify this
        // as `Sent` (it has no external recipient to aggregate); the live
        // classifier — the production [`live_transfer_multi_row`] that
        // `operations::transfer_multi` records through — must not either.
        let ours = parity_keys(7);
        let own_a = ours.default_address;
        let own_b = ours.address_at(3);
        let fee = 7u64;

        let (kind, amount, counterparty) =
            live_transfer_multi_row(&ours.incoming_viewing_key, &[(own_a, 10), (own_b, 20)], fee);
        assert_eq!(
            kind,
            ShieldedActivityKind::ShieldedSpend,
            "an all-own output set is a shielded spend, not a send"
        );
        assert_eq!(amount, fee, "only the fee left the pool");
        assert_eq!(counterparty, None, "there is no counterparty to name");

        // The restore side agrees on the kind: with both outgoing notes
        // recognized as own, the cluster has no external recipient, so the
        // `Sent` arm cannot fire.
        let input = ScanDeriveInput {
            notes: vec![own_note(0x80, 0x81, 900, 1_000, true)],
            outgoing: vec![
                outgoing(0x82, raw(&own_a), 900, 10, vec![0u8; 36]),
                outgoing(0x83, raw(&own_b), 900, 20, vec![0u8; 36]),
            ],
            own_addresses: vec![raw(&own_a), raw(&own_b)],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert_eq!(d.len(), 1);
        assert_ne!(
            d[0].kind,
            ShieldedActivityKind::Sent,
            "restoration must not call an all-own output set a send"
        );
        assert_eq!(d[0].kind, kind, "kind must agree with the live row");
    }

    #[test]
    fn multi_recipient_restore_aggregates_without_attributing_to_one_recipient() {
        // The reviewer's case: ONE Type-16 transition paying 10 credits to
        // A and 20 to B. Both outgoing notes are OVK-recovered into the
        // same height cluster, so restoration must produce ONE aggregate
        // row of 30 — and must NOT pin that 30 on either recipient.
        let a = addr(0xAA);
        let b = addr(0xBB);
        let input = ScanDeriveInput {
            notes: vec![],
            outgoing: vec![
                outgoing(0x60, a.clone(), 700, 10, vec![0u8; 36]),
                outgoing(0x61, b.clone(), 700, 20, vec![0u8; 36]),
            ],
            own_addresses: vec![addr(0x01)],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;

        assert_eq!(d.len(), 1, "one transition restores as one activity row");
        assert_eq!(d[0].kind, ShieldedActivityKind::Sent);
        assert_eq!(d[0].direction, ShieldedDirection::Out);
        assert_eq!(d[0].amount, 30, "amount is the sum over both outputs");
        assert_eq!(
            d[0].counterparty, None,
            "a 30-credit row covering two distinct recipients must name \
             neither of them"
        );
        // Live-path parity: the same rule, over the same raw encodings,
        // is what `transfer_multi` records for this transition.
        assert_eq!(
            d[0].counterparty,
            live_counterparty(&[a, b]),
            "restored attribution must match what the live recorder writes"
        );
    }

    #[test]
    fn multi_recipient_restore_is_independent_of_output_order() {
        // Orchard's builder shuffles outputs before pairing them into
        // actions, so "the first external output" is not a stable choice.
        // Feeding the same two outputs in the opposite order must derive
        // a byte-identical row (id included).
        let a = addr(0xAA);
        let b = addr(0xBB);
        let forward = outgoing(0x60, a.clone(), 700, 10, vec![0u8; 36]);
        let reverse = outgoing(0x61, b.clone(), 700, 20, vec![0u8; 36]);

        let mut d1 = derive_activity_from_scan_data(
            &ScanDeriveInput {
                notes: vec![],
                outgoing: vec![forward.clone(), reverse.clone()],
                own_addresses: vec![addr(0x01)],
            },
            &BTreeMap::new(),
        )
        .new_entries;
        let mut d2 = derive_activity_from_scan_data(
            &ScanDeriveInput {
                notes: vec![],
                outgoing: vec![reverse, forward],
                own_addresses: vec![addr(0x01)],
            },
            &BTreeMap::new(),
        )
        .new_entries;

        assert_eq!(d1.len(), 1);
        assert_eq!(d2.len(), 1);
        // `created_at_ms` is wall-clock; everything that describes the
        // transition must match. `note_cmxs` is stored in encounter
        // order (only `compute_activity_id` sorts), so it is compared as
        // the SET it represents — which is what the id contract keys on.
        let (e1, e2) = (d1.remove(0), d2.remove(0));
        assert_eq!(e1.id, e2.id, "cluster id is order-independent");
        assert_eq!(e1.amount, e2.amount);
        assert_eq!(
            e1.counterparty, e2.counterparty,
            "attribution must not depend on which output the scan saw first"
        );
        assert_eq!(e1.memo, e2.memo);
        let (mut c1, mut c2) = (e1.note_cmxs.clone(), e2.note_cmxs.clone());
        c1.sort_unstable();
        c2.sort_unstable();
        assert_eq!(c1, c2, "same cluster covers the same cmx set");
    }

    #[test]
    fn multi_note_restore_to_a_single_address_keeps_the_counterparty() {
        // The fund-one-address-with-N-notes shape (how a two-note invite
        // is funded): every output names the SAME address, so the row
        // still has one honest counterparty and must keep it — the fix
        // must not over-correct into always dropping attribution.
        let target = addr(0xCC);
        let memo = {
            let mut m = vec![0u8; 36];
            m[0] = 1;
            m
        };
        let input = ScanDeriveInput {
            notes: vec![],
            outgoing: vec![
                outgoing(0x70, target.clone(), 800, 100, memo.clone()),
                outgoing(0x71, target.clone(), 800, 250, memo.clone()),
            ],
            own_addresses: vec![addr(0x01)],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;

        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, ShieldedActivityKind::Sent);
        assert_eq!(d[0].amount, 350);
        assert_eq!(
            d[0].counterparty.as_deref(),
            Some(target.as_slice()),
            "all outputs agree on the address, so the row keeps it"
        );
        assert_eq!(
            d[0].memo,
            Some(memo),
            "transfer_multi attaches one memo to every recipient note, so \
             the unanimous memo survives"
        );
        assert_eq!(
            d[0].counterparty,
            live_counterparty(&[target.clone(), target]),
            "restored attribution must match what the live recorder writes"
        );
    }

    #[test]
    fn multi_recipient_restore_drops_a_memo_the_outputs_disagree_on() {
        // Same misattribution class as the counterparty: a memo that only
        // one output carried must not be presented as the whole
        // transfer's memo.
        let mut memo_a = vec![0u8; 36];
        memo_a[0] = 1;
        let input = ScanDeriveInput {
            notes: vec![],
            outgoing: vec![
                outgoing(0x80, addr(0xAA), 900, 10, memo_a),
                outgoing(0x81, addr(0xBB), 900, 20, vec![0u8; 36]),
            ],
            own_addresses: vec![addr(0x01)],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;

        assert_eq!(d.len(), 1);
        assert_eq!(d[0].amount, 30);
        assert_eq!(d[0].counterparty, None);
        assert_eq!(d[0].memo, None, "outputs disagree, so no memo is claimed");
    }

    #[test]
    fn unanimous_bytes_rule() {
        assert_eq!(unanimous_bytes(std::iter::empty()), None, "no outputs");
        assert_eq!(
            unanimous_bytes([b"abc".as_slice()]),
            Some(b"abc".to_vec()),
            "a single output always names itself"
        );
        assert_eq!(
            unanimous_bytes([b"abc".as_slice(), b"abc".as_slice()]),
            Some(b"abc".to_vec())
        );
        assert_eq!(
            unanimous_bytes([b"abc".as_slice(), b"abd".as_slice()]),
            None
        );
        // Disagreement anywhere in the list counts, not just against the
        // second element.
        assert_eq!(
            unanimous_bytes([b"abc".as_slice(), b"abc".as_slice(), b"zzz".as_slice()]),
            None
        );
    }

    #[test]
    fn sent_with_own_change_in_same_cluster_excludes_change_from_amount() {
        // A send that also produced an own change note at the same height:
        // the change receipt must NOT inflate the amount, and the cluster
        // classifies as Sent (not Received).
        let input = ScanDeriveInput {
            notes: vec![own_note(0x40, 0x41, 300, 9_999, false)], // change
            outgoing: vec![outgoing(0x42, addr(0xEE), 300, 600, vec![])], // payment
            own_addresses: vec![addr(0x01)],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, ShieldedActivityKind::Sent);
        assert_eq!(d[0].amount, 600, "amount is the payment, not the change");
    }

    #[test]
    fn self_change_only_cluster_is_shielded_spend() {
        // An outgoing note paying one of our OWN addresses, nothing else,
        // in a wallet that HAS spent notes: a spend whose output is all
        // self-change → residual ShieldedSpend. (In a never-spent wallet
        // the same shape collapses to Received — see
        // `self_pay_in_never_spent_wallet_is_received`.)
        let own = addr(0x01);
        let spent_elsewhere = own_note(9, 0x99, 5, 7_000, true);
        let input = ScanDeriveInput {
            notes: vec![spent_elsewhere],
            outgoing: vec![outgoing(0x50, own.clone(), 400, 0, vec![])],
            own_addresses: vec![own],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        let e = d
            .iter()
            .find(|e| e.kind == ShieldedActivityKind::ShieldedSpend)
            .expect("self-change cluster entry");
        assert_eq!(e.direction, ShieldedDirection::SelfTransfer);
        assert!(e.fee.is_none());
    }

    // ── zero-value filler / memo exclusion ─────────────────────────

    #[test]
    fn zero_memo_is_not_attached() {
        let input = ScanDeriveInput {
            notes: vec![],
            outgoing: vec![outgoing(0x60, addr(0xEE), 100, 500, vec![0u8; 36])],
            own_addresses: vec![addr(0x01)],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert_eq!(d.len(), 1);
        assert!(d[0].memo.is_none(), "an all-zero memo must surface as None");
    }

    #[test]
    fn empty_cluster_visible_set_is_skipped() {
        // No notes, no outgoing → no clusters → no entries.
        let input = ScanDeriveInput::default();
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert!(d.is_empty());
    }

    // ── backfill over existing store (multiple heights) ────────────

    #[test]
    fn backfill_over_existing_notes_produces_one_entry_per_cluster() {
        // A wallet that upgraded with existing state: receives at h=10,
        // a send at h=20. Backfill (existing_ids empty) yields exactly two
        // entries, one per height cluster.
        let input = ScanDeriveInput {
            notes: vec![own_note(0x70, 0x71, 10, 2_000, false)],
            outgoing: vec![outgoing(0x72, addr(0xEE), 20, 800, vec![])],
            own_addresses: vec![addr(0x01)],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert_eq!(d.len(), 2);
        // One entry per batch cluster; both carry the honest unknown
        // sentinels (no inclusion height / record time is recoverable).
        let sent = d
            .iter()
            .find(|e| e.kind == ShieldedActivityKind::Sent)
            .expect("send cluster entry");
        let received = d
            .iter()
            .find(|e| e.kind == ShieldedActivityKind::Received)
            .expect("receive cluster entry");
        assert_eq!(sent.block_height, None);
        assert_eq!(received.block_height, None);
        assert_eq!(sent.created_at_ms, 0);
        assert_eq!(received.created_at_ms, 0);
    }

    #[test]
    fn backfill_skips_clusters_already_recorded() {
        // h=10 already has a live entry; only h=20 should be derived.
        let input = ScanDeriveInput {
            notes: vec![own_note(0x70, 0x71, 10, 2_000, false)],
            outgoing: vec![outgoing(0x72, addr(0xEE), 20, 800, vec![])],
            own_addresses: vec![addr(0x01)],
        };
        let live_id = compute_activity_id(&[[0x70u8; 32]]);
        let mut existing = BTreeMap::new();
        existing.insert([0x70u8; 32], live_id);
        let d = derive_activity_from_scan_data(&input, &existing);
        assert_eq!(d.new_entries.len(), 1);
        assert_eq!(d.new_entries[0].kind, ShieldedActivityKind::Sent);
        assert_eq!(
            d.confirmations,
            vec![(live_id, 10)],
            "clusters overlapping an existing entry's cmxs must be reported \
             as on-chain sightings with their observed height (the \
             Pending->Confirmed promise the ambiguous post-broadcast paths \
             rely on)"
        );
    }

    // ── ordering ───────────────────────────────────────────────────

    #[test]
    fn display_sort_floats_pendings_then_height_desc() {
        let mk = |height: Option<u64>, created: u64, id: u8| ShieldedActivityEntry {
            id: [id; 32],
            kind: ShieldedActivityKind::Sent,
            direction: ShieldedDirection::Out,
            amount: 1,
            fee: None,
            counterparty: None,
            memo: None,
            block_height: height,
            status: if height.is_none() {
                ShieldedActivityStatus::Pending
            } else {
                ShieldedActivityStatus::Confirmed
            },
            created_at_ms: created,
            min_note_position: None,
            note_cmxs: vec![[id; 32]],
            spent_nullifiers: vec![],
        };
        // A live success: Confirmed but the scan hasn't backfilled the
        // height yet. Must NOT land in the pending band (status is the
        // discriminator), but sorts above heighted settled rows.
        let mut confirmed_no_height = mk(None, 4, 5);
        confirmed_no_height.status = ShieldedActivityStatus::Confirmed;

        let mut v = vec![
            mk(Some(100), 1, 1),
            mk(None, 5, 2), // pending — must float to top
            mk(Some(300), 2, 3),
            mk(Some(200), 3, 4),
            confirmed_no_height,
        ];
        sort_activity_for_display(&mut v);
        assert_eq!(v[0].status, ShieldedActivityStatus::Pending);
        assert_eq!(v[0].block_height, None, "pending floats to top");
        assert_eq!(
            (v[1].status, v[1].block_height),
            (ShieldedActivityStatus::Confirmed, None),
            "confirmed-without-height sorts at the top of the settled band, not as pending"
        );
        assert_eq!(v[2].block_height, Some(300));
        assert_eq!(v[3].block_height, Some(200));
        assert_eq!(v[4].block_height, Some(100));
    }

    /// Scan-derived restored rows (no height AND no record time) must
    /// sink below every dated/heighted row: unknown age must never read
    /// as "newest". Within the band they order by `min_note_position`
    /// descending — tree positions are exact chain order — so restored
    /// history reads newest-first like the rest of the list, and
    /// identically on every device restoring the same wallet.
    #[test]
    fn display_sort_sinks_unknown_age_scan_derived_rows_in_chain_order() {
        let mk = |height: Option<u64>, created: u64, position: Option<u64>, id: u8| {
            ShieldedActivityEntry {
                id: [id; 32],
                kind: ShieldedActivityKind::Sent,
                direction: ShieldedDirection::Out,
                amount: 1,
                fee: None,
                counterparty: None,
                memo: None,
                block_height: height,
                status: ShieldedActivityStatus::Confirmed,
                created_at_ms: created,
                min_note_position: position,
                note_cmxs: vec![[id; 32]],
                spent_nullifiers: vec![],
            }
        };
        let mut v = vec![
            // Scan-derived rows with positions deliberately out of id
            // order: id 9 holds the EARLIER position, so chain order
            // (position desc) must place id 2 first — proving the sort
            // keys on position, not id.
            mk(None, 0, Some(7), 9),
            mk(Some(100), 1, None, 1), // settled
            mk(None, 0, Some(41), 2),  // scan-derived, later in chain
            mk(None, 0, None, 6),      // scan-derived, position-less (send-only cluster)
            mk(None, 5, None, 3),      // fresh live success, height not yet backfilled
        ];
        sort_activity_for_display(&mut v);
        assert_eq!(
            (v[0].block_height, v[0].created_at_ms),
            (None, 5),
            "fresh live success stays on top of the settled bands"
        );
        assert_eq!(v[1].block_height, Some(100));
        assert_eq!(
            (v[2].id[0], v[3].id[0], v[4].id[0]),
            (2, 9, 6),
            "unknown-age rows sink to the bottom in chain order (position \
             desc), position-less rows last"
        );
    }

    /// The restored-history determinism criterion: deriving the same
    /// wallet data twice (two devices restoring the same seed) must
    /// produce byte-identical entries — same ids, same (absent) heights,
    /// same (unknown) timestamps — with no scan-moment dependence.
    #[test]
    fn scan_derivation_is_deterministic_across_devices() {
        let input = ScanDeriveInput {
            notes: vec![own_note(0x70, 0x71, 10, 2_000, false)],
            outgoing: vec![outgoing(0x72, addr(0xEE), 20, 800, vec![])],
            own_addresses: vec![addr(0x01)],
        };
        let device_a = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        // "Second device": same persisted chain data, different scan
        // moment — nothing in the derivation may read a clock.
        let device_b = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert_eq!(device_a, device_b);
        for e in &device_a {
            assert_eq!(e.block_height, None);
            assert_eq!(e.created_at_ms, 0);
        }
    }

    #[test]
    fn kind_tags_are_stable_and_distinct() {
        use std::collections::BTreeSet as Set;
        let kinds = [
            ShieldedActivityKind::Shield,
            ShieldedActivityKind::ShieldFromAssetLock,
            ShieldedActivityKind::Received,
            ShieldedActivityKind::Sent,
            ShieldedActivityKind::Unshield,
            ShieldedActivityKind::Withdrawal,
            ShieldedActivityKind::IdentityCreate {
                identity_id: [0u8; 32],
            },
            ShieldedActivityKind::ShieldedSpend,
        ];
        let tags: Set<u8> = kinds.iter().map(|k| k.tag()).collect();
        assert_eq!(tags.len(), kinds.len(), "every kind tag must be distinct");
    }

    // ── rho linkage (restore-path spend identification) ───────────

    /// Build a note whose `note_data` carries `rho` at the documented
    /// offset (recipient(43) || value(8) || rho(32) || rseed(32)).
    fn own_note_with_rho(
        cmx: u8,
        nullifier: u8,
        height: u64,
        value: u64,
        rho: [u8; 32],
    ) -> ShieldedNote {
        let mut data = vec![0u8; 115];
        data[51..83].copy_from_slice(&rho);
        ShieldedNote {
            position: 0,
            cmx: [cmx; 32],
            nullifier: [nullifier; 32],
            block_height: height,
            is_spent: false,
            value,
            note_data: data,
        }
    }

    #[test]
    fn rho_linked_change_classifies_as_spend_with_exact_amount() {
        // Spent note (0.4979) at h13; its spender's change (0.3979) lands
        // at h113 with rho == the spent note's nullifier. The change also
        // appears as a self-pay OVK note (same cmx) — the exact shape a
        // restored wallet sees after an unshield / identity-create.
        let own = addr(0xAA);
        let spent = own_note(1, 7, 13, 49_787_148_800, true);
        let change = own_note_with_rho(2, 8, 113, 39_787_148_800, [7u8; 32]);
        let input = ScanDeriveInput {
            notes: vec![spent, change],
            outgoing: vec![outgoing(2, own.clone(), 113, 39_787_148_800, vec![0u8; 36])],
            own_addresses: vec![own],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        let spend = d
            .iter()
            .find(|e| e.kind == ShieldedActivityKind::ShieldedSpend)
            .expect("spend cluster entry");
        assert_eq!(spend.direction, ShieldedDirection::Out);
        assert_eq!(
            spend.amount, 10_000_000_000,
            "amount must be spent − change (value that left the pool)"
        );
        assert_eq!(spend.spent_nullifiers, vec![[7u8; 32]]);
    }

    #[test]
    fn rho_linked_external_send_recovers_exact_fee() {
        // Transfer: spend 0.5, send 0.1 to a foreign address, change back
        // to self, rho links the change to the spent note → the exact fee
        // (spent − sent − change) is recoverable on restore.
        let own = addr(0xAA);
        let foreign = addr(0xBB);
        let spent = own_note(1, 7, 10, 50_000_000_000, true);
        let change = own_note_with_rho(2, 8, 20, 39_900_000_000, [7u8; 32]);
        let input = ScanDeriveInput {
            notes: vec![spent, change],
            outgoing: vec![
                outgoing(3, foreign.clone(), 20, 10_000_000_000, vec![0u8; 36]),
                outgoing(2, own.clone(), 20, 39_900_000_000, vec![0u8; 36]),
            ],
            own_addresses: vec![own],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        let sent = d
            .iter()
            .find(|e| e.kind == ShieldedActivityKind::Sent)
            .expect("send cluster entry");
        assert_eq!(sent.amount, 10_000_000_000);
        assert_eq!(
            sent.fee,
            Some(100_000_000),
            "rho linkage makes the exact fee recoverable"
        );
        assert_eq!(sent.counterparty.as_deref(), Some(foreign.as_slice()));
    }

    #[test]
    fn unlinked_self_pay_is_self_transfer_when_wallet_has_spends() {
        // Self-pay cluster with no rho link in a wallet that HAS spent
        // notes: either a shield-to-self or change whose output landed in
        // a different action than the real spend (the ~50% shuffle miss).
        // Must NOT surface as `Received` — that would show a spend as
        // money arriving.
        let own = addr(0xAA);
        let spent_elsewhere = own_note(9, 0x99, 5, 7_000, true);
        let note = own_note(1, 7, 30, 1_000_000, false);
        let input = ScanDeriveInput {
            notes: vec![spent_elsewhere, note],
            outgoing: vec![outgoing(1, own.clone(), 30, 1_000_000, vec![0u8; 36])],
            own_addresses: vec![own],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        let e = d
            .iter()
            .find(|e| e.kind == ShieldedActivityKind::ShieldedSpend)
            .expect("self-pay cluster entry");
        assert_eq!(e.direction, ShieldedDirection::SelfTransfer);
        assert_eq!(e.amount, 1_000_000);
    }

    #[test]
    fn self_pay_in_never_spent_wallet_is_received() {
        // The same self-pay shape in a subwallet with ZERO spent notes:
        // change-from-spend is impossible, so it is necessarily a shield
        // to self and must read as inbound value, not a self-transfer.
        let own = addr(0xAA);
        let note = own_note(1, 7, 30, 1_000_000, false);
        let input = ScanDeriveInput {
            notes: vec![note],
            outgoing: vec![outgoing(1, own.clone(), 30, 1_000_000, vec![0u8; 36])],
            own_addresses: vec![own],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, ShieldedActivityKind::Received);
        assert_eq!(d[0].direction, ShieldedDirection::In);
        assert_eq!(d[0].amount, 1_000_000);
    }

    #[test]
    fn third_party_receive_stays_received() {
        // A true receive: own new note with NO OVK pairing at all (we
        // never hold the sender's OVK). `Received` is reliable here.
        let own = addr(0xAA);
        let note = own_note(1, 7, 40, 5_000_000_000, false);
        let input = ScanDeriveInput {
            notes: vec![note],
            outgoing: vec![],
            own_addresses: vec![own],
        };
        let d = derive_activity_from_scan_data(&input, &BTreeMap::new()).new_entries;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, ShieldedActivityKind::Received);
        assert_eq!(d[0].direction, ShieldedDirection::In);
        assert_eq!(d[0].amount, 5_000_000_000);
    }
}
