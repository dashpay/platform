//! The two ranked executors on [`DriveDocumentRankedQuery`]: a direct
//! read of the axis secondary, and generation of the equivalent proof.
//!
//! Both are thin — all of the work happens inside grovedb, which walks
//! the pre-sorted secondary Merk directly. That is the whole point of the
//! ranked surface: no value trees are opened, no documents are
//! materialized, and the cost is `O(log n + k)` rather than
//! `O(groups × log n)`.
//!
//! Whole module is gated `feature = "server"` via the parent's
//! `pub mod execute_top_k;` declaration.

use super::{DriveDocumentRankedQuery, RankedAxis, RankedEntry, RankedEntryValue};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use grovedb_costs::CostContext;

impl DriveDocumentRankedQuery<'_> {
    /// Read the top / bottom `k` groups directly from the axis
    /// secondary. Entries come back in ranking order — see
    /// [`DriveDocumentRankedQuery::descending`] for the direction and the
    /// tie contract.
    ///
    /// Fewer than `k` entries is normal (the index simply has fewer
    /// groups) and is not an error. A missing path *is* an error rather
    /// than an empty result: the indexed property-name tree is created
    /// when the contract is registered, so its absence means the
    /// contract-level state is not what the request claims, not that the
    /// ranking is empty. (An index with no documents yet has the tree,
    /// with an empty secondary, and yields an empty entry list.)
    pub fn execute_top_k_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<RankedEntry>, Error> {
        let grove_version = &platform_version.drive.grove_version;
        let path = self.indexed_property_name_tree_path()?;
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();

        // Costs are destructured away rather than `.unwrap()`-ed:
        // `CostContext::unwrap` is infallible (it drops the cost field)
        // but reads like a panicking unwrap at the call site. The
        // dispatcher wraps these executors with its own fee accounting,
        // exactly as the count surface's `execute_range_count_no_proof`
        // does.
        let entries = match self.axis {
            RankedAxis::Count => {
                let CostContext { value, cost: _ } = drive.grove.indexed_count_top_k(
                    path_refs.as_slice(),
                    self.k,
                    self.descending,
                    transaction,
                    grove_version,
                );
                value
                    .map_err(|e| Error::GroveDB(Box::new(e)))?
                    .into_iter()
                    .map(|(count, key)| RankedEntry {
                        key,
                        value: RankedEntryValue::Count(count),
                    })
                    .collect::<Vec<_>>()
            }
            RankedAxis::Sum => {
                let CostContext { value, cost: _ } = drive.grove.indexed_sum_top_k(
                    path_refs.as_slice(),
                    self.k,
                    self.descending,
                    transaction,
                    grove_version,
                );
                value
                    .map_err(|e| Error::GroveDB(Box::new(e)))?
                    .into_iter()
                    .map(|(sum, key)| RankedEntry {
                        key,
                        value: RankedEntryValue::Sum(sum),
                    })
                    .collect::<Vec<_>>()
            }
            RankedAxis::Avg => {
                let CostContext { value, cost: _ } = drive.grove.indexed_avg_top_k(
                    path_refs.as_slice(),
                    self.k,
                    self.descending,
                    transaction,
                    grove_version,
                );
                value
                    .map_err(|e| Error::GroveDB(Box::new(e)))?
                    .into_iter()
                    .map(|(avg, key)| RankedEntry {
                        key,
                        value: RankedEntryValue::AvgFixedPoint(avg),
                    })
                    .collect::<Vec<_>>()
            }
        };

        // `k` is the contract with the caller, and on the prove path it
        // is re-checked inside the proof envelope. Asserting it here too
        // keeps the no-proof and prove responses shape-identical: a
        // caller must never see an over-long list from one path and a
        // capped one from the other.
        if entries.len() > self.k as usize {
            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "ranked {:?} read returned {} entries for k = {}",
                self.axis,
                entries.len(),
                self.k
            ))));
        }
        Ok(entries)
    }

    /// Generate the grovedb indexed-axis top-k proof for this query.
    ///
    /// The envelope commits the walked secondary entries, the primary's
    /// root hash, the sibling axes' root hashes, and a per-ancestor
    /// attestation chain up to the grovedb root — so the client
    /// reconstructs the platform root hash from it. It also echoes
    /// `(axis, k, descending)`, which
    /// [`grovedb::GroveDb::verify_indexed_axis_top_k`] re-checks against
    /// what the client asked for; that is why `k` is validated rather
    /// than clamped upstream (a clamped `k` would produce a proof the
    /// client's own reconstruction rejects).
    ///
    /// Verified by
    /// [`DriveDocumentRankedQuery::verify_ranked_top_k_proof`](crate::query::DriveDocumentRankedQuery::verify_ranked_top_k_proof).
    ///
    /// # Known limitation: an empty ranking cannot be proved
    ///
    /// When the index holds no documents at all, its axis secondary is an
    /// empty Merk and grovedb's prover fails with "Cannot create proof for
    /// empty tree" — there is no absence-proof shape for "this ranking has
    /// no entries". The unproven path returns an empty list for the same
    /// state, so the two paths disagree exactly in the empty case. This is
    /// reachable by any client querying a freshly registered contract with
    /// `prove = true`; callers that must tolerate empty state either read
    /// unproven or treat that specific error as "empty". Pinned by the
    /// `ranking_an_empty_index_reads_empty_but_cannot_be_proved` test,
    /// which flips to a successful round trip the day grovedb grows an
    /// empty-tree envelope.
    pub fn execute_top_k_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let grove_version = &platform_version.drive.grove_version;
        let path = self.indexed_property_name_tree_path()?;
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();

        // Same destructure-don't-unwrap rationale as the no-proof arm.
        let CostContext { value, cost: _ } = drive.grove.prove_indexed_axis_top_k(
            path_refs.as_slice(),
            self.axis.into(),
            self.k,
            self.descending,
            transaction,
            grove_version,
        );
        value.map_err(|e| Error::GroveDB(Box::new(e)))
    }
}
