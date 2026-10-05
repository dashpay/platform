use crate::error::Error;
use crate::query::drive_document_count_query::counter_sum_entry_as_count_entry;
use crate::query::{
    index_keeps_empty_groups, DriveDocumentCountQuery, SplitCountEntry, WhereOperator,
};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentCountQuery<'_> {
    /// v0 of [`Self::verify_distinct_count_proof`].
    ///
    /// Rebuilds the same `PathQuery` the prover used via
    /// [`Self::distinct_count_path_query`] (including `limit` and
    /// `left_to_right` — both are encoded into the path query
    /// bytes), feeds it through `GroveDb::verify_query`, then walks
    /// the verified `(path, key, Option<Element>)` triples to build
    /// the per-`(in_key, key)` entry list.
    ///
    /// For compound queries (`In` on prefix) the In value sits at
    /// `path[base_path_len]` (the first extra path segment beyond
    /// the path query's `path`); for flat queries the emitted path
    /// equals `path_query.path`, so `in_key` stays `None`.
    ///
    /// Cross-fork aggregation is intentionally NOT done here —
    /// callers reduce by `key` client-side if they want a flat
    /// histogram. See [`SplitCountEntry`]'s doc for the no-merge
    /// rationale.
    ///
    /// `GroveDb::verify_query` is appropriate here for both flat and
    /// compound shapes:
    /// - For flat queries (no `In` on prefix) the path query has a
    ///   single range `QueryItem` and no explicit `Key` items;
    ///   range items can't be enumerated for absence checks anyway
    ///   (`Query::terminal_keys_inner` errors `NotSupported` on
    ///   unbounded ranges).
    /// - For compound queries (`In` on prefix) the outer Query has
    ///   explicit `Key` items per In value, but because we don't sum
    ///   across forks, a missing `Key` branch surfaces as missing
    ///   entries with that `in_key` rather than as a wrong total —
    ///   the caller can detect "I asked for 3 In values but only
    ///   got entries for 2" directly. We don't need
    ///   `absence_proofs_for_non_existing_searched_keys: true` for
    ///   soundness; it would be a useful future addition for
    ///   "prove this In value has zero entries" but isn't required.
    #[inline(always)]
    pub(super) fn verify_distinct_count_proof_v0(
        &self,
        proof: &[u8],
        limit: u16,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<SplitCountEntry>), Error> {
        // A `summableOffCountIndex` index's documents are its range sums
        // (`counter_sums_query`), proved by the sum surface. Edited in place in
        // this shipped generation: only meta-schema v3 (protocol version 14)
        // admits such an index, so every earlier version verifies as before.
        if let Some(sums) = self.counter_sums_query() {
            let (root_hash, entries) =
                sums.verify_distinct_sum_proof(proof, limit, left_to_right, platform_version)?;
            // A preallocated counter at zero stays, as a count of zero: the
            // proof's limit counted it, so dropping it would end a page early
            // (the unproven read keeps it too).
            return Ok((
                root_hash,
                entries
                    .into_iter()
                    .map(counter_sum_entry_as_count_entry)
                    .collect(),
            ));
        }
        let path_query =
            self.distinct_count_path_query(Some(limit), left_to_right, platform_version)?;
        let base_path_len = path_query.path.len();
        let has_in_on_prefix = self
            .where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let keeps_empty_groups = index_keeps_empty_groups(self.document_type, self.index);
        let mut out: Vec<SplitCountEntry> = Vec::with_capacity(elements.len());
        for (path, key, elem) in elements {
            if let Some(e) = elem {
                let count = e.count_value_or_default();
                // An empty group a preallocation or an outliving index left
                // stays, as a count of zero: the proof's limit counted it (the
                // unproven read keeps it too). Edited in place in this shipped
                // generation: only meta-schema v3 (protocol version 14) admits
                // `preallocated` and `outlivesDelete`, so every earlier
                // version verifies as before.
                if count == 0 && !keeps_empty_groups {
                    continue;
                }
                let in_key = if has_in_on_prefix && path.len() > base_path_len {
                    Some(path[base_path_len].clone())
                } else {
                    None
                };
                // Distinct-count proof emits one entry per
                // verified `KVCount` op in the proof — always
                // `Some(_)`. SDK-side synthesis can add `None`
                // entries for missing-from-proof keys if the
                // caller's request named them (only meaningful
                // for In-grouped paths; range-distinct doesn't
                // enumerate keys in advance).
                out.push(SplitCountEntry {
                    in_key,
                    key,
                    count: Some(count),
                });
            }
        }
        Ok((root_hash, out))
    }
}
