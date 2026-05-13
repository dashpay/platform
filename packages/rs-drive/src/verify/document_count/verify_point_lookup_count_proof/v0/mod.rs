use crate::error::Error;
use crate::query::{DriveDocumentCountQuery, SplitCountEntry, WhereOperator};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentCountQuery<'_> {
    /// v0 of [`Self::verify_point_lookup_count_proof`].
    ///
    /// Rebuilds the same `PathQuery` the prover used via
    /// [`Self::point_lookup_count_path_query`], feeds it through
    /// `GroveDb::verify_query`, and walks the verified
    /// `(path, key, Option<Element>)` triples to build the per-branch
    /// entry list.
    ///
    /// For the compound shape (`In` at any index position, with 0..N
    /// trailing Equals afterwards) the In value sits at
    /// `path[base_path_len]` — the first extra path segment beyond
    /// the path query's `path`. The builder stops `base_path` at the
    /// In-bearing property's property-name subtree (see
    /// [`Self::point_lookup_count_path_query`]), regardless of how
    /// many trailing Equals exist, so the In value lands at the same
    /// offset in every compound emission. For the Equal-only shape
    /// the emitted path equals `path_query.path` so the entry's `key`
    /// stays empty.
    ///
    /// `GroveDb::verify_query` returns `(path, key, Option<Element>)`
    /// triples — `Some(element)` for keys that exist in the merk tree,
    /// `None` for queried keys whose merk traversal terminated without
    /// finding the CountTree element. We propagate the `Option`
    /// directly onto [`SplitCountEntry::count`]:
    ///
    /// - `Some(element)` → `Some(element.count_value_or_default())` —
    ///   verified count for an existing branch.
    /// - `None` (grovedb's missing-key signal) → `count: None` — the
    ///   merk path was traversed for this In value but no CountTree
    ///   element was there. Distinct from `Some(0)`: the path query
    ///   doesn't set `absence_proofs_for_non_existing_searched_keys`,
    ///   so this isn't a cryptographic "verified zero docs" — it's
    ///   "the proof was implicit about this branch." Callers that
    ///   want explicit zero-proof bytes should use a future variant
    ///   that flips the flag.
    ///
    /// Crucially, the SDK does NOT need to re-discover missing In
    /// values by comparing the request's In array against the
    /// verifier output — grovedb already enumerates them.
    #[inline(always)]
    pub(super) fn verify_point_lookup_count_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<SplitCountEntry>), Error> {
        let path_query = self.point_lookup_count_path_query(platform_version)?;
        let base_path_len = path_query.path.len();
        // Set once an `In` clause is present anywhere on the covering
        // index — the builder stops `base_path` at the In-bearing
        // property's name subtree regardless of how many trailing
        // Equals descend further, so the In value always sits at
        // `path[base_path_len]` in the compound emission.
        let has_in_clause = self
            .where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let mut out: Vec<SplitCountEntry> = Vec::with_capacity(elements.len());
        for (path, _grove_key, elem) in elements {
            // `_grove_key` is the trailing key on the path (always
            // `[0]` here — the CountTree key under the value tree);
            // we don't store it in the entry because the count's
            // user-visible key is the In value (compound shape) or
            // empty (Equal-only).
            //
            // Compound shape (In at any index position, 0..N
            // trailing Equals afterwards): the In value sits at
            // `path[base_path_len]` — the first extra segment past
            // the path query's base path. When trailing Equals are
            // present the descent continues through
            // `[trailing_prop_name_1, trailing_value_1, ...,
            // trailing_prop_name_n, trailing_value_n, 0]`, but the
            // In value is still at the same offset because
            // `base_path` stops at the In-bearing property's
            // property-name subtree regardless of how many trailing
            // segments follow. Equal-only shape: the emitted path
            // equals `path_query.path` (no extra segments) so the
            // `key` field is empty.
            let key = if has_in_clause && path.len() > base_path_len {
                path[base_path_len].clone()
            } else {
                Vec::new()
            };
            // Propagate grovedb's `Option<Element>` directly:
            //   `Some(element)` → `Some(count_value_or_default())`
            //   `None`          → `None` (queried but proof was
            //                     implicit; not the same as
            //                     `Some(0)` since this path doesn't
            //                     request explicit absence proofs).
            // Zero-count CountTree elements aren't materialized in
            // the merk tree (a CountTree is removed when its last
            // doc is deleted), so `Some(0)` from this branch would
            // mean a malformed proof — pass it through verbatim
            // rather than swallow it.
            let count = elem.map(|e| e.count_value_or_default());
            out.push(SplitCountEntry {
                in_key: None,
                key,
                count,
            });
        }
        Ok((root_hash, out))
    }
}
