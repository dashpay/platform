//! Live activity recording for the six shielded operation paths.
//!
//! At execution time each operation knows everything about itself — the
//! type, amounts, fee, recipient, memo, spent notes, and (for Type 20)
//! the created identity id. This module turns that knowledge into a
//! [`ShieldedActivityEntry`] and ships it through the
//! [`ShieldedChangeSet`] so it rides the existing persister flush.
//!
//! # Why recover cmxs from the built bundle
//!
//! The dedupe contract (see [`super::activity`]) keys an entry by
//! `sha256(sorted visible output cmxs)`, where "visible" = the outputs
//! the wallet's own IVK / OVK can recognize (own change notes + the
//! recovered send), excluding dummy fillers and spend actions. The scan
//! deriver computes that set from persisted notes / outgoing notes; for
//! the live entry to dedupe against a later rescan it must compute the
//! SAME set. So rather than guess which serialized action is an output,
//! we run the wallet's own IVK decrypt + OVK recovery over the built
//! bundle's actions — exactly what the scan does — and take the cmxs
//! that recover. This guarantees `live id == scan id` by construction.

use dash_sdk::platform::shielded::{try_decrypt_note, try_recover_outgoing_note};
// `ShieldedEncryptedNote` is the wire form the scan re-parses each
// on-chain note into. `drive-proof-verifier` is only a dev-dependency
// of this crate, so reach the type through `dash-sdk`'s public
// re-export (`pub use drive_proof_verifier::types as query_types`)
// rather than depending on it directly.
use dash_sdk::query_types::ShieldedEncryptedNote;
use grovedb_commitment_tree::ExtractedNoteCommitment;

use super::activity::{
    compute_activity_id, ShieldedActivityEntry, ShieldedActivityKind, ShieldedActivityStatus,
    ShieldedDirection,
};
use super::keys::AccountViewingKeys;
use super::store::{ShieldedNote, SubwalletId};
use crate::changeset::ShieldedChangeSet;

use dpp::shielded::SerializedAction;

/// Reconstruct the wallet-visible output cmxs from a built bundle's
/// serialized actions, using the wallet's own viewing keys — the same
/// IVK-decrypt + OVK-recover the scan runs, so the resulting id matches.
///
/// An action's cmx is included iff:
/// - it IVK-decrypts under `keys.incoming_viewing_key` (an own output —
///   a change note or a self-send), OR
/// - it OVK-recovers under `keys.outgoing_viewing_key` (a send the
///   wallet authored, encrypted to its own OVK at build time).
///
/// Dummy fillers (random OVK, not own IVK) and spend actions (no
/// recoverable output plaintext for this wallet) contribute nothing —
/// exactly the outputs the scan can't see either. Order-independent: the
/// caller hashes via [`compute_activity_id`], which sorts + dedups.
pub fn visible_output_cmxs(
    actions: &[SerializedAction],
    keys: &AccountViewingKeys,
) -> Vec<[u8; 32]> {
    let mut cmxs: Vec<[u8; 32]> = Vec::new();
    for a in actions {
        let wire = ShieldedEncryptedNote {
            cmx: a.cmx.to_vec(),
            nullifier: a.nullifier.to_vec(),
            cv_net: a.cv_net.to_vec(),
            encrypted_note: a.encrypted_note.clone(),
        };
        // Own output (change / self-send) via IVK.
        if let Some((note, _addr)) = try_decrypt_note(&keys.prepared_ivk, &wire) {
            cmxs.push(ExtractedNoteCommitment::from(note.commitment()).to_bytes());
            continue;
        }
        // Authored send via OVK (encrypted to the wallet's own OVK).
        if let Some((note, _recipient, _memo)) =
            try_recover_outgoing_note(&keys.outgoing_viewing_key, &wire)
        {
            cmxs.push(ExtractedNoteCommitment::from(note.commitment()).to_bytes());
        }
    }
    cmxs
}

/// Inputs the recorder needs that aren't carried by the kind itself.
pub struct LiveEntryParams<'a> {
    /// The classified kind (rich — the live path knows it exactly).
    pub kind: ShieldedActivityKind,
    /// Net direction relative to the wallet.
    pub direction: ShieldedDirection,
    /// Display amount in credits (principal — excludes self-change and
    /// zero-value fillers).
    pub amount: u64,
    /// Exact fee in credits when derivable. The live path derives it as
    /// `spent − outputs − change`; pass `None` only when genuinely
    /// unknown.
    pub fee: Option<u64>,
    /// Counterparty bytes (43B Orchard / 21B PlatformAddress / Core
    /// script) or `None`.
    pub counterparty: Option<Vec<u8>>,
    /// 36-byte memo when present and non-zero.
    pub memo: Option<Vec<u8>>,
    /// The bundle's serialized actions, for cmx extraction + nullifier
    /// linkage.
    pub actions: &'a [SerializedAction],
    /// The notes this operation spent (for `spent_nullifiers` linkage).
    /// Empty for output-only kinds (Shield / ShieldFromAssetLock).
    pub spent_notes: &'a [ShieldedNote],
}

