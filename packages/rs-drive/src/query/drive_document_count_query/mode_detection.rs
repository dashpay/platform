//! Mode detection + operator classification for the count query.
//!
//! Pure functions on the where-clause shape + request flags — no
//! Drive, no contract, no indexes. Used both server-side (to pick an
//! executor) and verifier-side (to validate the request before
//! attempting verification).

use super::super::conditions::{WhereClause, WhereOperator};
use super::{CountMode, DocumentCountMode, DriveDocumentCountQuery};
#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::query::QuerySyntaxError;

impl DriveDocumentCountQuery<'_> {
    /// Returns `true` if the where-clause operator is one the count fast path
    /// can serve via point-lookups in a CountTree.
    ///
    /// Today that's `Equal` (one path) and `In` (cartesian fork over the listed
    /// values). Range operators (`>`, `<`, `Between*`, `StartsWith`) need a
    /// boundary walk that the current PathQuery infrastructure cannot express;
    /// callers detect those via [`Self::has_unsupported_operator`] and surface
    /// an error instead of silently returning a wrong count.
    ///
    /// `pub(super)` so the sibling [`index_picker`](super::index_picker) module
    /// can call it from `Self::is_indexable_for_count`; not part of the public
    /// API.
    pub(super) fn is_indexable_for_count(op: WhereOperator) -> bool {
        matches!(op, WhereOperator::Equal | WhereOperator::In)
    }

    /// Returns `true` if `op` is a range operator that can be served by a
    /// `range_countable` index walking the property-name `ProvableCountTree`'s
    /// children. The non-prefix portion of a range count query carries
    /// exactly one range operator on the index's last property.
    pub fn is_range_operator(op: WhereOperator) -> bool {
        matches!(
            op,
            WhereOperator::GreaterThan
                | WhereOperator::GreaterThanOrEquals
                | WhereOperator::LessThan
                | WhereOperator::LessThanOrEquals
                | WhereOperator::Between
                | WhereOperator::BetweenExcludeBounds
                | WhereOperator::BetweenExcludeLeft
                | WhereOperator::BetweenExcludeRight
                | WhereOperator::StartsWith
        )
    }

    /// Returns `true` if any where clause uses an operator the count fast path
    /// cannot serve. Callers should treat this as a query-rejection signal.
    pub fn has_unsupported_operator(where_clauses: &[WhereClause]) -> bool {
        where_clauses
            .iter()
            .any(|wc| !Self::is_indexable_for_count(wc.operator))
    }

    /// Classify a count query's mode from its where clauses + request flags.
    ///
    /// This is the protocol-version-agnostic shape detection that
    /// decides which executor (one of the six
    /// [`DocumentCountMode`] variants — `Total` / `PerInValue` /
    /// `RangeNoProof` / `RangeProof` / `RangeDistinctProof` /
    /// `PointLookupProof`) the request maps to. The returned
    /// `DocumentCountMode` discriminates among the dispatcher's
    /// match arms; concrete pagination / index-picker inputs flow
    /// through the call sites separately.
    ///
    /// All validation that depends only on the where clauses + flags
    /// (multiple range clauses, range mixed with `In`, distinct mode on
    /// the prove path, distinct mode without a range clause, etc.) is
    /// done here and surfaces as
    /// [`QuerySyntaxError::InvalidWhereClauseComponents`]. Validation
    /// that depends on the contract's index set (no covering index)
    /// stays at the call site since it requires the
    /// `&BTreeMap<String, Index>`.
    ///
    /// **Versioning note**: this function is the v0 routing table.
    /// Production callers go through [`Self::detect_mode_versioned`]
    /// (which dispatches on
    /// `platform_version.drive.methods.document.query.detect_count_mode`)
    /// so future protocol versions can change the routing table behind
    /// a method-version bump. Tests that want to pin the v0 contract
    /// directly call this function.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn detect_mode(
        where_clauses: &[WhereClause],
        mode: CountMode,
        prove: bool,
    ) -> Result<DocumentCountMode, QuerySyntaxError> {
        // The caller-supplied `CountMode` is the SQL-shape contract
        // (Aggregate / GroupByIn / GroupByRange / GroupByCompound).
        // Translate it back to the "distinct walk required?" boolean
        // the per-mode tuple match below was written against. Pure
        // mechanical mapping: the distinct walk is what the two
        // range-grouped variants need.
        let distinct = mode.requires_distinct_walk();
        // Reject any operator that's neither an indexable point operator
        // (Equal/In) nor a range operator. Defense-in-depth: the request
        // shape forbids these elsewhere, but folding the check in here
        // keeps the mode-detection contract self-contained.
        //
        // `startsWith` IS in `is_range_operator` and routes through the
        // same `Range(a..b)` path as `betweenExcludeRight` — the
        // half-open upper bound is computed by byte-incrementing the
        // serialized prefix's last byte (see `range_clause_to_query_item`,
        // mirroring `conditions.rs:1129`'s normal-docs encoding).
        for wc in where_clauses {
            if !Self::is_indexable_for_count(wc.operator) && !Self::is_range_operator(wc.operator) {
                return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                    "count query supports only `==`, `in`, and range operators",
                ));
            }
        }

        let range_count = where_clauses
            .iter()
            .filter(|wc| Self::is_range_operator(wc.operator))
            .count();
        let in_count = where_clauses
            .iter()
            .filter(|wc| wc.operator == WhereOperator::In)
            .count();

        // `range_count > 1` is rejected for every shape EXCEPT the
        // carrier-ACOR with outer range (G8): when the caller asks
        // for `GroupByRange` + `prove = true` and there are exactly
        // two range clauses on different fields (one on the index's
        // first property — the carrier — and one on the terminator —
        // the inner ACOR), grovedb's carrier-subquery composition
        // ([PR #663](https://github.com/dashpay/grovedb/pull/663)
        // shipped the carrier shape; [PR #664](https://github.com/dashpay/grovedb/pull/664)
        // permits `SizedQuery::limit` on it) makes this a well-formed
        // single-proof shape. The two ranges must be on distinct
        // fields — combining two-sided ranges on the same field still
        // routes through `between*` as before.
        if range_count > 1 {
            let two_ranges_on_distinct_fields = where_clauses
                .iter()
                .filter(|wc| Self::is_range_operator(wc.operator))
                .map(|wc| wc.field.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == range_count;
            if !(prove
                && matches!(mode, CountMode::GroupByRange)
                && range_count == 2
                && in_count == 0
                && two_ranges_on_distinct_fields)
            {
                return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                    "count query supports at most one range where-clause; combine \
                     two-sided ranges via `between*` instead of separate `>` / `<` \
                     clauses, or use `group_by = [outer_range_field]` with `prove = \
                     true` for the carrier-aggregate shape with one outer range and \
                     one inner ACOR range on a different field",
                ));
            }
        }
        if in_count > 1 {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "count query supports at most one `in` where-clause; the In carries \
                 the split property and only one split dimension is supported per request",
            ));
        }

        let has_range = range_count == 1;
        let has_in = in_count == 1;

        // `range + In` on the prove path used to be rejected
        // wholesale because grovedb's `AggregateCountOnRange`
        // primitive wraps a single inner range and historically
        // couldn't cartesian-fork. As of grovedb PR #663 it CAN
        // appear under outer `Keys` via the carrier-subquery
        // composition, so the rejection is now conditional:
        // - `CountMode::GroupByIn` (single-field group_by on the
        //   In field): routes to the new
        //   `DocumentCountMode::RangeAggregateCarrierProof` —
        //   produces one (in_key, u64) pair per In branch via a
        //   carrier-ACOR PathQuery (see
        //   [`super::path_query::carrier_aggregate_count_path_query`]).
        // - `CountMode::GroupByRange` / `GroupByCompound` (distinct
        //   modes): route to `RangeDistinctProof` as before.
        // - `CountMode::Aggregate` (no group_by): still rejected.
        //   With no group_by the caller asks for a single summed
        //   count across all In branches, but the carrier-ACOR
        //   primitive returns one count per branch — collapsing
        //   that back to a single sum at the verifier requires
        //   trusting the SDK to add them, which is exactly what
        //   the prove path is supposed to avoid (the verifier
        //   should get the consensus-committed answer, not
        //   compute a derived one).
        if has_range && has_in && prove && !distinct && !matches!(mode, CountMode::GroupByIn) {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "range count queries with an `in` clause are not supported on the \
                 aggregate prove path; use `group_by = [in_field]` for the carrier \
                 per-In-aggregate proof (one u64 per branch), `group_by = [in_field, \
                 range_field]` for the compound distinct-prove path (per-distinct-\
                 value entries), or `prove = false` for the no-proof variant",
            ));
        }

        if distinct && !has_range && range_count != 2 {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "GROUP BY on a range field requires a range where-clause; the \
                 range field must appear in `where` for the distinct walk to \
                 have a window to iterate over",
            ));
        }

        // G8 short-circuit: `GroupByRange + prove` with exactly two
        // range clauses on distinct fields routes to the carrier
        // ACOR shape. One range becomes the outer dimension of the
        // carrier walk; the other range becomes the inner ACOR
        // target. `SizedQuery::limit` caps the outer walk (per
        // [grovedb PR #664](https://github.com/dashpay/grovedb/pull/664)).
        // The validity of "which range is the outer / which is the
        // inner" depends on the picked index — checked in the
        // path-query builder against the index's property order.
        if prove && matches!(mode, CountMode::GroupByRange) && range_count == 2 && in_count == 0 {
            return Ok(DocumentCountMode::RangeAggregateCarrierProof);
        }

        Ok(match (has_range, has_in, prove, distinct) {
            // Range + prove + distinct (with or without In on
            // prefix): per-distinct-value counts come from a
            // regular range proof against the property-name
            // `ProvableCountTree`. With In on prefix the path
            // query uses grovedb's subquery primitive to
            // cartesian-fork; the verifier walks the same
            // compound shape.
            (true, _, true, true) => DocumentCountMode::RangeDistinctProof,
            // Range + prove + summed (no In): `AggregateCountOnRange`
            // collapse — single u64 verified out. The In case is
            // rejected above.
            (true, false, true, false) => DocumentCountMode::RangeProof,
            // Range + no-proof: the executor uses the same
            // `distinct_count_path_query` builder; In on prefix
            // forks via grovedb subquery at execution time. Sum
            // vs. distinct comes from `RangeCountOptions.distinct`
            // applied to the merged result.
            (true, _, false, _) => DocumentCountMode::RangeNoProof,
            (false, true, false, _) => DocumentCountMode::PerInValue,
            // `In` + `prove = true` (no range): route to the
            // CountTree-element proof path. The shared
            // `point_lookup_count_path_query` builder emits one
            // `Element::CountTree` per matched In branch (via
            // outer `Key`s + `[0]` subquery); the SDK's
            // `verify_point_lookup_count_proof` extracts
            // `count_value_or_default()` from each verified
            // element, and the `FromProof<DocumentQuery>` impl
            // for `DocumentSplitCounts` returns them as
            // per-In-value entries. Proof size is O(|In values|
            // × log n) — no document materialization, no
            // `u16::MAX` cap on matching docs.
            (false, true, true, _) => DocumentCountMode::PointLookupProof,
            // No range, no In, `prove = true`: same CountTree-
            // element proof shape — either the documents_countable
            // primary-key CountTree fast path (empty where) or
            // a single per-branch CountTree element for an
            // Equal-only fully-covered query.
            (false, false, true, _) => DocumentCountMode::PointLookupProof,
            (false, false, false, _) => DocumentCountMode::Total,
            // Range + In + prove + !distinct: the GroupByIn case
            // routes to the new carrier-ACOR proof shape (grovedb
            // PR #663); the Aggregate case is rejected above.
            (true, true, true, false) if matches!(mode, CountMode::GroupByIn) => {
                DocumentCountMode::RangeAggregateCarrierProof
            }
            (true, true, true, false) => {
                unreachable!("range + In + prove + Aggregate is rejected before the dispatch match")
            }
        })
    }

    /// Versioned dispatcher around [`Self::detect_mode`]. Production
    /// callers (the count dispatcher and the SDK's
    /// `verify_count_query` helper) go through this so a future
    /// protocol version can adjust the routing table behind a
    /// method-version bump without an API churn for tests.
    ///
    /// Routes through
    /// `platform_version.drive.methods.document.query.detect_count_mode`.
    /// Today only `0` is defined and maps to [`Self::detect_mode`]
    /// verbatim.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn detect_mode_versioned(
        where_clauses: &[WhereClause],
        mode: CountMode,
        prove: bool,
        platform_version: &dpp::version::PlatformVersion,
    ) -> Result<DocumentCountMode, QuerySyntaxError> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .detect_count_mode
        {
            0 => Self::detect_mode(where_clauses, mode, prove),
            version => Err(QuerySyntaxError::Unsupported(format!(
                "detect_count_mode: unknown method version {version}; only 0 is supported"
            ))),
        }
    }
}
