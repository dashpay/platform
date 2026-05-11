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
    /// For the compound shapes (`In` on the last property, or `In` on
    /// the before-last property with a trailing Equal) the In value
    /// sits at `path[base_path_len]` — the first extra path segment
    /// beyond the path query's `path`. Both shapes stop the
    /// `base_path` at the In-bearing property's property-name subtree
    /// (see [`Self::point_lookup_count_path_query`]), so the In value
    /// lands at the same offset whether or not a trailing Equal is
    /// also part of the descent. For the Equal-only shape the emitted
    /// path equals `path_query.path` so the entry's `key` stays empty.
    ///
    /// `GroveDb::verify_query` is appropriate here for the same reason
    /// as the distinct-count verifier: because each branch's count is
    /// returned as its own entry, a missing `Key` branch (no documents
    /// at that In value) surfaces as a missing entry rather than a
    /// wrong total — the caller can detect "I asked for 3 In values
    /// but got entries for 2" directly. We don't need
    /// `absence_proofs_for_non_existing_searched_keys: true` for
    /// soundness; it would be a useful future addition for "prove this
    /// In value has zero entries" but isn't required for the unmerged
    /// per-branch contract.
    #[inline(always)]
    pub(super) fn verify_point_lookup_count_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<SplitCountEntry>), Error> {
        let path_query = self.point_lookup_count_path_query(platform_version)?;
        let base_path_len = path_query.path.len();
        // Set once an `In` clause is present anywhere on the covering
        // index — both supported In positions (last and before-last)
        // produce the same `base_path_len`-prefixed compound shape.
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
            let Some(e) = elem else { continue };
            let count = e.count_value_or_default();
            if count == 0 {
                continue;
            }
            // Compound shape (In on last or before-last): the In
            // value sits at `path[base_path_len]` — the first extra
            // segment past the path query's base path. For the In-
            // on-before-last shape the descent continues through
            // `[trailing_prop_name, trailing_value, 0]` but the In
            // value is still at the same offset because the path
            // query's base path stops at the In-bearing property's
            // property-name subtree in both shapes. Equal-only shape:
            // the emitted path equals `path_query.path` (no extra
            // segments) so the `key` field is empty.
            let key = if has_in_clause && path.len() > base_path_len {
                path[base_path_len].clone()
            } else {
                Vec::new()
            };
            out.push(SplitCountEntry {
                in_key: None,
                key,
                count,
            });
        }
        Ok((root_hash, out))
    }
}
