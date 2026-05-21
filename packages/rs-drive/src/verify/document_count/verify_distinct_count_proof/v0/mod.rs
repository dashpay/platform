use crate::error::Error;
use crate::query::{DriveDocumentCountQuery, SplitCountEntry, WhereOperator};
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

        let mut out: Vec<SplitCountEntry> = Vec::with_capacity(elements.len());
        for (path, key, elem) in elements {
            if let Some(e) = elem {
                let count = e.count_value_or_default();
                if count == 0 {
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
