//! Durable record of the DashPay coreHeight backfill (DIP-15 §12.6).
//!
//! `reconcile_dashpay_rescan` lowers the wallet's SPV `synced_height` to the
//! earliest contact-request height among receival contacts whose addresses
//! were not yet watched when those blocks were first scanned, so the filter
//! scanner re-matches that range with the contact's addresses in the set.
//! The guard that stops the recurring sweep from re-lowering the cursor every
//! pass used to live only in memory (`DashPayState::rescan_triggered`), while
//! the cursor it lowers is durable: the host persists every
//! `SyncHeightAdvanced` the engine emits as the rescan climbs. A fresh process
//! therefore restored a cursor *inside* the climb, saw no guard, and rewound
//! again — one full re-walk per launch, forever (dashpay/platform#4302).
//!
//! [`DashPayBackfillRecord`] is the guard's durable half. It travels with the
//! wallet's own changeset round (the one that also carries the lowered
//! cursor), crosses the FFI as its own size-negotiated persistence slot and a
//! wallet-restore field, and comes back through
//! [`ClientWalletStartState::dashpay_backfill`].
//!
//! [`ClientWalletStartState::dashpay_backfill`]: crate::changeset::ClientWalletStartState::dashpay_backfill

use dpp::prelude::Identifier;

/// One receival contact the backfill covers, and the height it covers it
/// from.
///
/// `covered_from` is the contact's scan checkpoint at the moment its coverage
/// was recorded — the earliest height whose block the scanner is guaranteed to
/// test against this contact's addresses. It is stored per contact rather than
/// as one wallet-wide floor because a contact's checkpoint can *drop* after it
/// was covered (the reciprocal request turns out to be older than ours, or a
/// rotation falls back to wallet birth): a wallet-wide floor would then read
/// as "still covered" for blocks the scan never tested with that contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DashPayBackfillCoveredContact {
    /// Our identity — the owner of the receival account.
    pub owner: Identifier,
    /// The contact whose payments the receival account collects.
    pub contact: Identifier,
    /// Lowest height the scan is guaranteed to have tested (or to test, as it
    /// climbs) with this contact's addresses watched.
    pub covered_from: u32,
}

/// Per-wallet durable record of the DashPay coreHeight backfill.
///
/// # What a record claims
///
/// For every entry in [`covered`](Self::covered): from the moment the record
/// was written, the contact's receival addresses are in the filter match set,
/// and the durable `synced_height` was lowered to at most that contact's
/// `covered_from` in the very same persistence round. From then on the
/// persisted cursor tracks the scanner's real progress *with the contact
/// watched* — the pinned engine refuses to certify a batch scanned at the old
/// checkpoint once the cursor sits below it (contiguity guard,
/// dashpay/rust-dashcore#649) — so on any later launch every block in
/// `[covered_from, synced_height]` has been tested against that contact, and
/// everything above the cursor is about to be. Such a contact needs no second
/// rewind unless its checkpoint has since dropped *below* `covered_from`.
///
/// # What it does not claim
///
/// Nothing about a receival account the record does not list. A contact whose
/// account was registered after the record was written (or on this device by
/// a path that never recorded it) is re-armed exactly as before, because the
/// scan that produced this record never watched it. Any ambiguity falls back
/// to re-running the backfill: slow, never lossy.
///
/// # Completion
///
/// [`floor`](Self::floor) and [`rewound_from`](Self::rewound_from) describe
/// the backfill's extent: the lowest height it rewound to and the cursor it
/// rewound from. The backfill is complete once `synced_height` has climbed
/// back past `rewound_from` ([`is_complete`](Self::is_complete)); while it is
/// still climbing ([`is_pending`](Self::is_pending)) a restart resumes the
/// climb from the persisted cursor rather than restarting it from the floor.
/// Completion is never *marked* — it is read off the cursor, which is the
/// only thing that actually moved. Marking it at trigger time would have made
/// an interrupted backfill look finished; not marking it costs nothing,
/// because the per-contact `covered_from` entries, not the completion state,
/// are what suppress a second rewind.
///
/// A later rewind folds into the existing record ([`record_pass`](Self::record_pass)):
/// the floor can only go down, `rewound_from` can only go up, and a contact's
/// entry is replaced by the new checkpoint it was covered from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DashPayBackfillRecord {
    /// Lowest height a backfill rewound the cursor to. Meaningless while
    /// `covered` is empty.
    pub floor: u32,
    /// Highest cursor a backfill rewound from — the height the scan has to
    /// climb back to for the backfill to be complete. Meaningless while
    /// `covered` is empty.
    pub rewound_from: u32,
    /// Receival contacts the backfill covers, sorted by `(owner, contact)`,
    /// at most one entry per pair.
    pub covered: Vec<DashPayBackfillCoveredContact>,
}

