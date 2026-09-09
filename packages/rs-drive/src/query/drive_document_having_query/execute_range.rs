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

use super::super::drive_document_ranked_query::branches::{
    axis_keys_to_ranked, decompose_branch_paths, read_branched_union,
};
use super::super::drive_document_ranked_query::RankedEntry;
use super::DriveDocumentHavingQuery;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, PathQueryRun, TransactionArg};
use grovedb_costs::CostContext;
use grovedb_query::AxisQuery;

impl DriveDocumentHavingQuery<'_> {
    /// Read the matching groups directly from the axis secondary: every
    /// group whose aggregate falls inside the bounds, up to `limit`, in
    /// axis order in the walk direction.
    ///
    /// Fewer than `limit` entries is normal (fewer groups match) and is
    /// not an error; exactly `limit` entries may mean the match set was
    /// cut.
    ///
    /// Missing paths follow the ranked surface's rule. Under a single
    /// `==` pin (or no pins) a missing path *is* an error rather than an
    /// empty result: the indexed property-name tree is created at
    /// contract registration, so its absence means the contract-level
    /// state is not what the request claims. On an `IN`-pinned request,
    /// an element whose branch chain is missing at ANY depth — the
    /// branch key, or any deeper pinned segment under a *present* key —
    /// contributes an **empty branch** instead (union semantics, exactly
    /// as the proved envelope authenticates it), and the union is served
    /// from one committed state (a `None` read runs under a grovedb
    /// snapshot read transaction). An index with no documents has the
    /// tree, with an empty secondary, and yields an empty entry list.
    pub fn execute_range_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<RankedEntry>, Error> {
        if self.prefix_branches.len() > 1 {
            // ONE grovedb call for the whole union, pinned to one
            // committed state — the entire sequence lives in the ranked
            // surface's `branches::read_branched_union`, shared with the
            // ranked executor so the two cannot drift.
            let paths = (0..self.prefix_branches.len())
                .map(|branch| self.indexed_property_name_tree_path(branch))
                .collect::<Result<Vec<_>, Error>>()?;
            let axis = self.bounds.axis();
            let (lo, hi) = self.bounds.inclusive_bounds_i128();
            return read_branched_union(
                &drive.grove,
                "having",
                &self.prefix_branches,
                &paths,
                axis,
                AxisQuery::bounded(axis.into(), lo, hi, self.limit, self.descending),
                self.limit as usize,
                self.descending,
                transaction,
                &platform_version.drive.grove_version,
            );
        }
        self.execute_range_no_proof_branch(0, drive, transaction, platform_version)
    }

    /// One branch's in-bound page — the entire pre-`IN` executor,
    /// parameterized by which prefix branch's terminal tree it walks.
    fn execute_range_no_proof_branch(
        &self,
        branch: usize,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<RankedEntry>, Error> {
        let grove_version = &platform_version.drive.grove_version;
        let path = self.indexed_property_name_tree_path(branch)?;
        let axis = self.bounds.axis();
        let (lo, hi) = self.bounds.inclusive_bounds_i128();

        // Costs are destructured away rather than `.unwrap()`-ed, same
        // as the ranked executors: `CostContext::unwrap` is infallible
        // but reads like a panicking unwrap at the call site.
        let path_query = PathQuery::new_axis(
            path,
            AxisQuery::bounded(axis.into(), lo, hi, self.limit, self.descending).keys_only(),
        );
        let CostContext { value, cost: _ } = drive.grove.run_path_query(
            &path_query,
            true,
            true,
            true,
            QueryResultType::QueryKeyElementPairResultType,
            transaction,
            grove_version,
        );
        let run = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
        let PathQueryRun::AxisKeys { keys, skipped: _ } = run else {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "a keys-only having read returned a different result shape".to_string(),
            )));
        };
        let entries = axis_keys_to_ranked(axis, keys)?;
        if entries.len() > self.limit as usize {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "having {axis:?} read returned {} entries for limit = {}",
                entries.len(),
                self.limit
            ))));
        }
        Ok(entries)
    }

    /// Generate the grovedb indexed-axis range proof for this query.
    ///
    /// The envelope commits the in-range secondary entries, the
    /// primary's root hash, the sibling axes' root hashes, and a
    /// per-ancestor attestation chain up to the grovedb root — so the
    /// client reconstructs the platform root hash from it. The bounds,
    /// direction and limit bind by RECONSTRUCTION: the verifier rebuilds
    /// the same `Bounded` axis `PathQuery` from the request
    /// ([`AxisRangeBounds::inclusive_bounds_i128`]) and re-executes the
    /// proof against it — which is why the bounds are validated rather
    /// than clamped upstream, and why completeness needs no extra
    /// machinery: a Merk range proof over a sorted keyspace commits its
    /// boundaries, so an in-range group the server omitted fails
    /// reconstruction.
    ///
    /// Verified by
    /// [`DriveDocumentHavingQuery::verify_having_range_proof`](crate::query::DriveDocumentHavingQuery::verify_having_range_proof).
    pub fn execute_range_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        // Same fail-closed rule as the ranked prover: grovedb's
        // `prove_query` proves committed state only and cannot see the
        // caller's transaction — single-prefix and branched alike.
        if transaction.is_some() {
            return Err(Error::Drive(DriveError::NotSupported(
                "a having-range proof is generated from committed state only: grovedb's \
                 prove_query cannot see the caller's transaction — commit first",
            )));
        }
        if self.prefix_branches.len() > 1 {
            // One grovedb **branched** envelope — see the ranked
            // executor's multi-branch arm for the shape.
            let grove_version = &platform_version.drive.grove_version;
            let paths = (0..self.prefix_branches.len())
                .map(|branch| self.indexed_property_name_tree_path(branch))
                .collect::<Result<Vec<_>, Error>>()?;
            let (prefix, keys, suffix) = decompose_branch_paths(&paths)?;
            let (lo, hi) = self.bounds.inclusive_bounds_i128();
            let path_query = PathQuery::new_branched_axis(
                prefix,
                keys,
                suffix,
                AxisQuery::bounded(
                    self.bounds.axis().into(),
                    lo,
                    hi,
                    self.limit,
                    self.descending,
                ),
            );
            let CostContext { value, cost: _ } =
                drive.grove.prove_query(&path_query, None, grove_version);
            return value.map_err(|e| Error::GroveDB(Box::new(e)));
        }
        self.execute_range_with_proof_branch(0, drive, platform_version)
    }

    /// One branch's proof — the entire pre-`IN` prover, parameterized by
    /// the prefix branch.
    fn execute_range_with_proof_branch(
        &self,
        branch: usize,
        drive: &Drive,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let grove_version = &platform_version.drive.grove_version;
        let path = self.indexed_property_name_tree_path(branch)?;
        let (lo, hi) = self.bounds.inclusive_bounds_i128();
        let path_query = PathQuery::new_axis_bounded(
            path,
            self.bounds.axis().into(),
            lo,
            hi,
            self.limit,
            self.descending,
        );
        // Same destructure-don't-unwrap rationale as the no-proof arm.
        let CostContext { value, cost: _ } =
            drive.grove.prove_query(&path_query, None, grove_version);
        value.map_err(|e| Error::GroveDB(Box::new(e)))
    }
}
