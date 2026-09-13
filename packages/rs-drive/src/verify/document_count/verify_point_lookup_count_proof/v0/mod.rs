use crate::error::Error;
use crate::query::drive_document_count_query::point_lookup_count_entries;
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
    /// ## Single terminator shape: value-tree-direct (kept in sync with the builder)
    ///
    /// The insertion side stores **every** countable index's
    /// terminator value tree as a `CountTree` (with sibling
    /// continuations wrapped `Element::NonCounted` so they don't
    /// pollute the parent's count). The builder takes advantage of
    /// this uniformly: proofs target the value tree directly via
    /// `Key(serialized_value)` instead of descending one more layer
    /// to a `Key([0])` CountTree child. The proof is exactly one
    /// merk hash shallower per resolved branch than the legacy `[0]`-
    /// child shape would have been.
    ///
    /// Emitted-path layouts:
    /// - **Equal-only**: `path == base_path` (ends at the
    ///   terminator's property-name segment, e.g. `[..., "color"]`),
    ///   `grove_key = serialized_terminator_value`. The verified
    ///   element is the terminator value tree's CountTree.
    /// - **In-on-terminator**: `path == base_path` (ends at the
    ///   In-bearing prop's name subtree), `grove_key = serialized_In_value`.
    ///   The outer `Key(in_value)` resolves directly to each
    ///   per-In CountTree.
    /// - **In + trailing Equals (terminator is a trailing Equal)**:
    ///   `path` extends through the In value + trailing `(name,
    ///   value)` pairs and ends at the terminator's property-name
    ///   segment; `grove_key = serialized_terminator_value`. The In
    ///   value sits at `path[base_path_len]`.
    ///
    /// ## In-value extraction
    ///
    /// For compound (In) shapes the In value is the per-branch user-
    /// visible key. The discriminator is `path.len() vs base_path_len`:
    ///
    /// - `path.len() > base_path_len`: the descent walked past
    ///   `base_path` through trailing-Equal segments. The In value
    ///   sits at `path[base_path_len]`.
    /// - `path.len() == base_path_len`: only reachable for the
    ///   In-on-terminator shape, where no subquery is set and the
    ///   outer `Key(in_value)` resolves to the value tree directly.
    ///   The In value is `grove_key`.
    ///
    /// For Equal-only shapes (`has_in_clause = false`) the per-key
    /// dimension is structurally meaningless and the entry's `key`
    /// stays empty.
    ///
    /// `GroveDb::verify_query` returns `(path, grove_key,
    /// Option<Element>)` triples. The path query built by
    /// [`Self::point_lookup_count_path_query`] does NOT set
    /// `absence_proofs_for_non_existing_searched_keys: true`, so:
    ///
    /// - **Present branches** → `Some(element)` triples →
    ///   `Some(element.count_value_or_default())` on the entry. The
    ///   element is the terminator value tree's CountTree, whose
    ///   `count_value_or_default()` returns the per-branch doc count
    ///   directly.
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

        // The layout decoder lives with the path-query builder — see
        // `point_lookup_count_entries` for the In-value placement.
        let out = point_lookup_count_entries(base_path_len, has_in_clause, elements);
        Ok((root_hash, out))
    }
}
