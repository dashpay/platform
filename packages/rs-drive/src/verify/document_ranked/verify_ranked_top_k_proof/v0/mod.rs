use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{
    DriveDocumentRankedQuery, RankedAxis, RankedEntry, RankedEntryValue, RankedPage,
};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::operations::proof::{indexed_axis::AxisEntries, VerifiedPathQuery};
use grovedb::{GroveDb, PathQuery};

impl DriveDocumentRankedQuery<'_> {
    /// v0 of [`Self::verify_ranked_top_k_proof`].
    ///
    /// Rebuilds the same [`PathQuery::new_axis_top_k`] the prover used —
    /// path via [`Self::indexed_property_name_tree_path`], the axis /
    /// `k` / offset / direction from this query — and hands the proof to
    /// [`GroveDb::verify_path_query`], which is an associated function —
    /// no database handle is involved, so this compiles and runs in a
    /// verifier-only build.
    ///
    /// Three things are checked before the page is returned:
    ///
    /// 1. **The proof answers this query.** The unified verifier is
    ///    query-as-input: nothing is echoed in the envelope; the proof is
    ///    verified against the verifier's own reconstruction of the
    ///    `PathQuery`, so a proof generated for a different ranking — or
    ///    a different page of the same ranking — fails verification
    ///    rather than being silently reinterpreted.
    /// 2. **The result's axis shape matches the requested axis** — a
    ///    `Count` request must not come back holding `Sum` entries. This
    ///    is belt-and-braces on top of (1) (the query's axis already
    ///    rules it out) and exists so a future decoder change that
    ///    decouples the query from the decoded variant surfaces as an
    ///    error here instead of a mis-typed number reaching the caller.
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
    #[inline(always)]
    pub(super) fn verify_ranked_top_k_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, RankedPage), Error> {
        let path = self.indexed_property_name_tree_path()?;
        let path_query = PathQuery::new_axis_top_k(
            path,
            self.axis.into(),
            self.k,
            self.offset as u64,
            self.descending,
        );

        let verified =
            GroveDb::verify_path_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let VerifiedPathQuery::AxisEntries {
            root_hash,
            entries,
            skipped,
        } = verified
        else {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "ranked top-k proof for the {:?} axis verified to a non-axis result shape",
                self.axis
            ))));
        };

        // A paginated (top-k) traversal always attests its skip count;
        // `None` is the bounded traversal's shape and cannot answer this
        // query.
        let skipped = skipped.ok_or_else(|| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "ranked top-k proof for the {:?} axis carried no attested skip count",
                self.axis
            )))
        })?;

        let entries = match (self.axis, entries) {
            (RankedAxis::Count, AxisEntries::Count(entries)) => entries
                .into_iter()
                .map(|entry| entry.key_pair())
                .map(|(count, key)| RankedEntry {
                    key,
                    value: RankedEntryValue::Count(count),
                })
                .collect::<Vec<_>>(),
            (RankedAxis::Sum, AxisEntries::Sum(entries)) => entries
                .into_iter()
                .map(|entry| entry.key_pair())
                .map(|(sum, key)| RankedEntry {
                    key,
                    value: RankedEntryValue::Sum(sum),
                })
                .collect::<Vec<_>>(),
            (RankedAxis::Avg, AxisEntries::Avg(entries)) => entries
                .into_iter()
                .map(|entry| entry.key_pair())
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

        Ok((root_hash, RankedPage { skipped, entries }))
    }
}