impl DashPayBackfillRecord {
    /// Whether no backfill has ever been recorded for this wallet.
    pub fn is_empty(&self) -> bool {
        self.covered.is_empty()
    }

    /// The height this record covers `(owner, contact)` from, if it covers
    /// it at all.
    pub fn covered_from(&self, owner: &Identifier, contact: &Identifier) -> Option<u32> {
        self.covered
            .binary_search_by(|entry| (entry.owner, entry.contact).cmp(&(*owner, *contact)))
            .ok()
            .map(|index| self.covered[index].covered_from)
    }

    /// Whether `(owner, contact)` is covered at `checkpoint`: the record lists
    /// it and its checkpoint has not dropped below the height it was covered
    /// from.
    pub fn covers(&self, owner: &Identifier, contact: &Identifier, checkpoint: u32) -> bool {
        self.covered_from(owner, contact)
            .is_some_and(|covered_from| checkpoint >= covered_from)
    }

    /// The backfill is still climbing back to the cursor it rewound from.
    pub fn is_pending(&self, synced_height: u32) -> bool {
        !self.is_empty() && synced_height < self.rewound_from
    }

    /// The scan has climbed back past the cursor the backfill rewound from,
    /// so every block in `[floor, rewound_from]` has been re-matched.
    pub fn is_complete(&self, synced_height: u32) -> bool {
        !self.is_empty() && synced_height >= self.rewound_from
    }

    /// Fold one reconcile pass into the record.
    ///
    /// `synced_height_before` is the cursor before the pass rewound it (the
    /// height the scan must climb back to); `floor` is where the pass rewound
    /// to, or `None` when every contact it handled was already covered by the
    /// forward scan. Each `(owner, contact, covered_from)` entry replaces any
    /// existing entry for the pair.
    ///
    /// Returns `true` when the record changed.
    pub fn record_pass(
        &mut self,
        synced_height_before: u32,
        floor: Option<u32>,
        entries: impl IntoIterator<Item = (Identifier, Identifier, u32)>,
    ) -> bool {
        let was_empty = self.covered.is_empty();
        let mut changed = false;
        for (owner, contact, covered_from) in entries {
            let entry = DashPayBackfillCoveredContact {
                owner,
                contact,
                covered_from,
            };
            match self.covered.binary_search_by(|existing| {
                (existing.owner, existing.contact).cmp(&(owner, contact))
            }) {
                Ok(index) => {
                    if self.covered[index] != entry {
                        self.covered[index] = entry;
                        changed = true;
                    }
                }
                Err(index) => {
                    self.covered.insert(index, entry);
                    changed = true;
                }
            }
        }
        if self.covered.is_empty() {
            return changed;
        }
        // A first record takes the pass's extent as-is; a later pass can only
        // widen it — deeper floor, higher climb target.
        let pass_floor = floor.unwrap_or(synced_height_before);
        let (new_floor, new_rewound_from) = if was_empty {
            (pass_floor, synced_height_before)
        } else {
            (
                self.floor.min(pass_floor),
                self.rewound_from.max(synced_height_before),
            )
        };
        if new_floor != self.floor || new_rewound_from != self.rewound_from {
            self.floor = new_floor;
            self.rewound_from = new_rewound_from;
            changed = true;
        }
        changed
    }

