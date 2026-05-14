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
    /// `(path, grove_key, Option<Element>)` triples to build the
    /// per-branch entry list.
    ///
    /// ## Two terminator shapes (kept in sync with the builder)
    ///
    /// The builder's output depends on whether the covering index is
    /// `range_countable: true`:
    ///
    /// - **Normal `countable` (NOT `range_countable`)**: proof targets
    ///   the `Key([0])` CountTree under the terminator's value tree.
    ///   For compound (In) shapes the emitted `path` extends the
    ///   builder's `base_path` with at least
    ///   `[in_value, ..., terminator_value]` and `grove_key = [0]`.
    ///   For Equal-only shapes `path == base_path` and `grove_key =
    ///   [0]`.
    /// - **`range_countable`**: proof targets the terminator's value
    ///   tree directly (the value tree itself IS a CountTree, since
    ///   continuation property-name subtrees beneath it are wrapped
    ///   `Element::NonCounted` so they don't contribute to the
    ///   value-tree count). For In-on-terminator shapes the emitted
    ///   `path` equals `base_path` and the In value lives in
    ///   `grove_key`. For Equal-only shapes the same is true — `path
    ///   == base_path` and `grove_key` holds the terminator value
    ///   (not consumed here, since the Equal-only count has no
    ///   per-key dimension). For In + trailing Equals shapes the
    ///   `path` extends through the trailing Equal `(name, value)`
    ///   pairs and ends at the terminator's property-name segment,
    ///   so the In value sits at `path[base_path_len]` exactly like
    ///   the normal shape; only the bottom layer changes.
    ///
    /// ## In-value extraction
    ///
    /// For compound (In) shapes the In value is the per-branch user-
    /// visible key. The discriminator is `path.len() vs base_path_len`:
    ///
    /// - `path.len() > base_path_len`: the descent walked past
    ///   `base_path` (either through the outer `Key(in_value)` —
    ///   normal countable In-on-terminator — or through the outer
    ///   key + trailing `(name, value)` pairs — compound trailing-
    ///   Equal shapes). The In value sits at `path[base_path_len]`.
    /// - `path.len() == base_path_len`: only reachable for the
    ///   `range_countable` In-on-terminator shape, where no subquery
    ///   is set and the outer `Key(in_value)` resolves to the value
    ///   tree directly. The In value is `grove_key`.
    ///
    /// For Equal-only shapes (`has_in_clause = false`) the per-key
    /// dimension is structurally meaningless and the entry's `key`
    /// stays empty regardless of which terminator shape was used.
    ///
    /// `GroveDb::verify_query` returns `(path, grove_key,
    /// Option<Element>)` triples. The path query built by
    /// [`Self::point_lookup_count_path_query`] does NOT set
    /// `absence_proofs_for_non_existing_searched_keys: true`, so:
    ///
    /// - **Present branches** → `Some(element)` triples →
    ///   `Some(element.count_value_or_default())` on the entry.
    ///   `count_value_or_default()` works uniformly for both
    ///   terminator shapes: for normal countable it reads the `[0]`
    ///   CountTree's count; for `range_countable` it reads the value
    ///   tree's own count.
    /// - **Absent branches** (queried In value with no element in
    ///   the merk tree) → silently omitted from the elements stream.
    ///   Callers detect "queried but absent" by diffing the
    ///   request's In array against the returned entries. See
    ///   `tests::test_point_lookup_proof_omits_absent_in_branches_from_entries`
    ///   for the end-to-end contract pin.
    ///
    /// The `elem.map(...)` below preserves grovedb's `Option<Element>`
    /// shape so a future variant that flips
    /// `absence_proofs_for_non_existing_searched_keys: true` surfaces
    /// absent branches as `count: None`.
    #[inline(always)]
    pub(super) fn verify_point_lookup_count_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<SplitCountEntry>), Error> {
        let path_query = self.point_lookup_count_path_query(platform_version)?;
        let base_path_len = path_query.path.len();
        // Set once an `In` clause is present anywhere on the covering
        // index. The In value's emission offset depends on the
        // terminator shape (see in-value-extraction section of this
        // fn's docstring); we discriminate inline via `path.len()
        // == base_path_len`.
        let has_in_clause = self
            .where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let mut out: Vec<SplitCountEntry> = Vec::with_capacity(elements.len());
        for (path, grove_key, elem) in elements {
            // For compound (In) shapes the In value is at:
            // - `path[base_path_len]` when the descent walked past
            //   `base_path` (normal countable In-on-terminator, or
            //   either-terminator-shape's In + trailing Equals);
            // - `grove_key` when no descent happened beyond
            //   `base_path` (the range_countable In-on-terminator
            //   shape, where outer `Key(in_value)` resolves to the
            //   value tree directly with no subquery).
            //
            // For Equal-only shapes (`has_in_clause = false`) the
            // entry has no per-key dimension; `key` stays empty.
            let key = if has_in_clause {
                if path.len() > base_path_len {
                    path[base_path_len].clone()
                } else {
                    // Only the range_countable In-on-terminator
                    // shape lands here — `grove_key` is the
                    // serialized In value.
                    grove_key
                }
            } else {
                Vec::new()
            };
            // Propagate grovedb's `Option<Element>` directly:
            //   `Some(element)` → `Some(count_value_or_default())`
            //   `None`          → `None` (not produced by today's
            //                     path query — see fn docstring;
            //                     forward-compat for an absence-proof
            //                     variant).
            // `count_value_or_default()` works for both terminator
            // shapes (CountTree at `[0]` for normal countable, or
            // the value tree itself for `range_countable`).
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