/// Build a `Pending` [`ShieldedActivityEntry`] from the live operation's
/// full knowledge. Returns `None` when the bundle exposes no
/// wallet-visible output cmx (a degenerate all-dummy bundle), so the
/// caller skips recording rather than persisting a degenerate id.
pub fn build_pending_entry(
    keys: &AccountViewingKeys,
    params: LiveEntryParams<'_>,
) -> Option<ShieldedActivityEntry> {
    let note_cmxs = visible_output_cmxs(params.actions, keys);
    if note_cmxs.is_empty() {
        return None;
    }
    let id = compute_activity_id(&note_cmxs);
    let spent_nullifiers: Vec<[u8; 32]> = params.spent_notes.iter().map(|n| n.nullifier).collect();
    Some(ShieldedActivityEntry {
        id,
        kind: params.kind,
        direction: params.direction,
        amount: params.amount,
        fee: params.fee,
        counterparty: params.counterparty,
        memo: params.memo,
        block_height: None,
        status: ShieldedActivityStatus::Pending,
        created_at_ms: ShieldedActivityEntry::now_ms(),
        // Live entries order by their real record time; the chain-order
        // key is the scan deriver's (positions aren't known at record
        // time — the notes are discovered by a later scan).
        min_note_position: None,
        note_cmxs,
        spent_nullifiers,
    })
}

/// Return a copy of `entry` with its status set to `status` and (when
/// confirming) `block_height` filled if known. Re-emitting this through
/// the changeset upserts the existing row by `entry.id`.
pub fn with_status(
    entry: &ShieldedActivityEntry,
    status: ShieldedActivityStatus,
    block_height: Option<u64>,
) -> ShieldedActivityEntry {
    let mut next = entry.clone();
    next.status = status;
    if block_height.is_some() {
        next.block_height = block_height;
    }
    next
}

/// Convert a fixed 36-byte memo into `Some(memo)` when it carries
/// content, or `None` when it's the all-zero "no memo" sentinel. Thin
/// fixed-array adapter over the scan deriver's slice-keyed helper so
/// the sentinel logic exists exactly once and the live and scan paths
/// can never drift.
pub fn non_zero_memo(memo: &[u8; 36]) -> Option<Vec<u8>> {
    super::activity::non_zero_memo(&memo[..])
}

/// Ship one entry to the host persister through a fresh
/// [`ShieldedChangeSet`]. Convenience for the operation paths, which
/// already hold a `(persister, wallet_id, SubwalletId)` and a queue
/// helper (`queue_shielded_changeset`) — they call that with the
/// changeset this returns.
pub fn changeset_for_entry(id: SubwalletId, entry: ShieldedActivityEntry) -> ShieldedChangeSet {
    let mut cs = ShieldedChangeSet::default();
    cs.record_activity_entry(id, entry);
    cs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `with_status` flips status + fills height without disturbing the
    /// rest of the entry (the upsert-by-id contract).
    #[test]
    fn with_status_flips_and_fills_height_only() {
        let base = ShieldedActivityEntry {
            id: [9u8; 32],
            kind: ShieldedActivityKind::Sent,
            direction: ShieldedDirection::Out,
            amount: 500,
            fee: Some(10),
            counterparty: Some(vec![1u8; 43]),
            memo: None,
            block_height: None,
            status: ShieldedActivityStatus::Pending,
            created_at_ms: 123,
            min_note_position: None,
            note_cmxs: vec![[7u8; 32]],
            spent_nullifiers: vec![[3u8; 32]],
        };

        let confirmed = with_status(&base, ShieldedActivityStatus::Confirmed, Some(4242));
        assert_eq!(confirmed.status, ShieldedActivityStatus::Confirmed);
        assert_eq!(confirmed.block_height, Some(4242));
        // Everything else is preserved so the upsert only refines.
        assert_eq!(confirmed.id, base.id);
        assert_eq!(confirmed.amount, base.amount);
        assert_eq!(confirmed.fee, base.fee);
        assert_eq!(confirmed.counterparty, base.counterparty);
        assert_eq!(confirmed.note_cmxs, base.note_cmxs);
        assert_eq!(confirmed.spent_nullifiers, base.spent_nullifiers);

        // Failing without a height leaves block_height untouched (None).
        let failed = with_status(&base, ShieldedActivityStatus::Failed, None);
        assert_eq!(failed.status, ShieldedActivityStatus::Failed);
        assert_eq!(failed.block_height, None);
    }

    /// `changeset_for_entry` carries exactly one entry under the given id.
    #[test]
    fn changeset_for_entry_carries_one_entry() {
        let id = SubwalletId::new([0xAA; 32], 0);
        let entry = ShieldedActivityEntry {
            id: [1u8; 32],
            kind: ShieldedActivityKind::Shield,
            direction: ShieldedDirection::In,
            amount: 1,
            fee: None,
            counterparty: None,
            memo: None,
            block_height: None,
            status: ShieldedActivityStatus::Pending,
            created_at_ms: 0,
            min_note_position: None,
            note_cmxs: vec![[1u8; 32]],
            spent_nullifiers: vec![],
        };
        let cs = changeset_for_entry(id, entry);
        assert_eq!(cs.activity_entries.get(&id).map(|v| v.len()), Some(1));
        assert!(!cs.is_empty());
    }
}
