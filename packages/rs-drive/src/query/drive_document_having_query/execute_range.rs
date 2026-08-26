//! The two having-range executors on [`DriveDocumentHavingQuery`]: a
//! direct value-bounded read of the axis secondary, and generation of
//! the equivalent proof.
//!
//! Both are thin — all of the work happens inside grovedb, which seeks
//! straight to the encoded bounds in the pre-sorted secondary Merk. No
//! value trees are opened, no documents are materialized, and the cost
//! is `O(log n + k)` in the number of *matching* groups returned, never
//! in the total group population.
//!
//! Whole module is gated `feature = "server"` via the parent's
//! `pub mod execute_range;` declaration.

use super::super::drive_document_ranked_query::{RankedAxis, RankedEntry, RankedEntryValue};
use super::DriveDocumentHavingQuery;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{AxisKeys, PathQuery, PathQueryRun, TransactionArg};
use grovedb_costs::CostContext;
use grovedb_query::AxisQuery;

impl DriveDocumentHavingQuery<'_> {
    /// Read the matching groups directly from the axis secondary: every
    /// group whose aggregate falls inside the bounds, up to `limit`, in
    /// axis order in the walk direction.
    ///
    /// Fewer than `limit` entries is normal (fewer groups match) and is
    /// not an error; exactly `limit` entries may mean the match set was
    /// cut. A missing path *is* an error rather than an empty result,
    /// for the same reason as on the ranked surface: the indexed
    /// property-name tree is created at contract registration, so its
    /// absence means the contract-level state is not what the request
    /// claims. (An index with no documents has the tree, with an empty
    /// secondary, and yields an empty entry list.)
    pub fn execute_range_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<RankedEntry>, Error> {
        let grove_version = &platform_version.drive.grove_version;
        let path = self.indexed_property_name_tree_path()?;

        // The same bounded axis PathQuery the prove path uses, with the
        // keys-only projection: the matching pairs are read straight off
        // the pinned secondary view, no primary values resolved.
        let (lo, hi) = self.bounds.i128_bounds();
        let path_query = PathQuery::new_axis(
            path,
            AxisQuery::bounded(
                self.bounds.axis().into(),
                lo,
                hi,
                self.limit,
                self.descending,
            )
            .keys_only(),
        );

        // Costs are destructured away rather than `.unwrap()`-ed, same
        // as the ranked executors: `CostContext::unwrap` is infallible
        // but reads like a panicking unwrap at the call site.
        let CostContext { value, cost: _ } = drive.grove.run_path_query(
            &path_query,
            true,
            true,
            true,
            QueryResultType::QueryPathKeyElementTrioResultType,
            transaction,
            grove_version,
        );
        // `skipped` is the paginated traversal's field and is `None`
        // for bounded ones — nothing to check here.
        let PathQueryRun::AxisKeys { keys, skipped: _ } =
            value.map_err(|e| Error::GroveDB(Box::new(e)))?
        else {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "having {:?} range read ran to a non-axis-keys result shape",
                self.bounds.axis()
            ))));
        };

        let entries = match (self.bounds.axis(), keys) {
            (RankedAxis::Count, AxisKeys::Count(pairs)) => pairs
                .into_iter()
                .map(|(count, key)| RankedEntry {
                    key,
                    value: RankedEntryValue::Count(count),
                })
                .collect::<Vec<_>>(),
            (RankedAxis::Sum, AxisKeys::Sum(pairs)) => pairs
                .into_iter()
                .map(|(sum, key)| RankedEntry {
                    key,
                    value: RankedEntryValue::Sum(sum),
                })
                .collect::<Vec<_>>(),
            (RankedAxis::Avg, AxisKeys::Avg(pairs)) => pairs
                .into_iter()
                .map(|(avg, key)| RankedEntry {
                    key,
                    value: RankedEntryValue::AvgFixedPoint(avg),
                })
                .collect::<Vec<_>>(),
            (axis, other) => {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "having {axis:?} range read returned {} pairs of a different axis shape",
                    other.len()
                ))));
            }
        };

        // The limit is the contract with the caller, and on the prove
        // path it is re-checked inside the proof envelope. Asserting it
        // here keeps the no-proof and prove responses shape-identical.
        if entries.len() > self.limit as usize {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "having {:?} range read returned {} entries for limit = {}",
                self.bounds.axis(),
                entries.len(),
                self.limit
            ))));
        }
        Ok(entries)
    }

    /// Generate the bounded-axis proof for this query, through the
    /// unified `PathQuery` surface (grovedb's only public proof surface
    /// for indexed-axis reads): the query is
    /// [`PathQuery::new_axis_bounded`] and the envelope is a GroveDBProof
    /// V1 carrying an axis descent into the queried secondary.
    ///
    /// The envelope commits the in-range secondary entries, the
    /// primary's root hash, the sibling axes' root hashes, and the
    /// ordinary layer chain up to the grovedb root — so the client
    /// reconstructs the platform root hash from it. Nothing is echoed:
    /// the verifier takes the client's own reconstruction of the same
    /// `PathQuery` as input, and grovedb lowers its bounds into the
    /// secondary's keyspace through one function shared by both proof
    /// sides — which is why the bounds are validated rather than clamped
    /// upstream, and why completeness needs no extra machinery: a Merk
    /// range proof over a sorted keyspace commits its boundaries, so an
    /// in-range group the server omitted fails reconstruction.
    ///
    /// Verified by
    /// [`DriveDocumentHavingQuery::verify_having_range_proof`](crate::query::DriveDocumentHavingQuery::verify_having_range_proof).
    pub fn execute_range_with_proof(
        &self,
        drive: &Drive,
        _transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let grove_version = &platform_version.drive.grove_version;
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

        // The unified prover proves committed state — it takes no
        // transaction. The parameter is kept for signature stability with
        // the no-proof executor; the query dispatch passes `None` on this
        // surface anyway (queries answer from committed state).
        //
        // Same destructure-don't-unwrap rationale as the no-proof arm.
        let CostContext { value, cost: _ } =
            drive.grove.prove_query(&path_query, None, grove_version);
        value.map_err(|e| Error::GroveDB(Box::new(e)))
    }
}
