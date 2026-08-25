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

use super::branches::{decompose_branch_paths, read_branched_union};
use super::{DriveDocumentRankedQuery, RankedAxis, RankedEntry, RankedEntryValue, RankedPage};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{IndexedTopKKeysPage, PathQuery, TransactionArg};
use grovedb_costs::CostContext;
use grovedb_query::AxisQuery;

impl DriveDocumentRankedQuery<'_> {
    /// Read one page of the ranking directly from the axis secondary:
    /// the `k` groups starting at rank `offset`. Entries come back in
    /// ranking order — see [`DriveDocumentRankedQuery::descending`] for
    /// the direction and the tie contract.
    ///
    /// Fewer than `k` entries is normal (the index simply has fewer
    /// groups than `offset + k`) and is not an error. On an `IN`-pinned
    /// request, an element whose branch chain is missing at ANY depth —
    /// the branch key itself, or any deeper pinned segment under a
    /// *present* key — contributes an **empty branch** (union
    /// semantics), exactly as the proved envelope authenticates it, and
    /// the union is served from **one committed state**: the branched
    /// read always runs under a grovedb snapshot read transaction, so
    /// every per-branch probe and walk reads the same RocksDB snapshot
    /// (a caller transaction is rejected on this shape, mirroring the
    /// branched prover — read per prefix element under a transaction).
    /// A missing path under a single `==` pin *is* an error rather than
    /// an empty result: the indexed property-name tree is created when
    /// the contract is registered, so its absence means the
    /// contract-level state is not what the request claims, not that
    /// the ranking is empty. (An index with no documents yet has the
    /// tree, with an empty secondary, and yields an empty entry list.)
    ///
    /// The paginated grovedb primitive is used unconditionally, with
    /// `offset = 0` standing in for an unpaginated request, so the
    /// no-proof and prove paths read the same code path in grovedb and
    /// cannot drift on the walk's semantics for offset-free queries.
    ///
    /// # The offset is counted, not walked
    ///
    /// grovedb descends the secondary reading each subtree's aggregate
    /// count off its link, and collapses any subtree that fits entirely
    /// inside the remaining offset instead of stepping through it. The
    /// skip therefore costs `O(log n)` at any offset rather than one
    /// iterator step and one decode per skipped entry, and an offset at
    /// or past the population is answered from the root's own count with
    /// no descent at all — the cheapest request on this surface rather
    /// than the most expensive. `offset = 0` keeps the plain iterator
    /// path and never touches the tree, so the common unpaginated
    /// request costs exactly what it always did.
    ///
    /// That is what makes an uncapped `OFFSET` safe rather than merely
    /// tolerated. Ranked queries carry no fee, cannot be cancelled once
    /// dispatched, and share their rate budget with state transitions
    /// rather than having one of their own, so a skip whose cost grew
    /// with the offset would be an unmetered lever for any
    /// unauthenticated caller. It does not grow.
    ///
    /// [`RankedPage::skipped`] comes back from grovedb rather than being
    /// echoed from the request: it is the requested offset when the skip
    /// succeeded, and the secondary's whole population when the walk ran
    /// out of groups first. That is the same quantity the proved path
    /// attests, so the two no longer disagree — though on this path it is
    /// the node's unverified claim rather than an attested value, exactly
    /// like the entries beside it. See [`RankedPage::skipped`].
    pub fn execute_top_k_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<RankedPage, Error> {
        if self.prefix_branches.len() > 1 {
            // ONE grovedb call for the whole union, pinned to one
            // committed state and merged with the shared comparator —
            // the entire sequence lives in
            // `branches::read_branched_union`, shared with the
            // having-range surface so the two cannot drift. `offset` is
            // grammar-rejected with `IN`, so `skipped` is always 0 here.
            let paths = (0..self.prefix_branches.len())
                .map(|branch| self.indexed_property_name_tree_path(branch))
                .collect::<Result<Vec<_>, Error>>()?;
            let entries = read_branched_union(
                &drive.grove,
                "ranked",
                &self.prefix_branches,
                &paths,
                self.axis,
                AxisQuery::top_k(
                    self.axis.into(),
                    self.k,
                    self.offset as u64,
                    self.descending,
                ),
                self.k as usize,
                self.descending,
                transaction,
                &platform_version.drive.grove_version,
            )?;
            return Ok(RankedPage {
                skipped: 0,
                entries,
            });
        }
        self.execute_top_k_no_proof_branch(0, drive, transaction, platform_version)
    }

    /// One branch's page — the entire pre-`IN` executor, parameterized
    /// by which prefix branch's terminal tree it walks.
    fn execute_top_k_no_proof_branch(
        &self,
        branch: usize,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<RankedPage, Error> {
        let grove_version = &platform_version.drive.grove_version;
        let path = self.indexed_property_name_tree_path(branch)?;
        let path_refs: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
        let offset = self.offset as u64;

        // The cost is dropped rather than `.unwrap()`-ed:
        // `CostContext::unwrap` is infallible (it drops the cost field)
        // but reads like a panicking unwrap at the call site. Dropping it
        // is all there is to do with it — nothing meters a query on this
        // surface: neither this executor's caller nor the dispatcher
        // above it accumulates or charges the cost, and no credit is
        // debited for a read. grovedb computes the `OperationCost`
        // because its API always does, and it ends here.
        let (entries, skipped) = match self.axis {
            RankedAxis::Count => {
                let CostContext { value, cost: _ } =
                    drive.grove.indexed_count_top_k_paginated_keys(
                        path_refs.as_slice(),
                        self.k,
                        offset,
                        self.descending,
                        transaction,
                        grove_version,
                    );
                let IndexedTopKKeysPage { entries, skipped } =
                    value.map_err(|e| Error::GroveDB(Box::new(e)))?;
                (
                    entries
                        .into_iter()
                        .map(|(count, key)| RankedEntry {
                            in_key: None,
                            key,
                            value: RankedEntryValue::Count(count),
                        })
                        .collect::<Vec<_>>(),
                    skipped,
                )
            }
            RankedAxis::Sum => {
                let CostContext { value, cost: _ } = drive.grove.indexed_sum_top_k_paginated_keys(
                    path_refs.as_slice(),
                    self.k,
                    offset,
                    self.descending,
                    transaction,
                    grove_version,
                );
                let IndexedTopKKeysPage { entries, skipped } =
                    value.map_err(|e| Error::GroveDB(Box::new(e)))?;
                (
                    entries
                        .into_iter()
                        .map(|(sum, key)| RankedEntry {
                            in_key: None,
                            key,
                            value: RankedEntryValue::Sum(sum),
                        })
                        .collect::<Vec<_>>(),
                    skipped,
                )
            }
            RankedAxis::Avg => {
                let CostContext { value, cost: _ } = drive.grove.indexed_avg_top_k_paginated_keys(
                    path_refs.as_slice(),
                    self.k,
                    offset,
                    self.descending,
                    transaction,
                    grove_version,
                );
                let IndexedTopKKeysPage { entries, skipped } =
                    value.map_err(|e| Error::GroveDB(Box::new(e)))?;
                (
                    entries
                        .into_iter()
                        .map(|(avg, key)| RankedEntry {
                            in_key: None,
                            key,
                            value: RankedEntryValue::AvgFixedPoint(avg),
                        })
                        .collect::<Vec<_>>(),
                    skipped,
                )
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
        Ok(RankedPage { skipped, entries })
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
        if self.prefix_branches.len() > 1 {
            // grovedb's unified `prove_query` proves COMMITTED state only —
            // it takes one internal snapshot of committed state and threads
            // it through every proof layer, and cannot see the caller's
            // transaction. Serving a proof for a different snapshot than the
            // unproved read would silently desynchronize the two paths, so a
            // transactional branched prove fails closed instead — exactly
            // like the branched unproved read.
            if transaction.is_some() {
                return Err(Error::Drive(DriveError::NotSupported(
                    "an IN-pinned (branched) ranked proof is generated from committed state \
                     only: grovedb's unified prove_query cannot see the caller's transaction \
                     — prove per prefix element, or commit first",
                )));
            }
            // One grovedb **branched** envelope: shared ancestor layers
            // once, one multi-key proof at the branching level, one
            // secondary proof per branch — a single proof with a single
            // root hash. The verifier re-derives the branch set from
            // the request, so a dropped, duplicated, or reordered
            // branch fails there.
            let grove_version = &platform_version.drive.grove_version;
            let paths = (0..self.prefix_branches.len())
                .map(|branch| self.indexed_property_name_tree_path(branch))
                .collect::<Result<Vec<_>, Error>>()?;
            let (prefix, keys, suffix) = decompose_branch_paths(&paths)?;
            let path_query = PathQuery::new_branched_axis(
                prefix,
                keys,
                suffix,
                AxisQuery::top_k(
                    self.axis.into(),
                    self.k,
                    self.offset as u64,
                    self.descending,
                ),
            );
            let CostContext { value, cost: _ } =
                drive.grove.prove_query(&path_query, None, grove_version);
            return value.map_err(|e| Error::GroveDB(Box::new(e)));
        }
        self.execute_top_k_with_proof_branch(0, drive, transaction, platform_version)
    }

    /// One branch's proof — the entire pre-`IN` prover, parameterized by
    /// the prefix branch.
    fn execute_top_k_with_proof_branch(
        &self,
        branch: usize,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let grove_version = &platform_version.drive.grove_version;
        let path = self.indexed_property_name_tree_path(branch)?;
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