    /// Drop every entry for which `keep` is false — used to forget contacts
    /// whose receival account no longer exists. Returns `true` when the record
    /// changed.
    pub fn retain(&mut self, mut keep: impl FnMut(&Identifier, &Identifier) -> bool) -> bool {
        let before = self.covered.len();
        self.covered
            .retain(|entry| keep(&entry.owner, &entry.contact));
        before != self.covered.len()
    }

    /// Wire size of one [`DashPayBackfillCoveredContact`] in
    /// [`covered_bytes`](Self::covered_bytes): owner ‖ contact ‖ `covered_from`
    /// (little-endian).
    pub const COVERED_ENTRY_LEN: usize = 32 + 32 + 4;

    /// Flat encoding of [`covered`](Self::covered) for hosts that store the
    /// list as one opaque blob: [`COVERED_ENTRY_LEN`](Self::COVERED_ENTRY_LEN)
    /// bytes per entry, in record order.
    pub fn covered_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.covered.len() * Self::COVERED_ENTRY_LEN);
        for entry in &self.covered {
            out.extend_from_slice(&entry.owner.to_buffer());
            out.extend_from_slice(&entry.contact.to_buffer());
            out.extend_from_slice(&entry.covered_from.to_le_bytes());
        }
        out
    }

    /// Inverse of [`covered_bytes`](Self::covered_bytes).
    ///
    /// Returns `None` when `bytes` is not a whole number of entries: a
    /// truncated or foreign blob must not be read as a shorter cover set,
    /// because a contact missing from the set is simply re-armed while a
    /// contact wrongly *present* in it is never backfilled again. The caller
    /// treats `None` as "no record" — the safe direction.
    pub fn from_parts(floor: u32, rewound_from: u32, covered_bytes: &[u8]) -> Option<Self> {
        if !covered_bytes.len().is_multiple_of(Self::COVERED_ENTRY_LEN) {
            return None;
        }
        let (chunks, _) = covered_bytes.as_chunks::<{ Self::COVERED_ENTRY_LEN }>();
        let mut covered: Vec<DashPayBackfillCoveredContact> = chunks
            .iter()
            .map(|chunk| {
                let mut owner = [0u8; 32];
                let mut contact = [0u8; 32];
                let mut height = [0u8; 4];
                owner.copy_from_slice(&chunk[..32]);
                contact.copy_from_slice(&chunk[32..64]);
                height.copy_from_slice(&chunk[64..68]);
                DashPayBackfillCoveredContact {
                    owner: Identifier::from(owner),
                    contact: Identifier::from(contact),
                    covered_from: u32::from_le_bytes(height),
                }
            })
            .collect();
        // Canonical order and one entry per pair, whatever the host stored.
        covered.sort_by_key(|entry| (entry.owner, entry.contact));
        covered.dedup_by(|a, b| (a.owner, a.contact) == (b.owner, b.contact));
        Some(Self {
            floor,
            rewound_from,
            covered,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    #[test]
    fn a_first_pass_sets_the_extent_and_a_later_one_only_widens_it() {
        let mut record = DashPayBackfillRecord::default();
        assert!(record.is_empty());
        assert!(!record.is_pending(0));
        assert!(!record.is_complete(u32::MAX));

        assert!(record.record_pass(1_000, Some(100), [(id(1), id(2), 100)]));
        assert_eq!((record.floor, record.rewound_from), (100, 1_000));
        assert!(record.is_pending(999));
        assert!(record.is_complete(1_000));

        // A shallower rewind while climbing neither raises the floor nor
        // lowers the climb target.
        assert!(record.record_pass(400, Some(300), [(id(1), id(3), 300)]));
        assert_eq!((record.floor, record.rewound_from), (100, 1_000));

        // A deeper rewind after completion lowers the floor and raises the
        // climb target to the cursor it rewound from.
        assert!(record.record_pass(2_000, Some(50), [(id(1), id(4), 50)]));
        assert_eq!((record.floor, record.rewound_from), (50, 2_000));

        // A forward-covered pass (no rewind) records the contact without
        // touching the extent it cannot widen.
        assert!(record.record_pass(1_500, None, [(id(1), id(5), 1_700)]));
        assert_eq!((record.floor, record.rewound_from), (50, 2_000));
        assert_eq!(record.covered_from(&id(1), &id(5)), Some(1_700));

        // An unchanged replay is reported as such.
        assert!(!record.record_pass(1_500, None, [(id(1), id(5), 1_700)]));
    }

    #[test]
    fn coverage_is_per_contact_and_keyed_on_the_recorded_checkpoint() {
        let mut record = DashPayBackfillRecord::default();
        record.record_pass(1_000, Some(100), [(id(1), id(2), 100), (id(1), id(3), 300)]);

        assert!(record.covers(&id(1), &id(2), 100));
        assert!(record.covers(&id(1), &id(2), 250));
        assert!(
            !record.covers(&id(1), &id(2), 99),
            "a checkpoint below the covered height re-arms"
        );
        assert!(
            !record.covers(&id(1), &id(9), 500),
            "an unlisted contact is never covered"
        );
        assert!(
            !record.covers(&id(7), &id(2), 500),
            "coverage is per owner, too"
        );

        // Re-recording a pair replaces its entry.
        assert!(record.record_pass(500, Some(40), [(id(1), id(2), 40)]));
        assert_eq!(record.covered_from(&id(1), &id(2)), Some(40));
        assert_eq!(record.covered.len(), 2);
    }

    #[test]
    fn the_flat_encoding_round_trips_and_refuses_a_torn_blob() {
        let mut record = DashPayBackfillRecord::default();
        record.record_pass(1_000, Some(100), [(id(9), id(2), 100), (id(1), id(3), 300)]);
        let bytes = record.covered_bytes();
        assert_eq!(bytes.len(), 2 * DashPayBackfillRecord::COVERED_ENTRY_LEN);

        let restored = DashPayBackfillRecord::from_parts(record.floor, record.rewound_from, &bytes)
            .expect("a whole blob decodes");
        assert_eq!(restored, record);

        assert!(
            DashPayBackfillRecord::from_parts(100, 1_000, &bytes[..bytes.len() - 1]).is_none(),
            "a torn blob must read as no record, never as a shorter cover set"
        );
        let empty = DashPayBackfillRecord::from_parts(0, 0, &[]).expect("empty decodes");
        assert!(empty.is_empty());
    }

    #[test]
    fn a_host_stored_blob_is_canonicalised_on_the_way_in() {
        let entry = |owner: u8, contact: u8, from: u32| {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&[owner; 32]);
            bytes.extend_from_slice(&[contact; 32]);
            bytes.extend_from_slice(&from.to_le_bytes());
            bytes
        };
        let mut bytes = entry(5, 6, 700);
        bytes.extend(entry(1, 2, 100));
        bytes.extend(entry(5, 6, 900)); // duplicate pair
        let record = DashPayBackfillRecord::from_parts(100, 1_000, &bytes).expect("decodes");
        assert_eq!(record.covered.len(), 2);
        assert_eq!(record.covered[0].owner, id(1));
        assert_eq!(record.covered_from(&id(5), &id(6)), Some(700));
    }

    #[test]
    fn retain_forgets_contacts_and_reports_whether_anything_went() {
        let mut record = DashPayBackfillRecord::default();
        record.record_pass(1_000, Some(100), [(id(1), id(2), 100), (id(1), id(3), 300)]);
        assert!(!record.retain(|_, _| true));
        assert!(record.retain(|_, contact| *contact != id(3)));
        assert_eq!(record.covered_from(&id(1), &id(3)), None);
        assert_eq!(record.covered_from(&id(1), &id(2)), Some(100));
    }
}
