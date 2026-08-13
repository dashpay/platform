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
    decompose_branch_paths, merge_branch_pages,
};
use super::super::drive_document_ranked_query::{RankedAxis, RankedEntry, RankedEntryValue};
use super::{AxisRangeBounds, DriveDocumentHavingQuery};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use grovedb_costs::CostContext;

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
        if self.prefix_branches.len() > 1 {
            // One bounded walk per branch, each fetching up to the full
            // limit (the merge lemma needs every branch's own in-bound
            // prefix), merged with the shared comparator.
            let per_branch = (0..self.prefix_branches.len())
                .map(|branch| {
                    self.execute_range_no_proof_branch(branch, drive, transaction, platform_version)
                })
                .collect::<Result<Vec<_>, Error>>()?;
            return merge_branch_pages(
                per_branch,
                &self.prefix_branches,
                self.descending,
                self.limit as usize,
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
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();

        // Costs are destructured away rather than `.unwrap()`-ed, same
        // as the ranked executors: `CostContext::unwrap` is infallible
        // but reads like a panicking unwrap at the call site.
        let entries = match self.bounds {
            AxisRangeBounds::Count { lo, hi } => {
                let CostContext { value, cost: _ } = drive.grove.indexed_count_range_keys(
                    path_refs.as_slice(),
                    lo,
                    hi,
                    self.descending,
                    self.limit,
                    transaction,
                    grove_version,
                );
                value
                    .map_err(|e| Error::GroveDB(Box::new(e)))?
                    .into_iter()
                    .map(|(count, key)| RankedEntry {
                        in_key: None,
                        key,
                        value: RankedEntryValue::Count(count),
                    })
                    .collect::<Vec<_>>()
            }
            AxisRangeBounds::Sum { lo, hi } => {
                let CostContext { value, cost: _ } = drive.grove.indexed_sum_range_keys(
                    path_refs.as_slice(),
                    lo,
                    hi,
                    self.descending,
                    self.limit,
                    transaction,
                    grove_version,
                );
                value
                    .map_err(|e| Error::GroveDB(Box::new(e)))?
                    .into_iter()
                    .map(|(sum, key)| RankedEntry {
                        in_key: None,
                        key,
                        value: RankedEntryValue::Sum(sum),
                    })
                    .collect::<Vec<_>>()
            }
            AxisRangeBounds::Avg { lo, hi } => {
                let CostContext { value, cost: _ } = drive.grove.indexed_avg_range_keys(
                    path_refs.as_slice(),
                    lo,
                    hi,
                    self.descending,
                    self.limit,
                    transaction,
                    grove_version,
                );
                value
                    .map_err(|e| Error::GroveDB(Box::new(e)))?
                    .into_iter()
                    .map(|(avg, key)| RankedEntry {
                        in_key: None,
                        key,
                        value: RankedEntryValue::AvgFixedPoint(avg),
                    })
                    .collect::<Vec<_>>()
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

    /// Generate the grovedb indexed-axis range proof for this query.
    ///
    /// The envelope commits the in-range secondary entries, the
    /// primary's root hash, the sibling axes' root hashes, and a
    /// per-ancestor attestation chain up to the grovedb root — so the
    /// client reconstructs the platform root hash from it. The Merk
    /// query (the encoded bounds and walk direction) and the limit are
    /// echoed and re-checked by grovedb's verifier against the client's
    /// own reconstruction via [`AxisRangeBounds::merk_query`] — which is
    /// why the bounds are validated rather than clamped upstream, and
    /// why completeness needs no extra machinery: a Merk range proof
    /// over a sorted keyspace commits its boundaries, so an in-range
    /// group the server omitted fails reconstruction.
    ///
    /// Verified by
    /// [`DriveDocumentHavingQuery::verify_having_range_proof`](crate::query::DriveDocumentHavingQuery::verify_having_range_proof).
    pub fn execute_range_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        if self.prefix_branches.len() > 1 {
            // One grovedb **branched** envelope — see the ranked
            // executor's multi-branch arm for the shape.
            let grove_version = &platform_version.drive.grove_version;
            let paths = (0..self.prefix_branches.len())
                .map(|branch| self.indexed_property_name_tree_path(branch))
                .collect::<Result<Vec<_>, Error>>()?;
            let (prefix, keys, suffix) = decompose_branch_paths(&paths)?;
            let prefix_refs: Vec<&[u8]> = prefix.iter().map(|s| s.as_slice()).collect();
            let suffix_refs: Vec<&[u8]> = suffix.iter().map(|s| s.as_slice()).collect();
            let CostContext { value, cost: _ } = drive.grove.prove_indexed_axis_query_branched(
                &prefix_refs,
                &keys,
                &suffix_refs,
                self.bounds.axis().into(),
                self.bounds.merk_query(self.descending),
                Some(self.limit),
                transaction,
                grove_version,
            );
            return value.map_err(|e| Error::GroveDB(Box::new(e)));
        }
        self.execute_range_with_proof_branch(0, drive, transaction, platform_version)
    }

    /// One branch's proof — the entire pre-`IN` prover, parameterized by
    /// the prefix branch.
    fn execute_range_with_proof_branch(
        &self,
        branch: usize,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let grove_version = &platform_version.drive.grove_version;
        let path = self.indexed_property_name_tree_path(branch)?;
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
        let secondary_query = self.bounds.merk_query(self.descending);

        // Same destructure-don't-unwrap rationale as the no-proof arm.
        let CostContext { value, cost: _ } = match self.bounds.axis() {
            RankedAxis::Count => drive.grove.prove_indexed_count_query(
                path_refs.as_slice(),
                secondary_query,
                Some(self.limit),
                transaction,
                grove_version,
            ),
            RankedAxis::Sum => drive.grove.prove_indexed_sum_query(
                path_refs.as_slice(),
                secondary_query,
                Some(self.limit),
                transaction,
                grove_version,
            ),
            RankedAxis::Avg => drive.grove.prove_indexed_avg_query(
                path_refs.as_slice(),
                secondary_query,
                Some(self.limit),
                transaction,
                grove_version,
            ),
        };
        value.map_err(|e| Error::GroveDB(Box::new(e)))
    }
}
