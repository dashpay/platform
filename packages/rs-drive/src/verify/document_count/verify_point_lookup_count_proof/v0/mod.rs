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
    /// triples. The path query built by
    /// [`Self::point_lookup_count_path_query`] does NOT set
    /// `absence_proofs_for_non_existing_searched_keys: true`, so under
    /// the current shape:
    ///
    /// - **Present branches** → `Some(element)` triples →
    ///   `Some(element.count_value_or_default())` on the entry.
    /// - **Absent branches** (queried In value with no CountTree
    ///   element in the merk tree) → silently omitted from the
    ///   elements stream. Callers detect "queried but absent" by
    ///   diffing the request's In array against the returned entries.
    ///   See `tests::test_point_lookup_proof_omits_absent_in_branches_from_entries`
    ///   for the end-to-end contract pin.
    ///
    /// The `elem.map(...)` below preserves grovedb's `Option<Element>`
    /// shape so a future variant that flips
    /// `absence_proofs_for_non_existing_searched_keys: true` surfaces
    /// absent branches as `count: None` — distinguishable from
    /// `Some(0)` (which a zero-count branch would never produce on its
    /// own since zero-count CountTree elements aren't materialized in
    /// merk). Today that branch is forward-compatible code, not active
    /// behavior.
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
            //   `None`          → `None` (not produced by today's
            //                     path query — see fn docstring;
            //                     forward-compat for an absence-proof
            //                     variant).
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
