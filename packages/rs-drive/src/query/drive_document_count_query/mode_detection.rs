//! Mode detection + operator classification for the count query.
//!
//! Pure functions on the where-clause shape + request flags — no
//! Drive, no contract, no indexes. Used both server-side (to pick an
//! executor) and verifier-side (to validate the request before
//! attempting verification).

use super::super::conditions::{WhereClause, WhereOperator};
use super::{DocumentCountMode, DriveDocumentCountQuery};
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
    /// This is the protocol-version-agnostic shape detection that decides
    /// which executor (Equal/In point lookup, range walk, range proof,
    /// materialize-and-count proof, etc.) the request maps to. The
    /// returned [`DocumentCountMode`] discriminates among the handler's
    /// dispatch arms; concrete pagination / index-picker inputs still
    /// flow through the call sites separately.
    ///
    /// All validation that depends only on the where clauses + flags
    /// (multiple range clauses, range mixed with `In`, distinct mode on
    /// the prove path, distinct mode without a range clause, etc.) is
    /// done here and surfaces as
    /// [`QuerySyntaxError::InvalidWhereClauseComponents`]. Validation
    /// that depends on the contract's index set (no covering index)
    /// stays at the call site since it requires the
    /// `&BTreeMap<String, Index>`.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn detect_mode(
        where_clauses: &[WhereClause],
        return_distinct_counts_in_range: bool,
        prove: bool,
    ) -> Result<DocumentCountMode, QuerySyntaxError> {
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

        if range_count > 1 {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "count query supports at most one range where-clause; combine \
                 two-sided ranges via `between*` instead of separate `>` / `<` clauses",
            ));
        }
        if in_count > 1 {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "count query supports at most one `in` where-clause; the In carries \
                 the split property and only one split dimension is supported per request",
            ));
        }

        let has_range = range_count == 1;
        let has_in = in_count == 1;

        // `range + In` is only rejected on the aggregate prove path
        // (grovedb's `AggregateCountOnRange` primitive wraps a single
        // inner range and can't cartesian-fork over multiple In
        // values at the merk layer — see the comment on
        // `aggregate_count_path_query`). For distinct modes (both
        // no-proof and prove) and for total-range-no-proof, the
        // `distinct_count_path_query` builder handles In on prefix
        // via grovedb's native subquery primitive.
        if has_range && has_in && prove && !return_distinct_counts_in_range {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "range count queries with an `in` clause are not supported on the \
                 aggregate prove path; use `return_distinct_counts_in_range = true` \
                 for compound In-on-prefix prove queries, or `prove = false` for the \
                 no-proof variant",
            ));
        }

        if return_distinct_counts_in_range && !has_range {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "return_distinct_counts_in_range requires a range where-clause",
            ));
        }

        Ok(
            match (has_range, has_in, prove, return_distinct_counts_in_range) {
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
                // materialize-and-count proof path. The SDK's
                // `FromProof<DocumentCountQuery>` for
                // `DocumentSplitCounts` then groups verified
                // documents by the `In` field's serialized value to
                // produce per-key count entries. There's no
                // aggregate-proof primitive that emits one
                // `(key, count)` per In value yet, but the
                // materialize path is correct, just bounded at
                // u16::MAX.
                (false, true, true, _) => DocumentCountMode::PointLookupProof,
                (false, false, true, _) => DocumentCountMode::PointLookupProof,
                (false, false, false, _) => DocumentCountMode::Total,
                // (true, true, true, false) — range + In on the
                // aggregate prove path — is rejected by the
                // explicit early check above.
                (true, true, true, false) => unreachable!(
                    "range + In + prove + !distinct is rejected before the dispatch match"
                ),
            },
        )
    }
}
