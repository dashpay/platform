use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{
    DriveDocumentRankedQuery, RankedAxis, RankedEntry, RankedEntryValue, RankedPage,
};
use crate::verify::RootHash;
use grovedb::operations::proof::indexed_axis::AxisEntries;
use grovedb::GroveDb;

impl DriveDocumentRankedQuery<'_> {
    /// v0 of [`Self::verify_ranked_top_k_proof`].
    ///
    /// Rebuilds the proved subtree path with
    /// [`Self::indexed_property_name_tree_path`] and hands the proof to
    /// [`GroveDb::verify_indexed_axis_top_k_paginated`], which is an
    /// associated function — no database handle is involved, so this
    /// compiles and runs in a verifier-only build.
    ///
    /// Three things are checked before the page is returned:
    ///
    /// 1. **The envelope's `(axis, k, offset, descending)` match this
    ///    query.** grovedb does this itself: the values are echoed in
    ///    the proof and compared against the arguments, so a proof
    ///    generated for a different ranking — or a different page of the
    ///    same ranking — is rejected rather than silently reinterpreted.
    /// 2. **The result's axis shape matches the requested axis** — a
    ///    `Count` request must not come back holding `Sum` entries. This
    ///    is belt-and-braces on top of (1) (the tag check already rules
    ///    it out) and exists so a future decoder change that decouples
    ///    the tag from the decoded variant surfaces as an error here
    ///    instead of a mis-typed number reaching the caller.
    /// 3. **At most `k` entries.** Fewer is normal — the index may hold
    ///    fewer groups than were asked for — but more would mean the
    ///    proof committed a longer walk than the request authorized.
    ///
    /// The returned [`RankedPage::skipped`] is grovedb's independently
    /// re-derived skip count, not an echo of the request: it is
    /// recomputed by the verifier from the counted subtree commitments
    /// in the proof bytes. It equals `self.offset` on a full page; when
    /// the walk exhausted the secondary during the skip it is smaller,
    /// `entries` is empty, and the pair is a proof that the ranking
    /// holds exactly `skipped` groups. Deliberately **not** rejected
    /// here — "you paged past the end, and here is how far the end is"
    /// is a useful answer, and the only place that knows whether it is
    /// acceptable is the caller.
    ///
    /// No `platform_version` argument: the parent dispatcher already
    /// consumed it to select this version, and nothing in the body needs
    /// a grove version (verification derives everything from the proof
    /// bytes plus the path).
    #[inline(always)]
    pub(super) fn verify_ranked_top_k_proof_v0(
        &self,
        proof: &[u8],
    ) -> Result<(RootHash, RankedPage), Error> {
        let path = self.indexed_property_name_tree_path()?;
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();

        let result = GroveDb::verify_indexed_axis_top_k_paginated(
            proof,
            path_refs.as_slice(),
            self.axis.into(),
            self.k,
            self.offset as u64,
            self.descending,
        )
        .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let entries = match (self.axis, result.entries) {
            (RankedAxis::Count, AxisEntries::Count(entries)) => entries
                .into_iter()
                .map(|(count, key)| RankedEntry {
                    key,
                    value: RankedEntryValue::Count(count),
                })
                .collect::<Vec<_>>(),
            (RankedAxis::Sum, AxisEntries::Sum(entries)) => entries
                .into_iter()
                .map(|(sum, key)| RankedEntry {
                    key,
                    value: RankedEntryValue::Sum(sum),
                })
                .collect::<Vec<_>>(),
            (RankedAxis::Avg, AxisEntries::Avg(entries)) => entries
                .into_iter()
                .map(|(avg, key)| RankedEntry {
                    key,
                    value: RankedEntryValue::AvgFixedPoint(avg),
                })
                .collect::<Vec<_>>(),
            (axis, other) => {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "ranked top-k proof for the {axis:?} axis verified to {} entries of a \
                     different axis shape",
                    other.len()
                ))));
            }
        };

        if entries.len() > self.k as usize {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "ranked top-k proof for the {:?} axis verified to {} entries for k = {}",
                self.axis,
                entries.len(),
                self.k
            ))));
        }

        Ok((
            result.root_hash,
            RankedPage {
                skipped: result.skipped,
                entries,
            },
        ))
    }
}
