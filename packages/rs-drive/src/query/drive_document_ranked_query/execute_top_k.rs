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

use super::{DriveDocumentRankedQuery, RankedAxis, RankedEntry, RankedEntryValue, RankedPage};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use grovedb_costs::CostContext;

impl DriveDocumentRankedQuery<'_> {
    /// Read one page of the ranking directly from the axis secondary:
    /// the `k` groups starting at rank `offset`. Entries come back in
    /// ranking order — see [`DriveDocumentRankedQuery::descending`] for
    /// the direction and the tie contract.
    ///
    /// Fewer than `k` entries is normal (the index simply has fewer
    /// groups than `offset + k`) and is not an error. A missing path
    /// *is* an error rather than an empty result: the indexed
    /// property-name tree is created when the contract is registered, so
    /// its absence means the contract-level state is not what the
    /// request claims, not that the ranking is empty. (An index with no
    /// documents yet has the tree, with an empty secondary, and yields
    /// an empty entry list.)
    ///
    /// The paginated grovedb primitive is used unconditionally, with
    /// `offset = 0` standing in for an unpaginated request, so the
    /// no-proof and prove paths read the same code path in grovedb and
    /// cannot drift on the walk's semantics for offset-free queries.
    ///
    /// [`RankedPage::skipped`] on this path is the *requested* offset:
    /// grovedb's read API returns an empty vector when the walk runs out
    /// during the skip and does not report how far it got, so an
    /// unproven read cannot distinguish "skipped exactly `offset`" from
    /// "the secondary holds fewer than `offset` groups". Only the proved
    /// path attests the true value — see [`RankedPage::skipped`].
    pub fn execute_top_k_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<RankedPage, Error> {
        let grove_version = &platform_version.drive.grove_version;
        let path = self.indexed_property_name_tree_path()?;
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
        let offset = self.offset as u64;

        // Costs are destructured away rather than `.unwrap()`-ed:
        // `CostContext::unwrap` is infallible (it drops the cost field)
        // but reads like a panicking unwrap at the call site. The
        // dispatcher wraps these executors with its own fee accounting,
        // exactly as the count surface's `execute_range_count_no_proof`
        // does.
        let entries = match self.axis {
            RankedAxis::Count => {
                let CostContext { value, cost: _ } = drive.grove.indexed_count_top_k_paginated(
                    path_refs.as_slice(),
                    self.k,
                    offset,
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
                let CostContext { value, cost: _ } = drive.grove.indexed_sum_top_k_paginated(
                    path_refs.as_slice(),
                    self.k,
                    offset,
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
                let CostContext { value, cost: _ } = drive.grove.indexed_avg_top_k_paginated(
                    path_refs.as_slice(),
                    self.k,
                    offset,
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
        Ok(RankedPage {
            skipped: offset,
            entries,
        })
    }

    /// Generate the grovedb indexed-axis paginated top-k proof for this
    /// query.
    ///
    /// The envelope commits the walked secondary entries, the number of
    /// entries skipped to reach them, the primary's root hash, the
    /// sibling axes' root hashes, and a per-ancestor attestation chain
    /// up to the grovedb root — so the client reconstructs the platform
    /// root hash from it. It also echoes `(axis, k, offset,
    /// descending)`, which
    /// [`grovedb::GroveDb::verify_indexed_axis_top_k_paginated`]
    /// re-checks against what the client asked for; that is why `k` is
    /// validated rather than clamped upstream (a clamped `k` would
    /// produce a proof the client's own reconstruction rejects).
    ///
    /// The paginated primitive is used unconditionally, with
    /// `offset = 0` for offset-free requests, so there is exactly one
    /// proof shape on this surface: a client never has to guess which of
    /// two envelope formats a server produced.
    ///
    /// Verified by
    /// [`DriveDocumentRankedQuery::verify_ranked_top_k_proof`](crate::query::DriveDocumentRankedQuery::verify_ranked_top_k_proof).
    ///
    /// # Empty rankings prove fine
    ///
    /// An index holding no documents has an empty axis secondary. The
    /// older non-paginated prover refused that outright ("Cannot create
    /// proof for empty tree"), which made a freshly registered contract
    /// unqueryable with `prove = true`; the paginated prover emits a
    /// guaranteed-empty range against the secondary instead, so the
    /// proved and unproven paths agree on empty state. Pinned by the
    /// `ranking_an_empty_index_reads_empty_and_proves_empty` test.
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
        let CostContext { value, cost: _ } = drive.grove.prove_indexed_axis_top_k_paginated(
            path_refs.as_slice(),
            self.axis.into(),
            self.k,
            self.offset as u64,
            self.descending,
            transaction,
            grove_version,
        );
        value.map_err(|e| Error::GroveDB(Box::new(e)))
    }
}
