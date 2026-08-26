use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{DriveDocumentHavingQuery, RankedAxis, RankedEntry, RankedEntryValue};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::operations::proof::{indexed_axis::AxisEntries, VerifiedPathQuery};
use grovedb::{GroveDb, PathQuery};

impl DriveDocumentHavingQuery<'_> {
    /// v0 of [`Self::verify_having_range_proof`].
    ///
    /// Rebuilds the same [`PathQuery::new_axis_bounded`] the prover used
    /// — the path via [`Self::indexed_property_name_tree_path`], the
    /// bounds via
    /// [`AxisRangeBounds::i128_bounds`](crate::query::drive_document_having_query::AxisRangeBounds::i128_bounds)
    /// — then hands the proof to [`GroveDb::verify_path_query`] — an
    /// associated function, no database handle, so this compiles and
    /// runs in a verifier-only build.
    ///
    /// Three things are checked before the entries are returned:
    ///
    /// 1. **The proof answers this query.** The unified verifier is
    ///    query-as-input: nothing is echoed in the envelope; grovedb
    ///    lowers the query's bounds into the secondary's keyspace
    ///    through the same function the prover used and verifies the
    ///    proof against that reconstruction, so a proof generated for a
    ///    different bound — or a different direction, or a different
    ///    limit — is rejected rather than silently reinterpreted.
    ///    Completeness rides on the same check: a Merk range proof
    ///    commits its boundaries, so an in-range group the prover
    ///    omitted fails reconstruction.
    /// 2. **The result's axis shape matches the requested axis** — the
    ///    same belt-and-braces check the ranked verifier does.
    /// 3. **At most `limit` entries.** Fewer is normal — fewer groups
    ///    may match the bound — but more would mean the proof committed
    ///    a longer walk than the request authorized.
    ///
    /// No `platform_version` argument: the parent dispatcher already
    /// consumed it to select this version, and verification derives
    /// everything else from the proof bytes plus the query.
    #[inline(always)]
    pub(super) fn verify_having_range_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<RankedEntry>), Error> {
        let path = self.indexed_property_name_tree_path()?;
        let (lo, hi) = self.bounds.i128_bounds();
        let path_query = PathQuery::new_axis_bounded(
            path,
            self.bounds.axis().into(),
            lo,
            hi,
            self.limit,
            self.descending,
        );

        let verified =
            GroveDb::verify_path_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        // `skipped` is the paginated traversal's field and is `None` for
        // bounded ones — nothing to check here.
        let VerifiedPathQuery::AxisEntries {
            root_hash,
            entries,
            skipped: _,
        } = verified
        else {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "having range proof for the {:?} axis verified to a non-axis result shape",
                self.bounds.axis()
            ))));
        };

        let entries = match (self.bounds.axis(), entries) {
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
                    "having range proof for the {axis:?} axis verified to {} entries of a \
                     different axis shape",
                    other.len()
                ))));
            }
        };

        if entries.len() > self.limit as usize {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "having range proof for the {:?} axis verified to {} entries for limit = {}",
                self.bounds.axis(),
                entries.len(),
                self.limit
            ))));
        }

        Ok((root_hash, entries))
    }
}
