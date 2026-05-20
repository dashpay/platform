//! v0 of [`super::detect_sum_mode`]. Routing table extracted
//! verbatim from the previous inline implementation in
//! `drive_document_sum_query::drive_dispatcher` so the v1
//! cutover is a pure code move with no semantic change.

use crate::error::Error;
use crate::query::drive_document_sum_query::{is_range_operator, DocumentSumMode, SumMode};
use crate::query::{WhereClause, WhereOperator};

/// Pure routing-table function: `(where_clauses, mode, prove)` →
/// resolved [`DocumentSumMode`]. The single source of truth — both
/// the server's `detect_sum_mode_v0` (above) and the SDK's
/// `detect_sum_mode_from_inputs` route through here so they can
/// never disagree on which executor handles a given shape.
///
/// Does NOT validate `sum_property` against the doctype — that's
/// the server-side wrapper's responsibility (the SDK enforces the
/// invariant via the index picker at picking time).
pub(super) fn detect_sum_mode_v0_from_inputs(
    where_clauses: &[WhereClause],
    mode: SumMode,
    prove: bool,
) -> Result<DocumentSumMode, Error> {
    let has_range = where_clauses
        .iter()
        .any(|wc| is_range_operator(wc.operator));
    let has_in = where_clauses
        .iter()
        .any(|wc| wc.operator == WhereOperator::In);

    Ok(match (mode, has_range, has_in, prove) {
        // No range / no In / no-proof — Total fast path. Covers both
        // empty-where (documents_summable) and Equal-only-fully-
        // covered (summable index lookup) — the executor branches on
        // where_clauses internally. Mirrors count's mapping.
        (SumMode::Aggregate, false, false, false) => DocumentSumMode::Total,
        // No range / has In / no-proof — per-In fan-out.
        (SumMode::Aggregate, false, true, false) => DocumentSumMode::PerInValue,
        (SumMode::Aggregate, true, _, false) => DocumentSumMode::RangeNoProof,
        (SumMode::Aggregate, true, false, true) => DocumentSumMode::RangeProof,
        // Aggregate + no-range + prove: routes to PointLookupProof for
        // both empty-where (documents_summable fast path) AND Equal/In
        // covered cases. The executor branches on `where_clauses`
        // internally.
        (SumMode::Aggregate, false, _, true) => DocumentSumMode::PointLookupProof,
        // GroupByIn: no range — falls back to PerInValue (no-proof)
        // or PointLookupProof (prove). Mirrors count's mapping.
        (SumMode::GroupByIn, false, _, false) => DocumentSumMode::PerInValue,
        (SumMode::GroupByIn, false, _, true) => DocumentSumMode::PointLookupProof,
        // GroupByIn + range: the carrier-ACOR shape (count's
        // RangeAggregateCarrierProof). Same routing on the sum side —
        // one sum per In branch.
        (SumMode::GroupByIn, true, true, true) => DocumentSumMode::RangeAggregateCarrierProof,
        (SumMode::GroupByIn, true, _, false) => DocumentSumMode::RangeNoProof,
        (SumMode::GroupByRange, true, _, true) => DocumentSumMode::RangeDistinctProof,
        (SumMode::GroupByRange, true, _, false) => DocumentSumMode::RangeNoProof,
        // GroupByCompound is a DISTINCT shape (per `(in_key,
        // range_key)` pair) — must route to `RangeDistinctProof` so
        // the prover walks every distinct in-range terminator per
        // In branch via `distinct_sum_path_query` (whose compound
        // arm handles `In on prefix + range on terminator`). Routing
        // this to `RangeAggregateCarrierProof` would have emitted
        // one entry PER In branch (collapsing the range axis), which
        // contradicts the no-proof semantics that emit one entry
        // per `(in_key, range_key)` pair. Mirrors count's
        // `(true, _, true, true) => RangeDistinctProof` for the
        // distinct branch (count's `GroupByCompound` is in
        // `requires_distinct_walk()` for the same reason).
        (SumMode::GroupByCompound, true, true, true) => DocumentSumMode::RangeDistinctProof,
        (SumMode::GroupByCompound, true, true, false) => DocumentSumMode::RangeNoProof,
        _ => {
            return Err(Error::Drive(crate::error::drive::DriveError::NotSupported(
                "sum-query dispatcher: where-shape × mode × prove combination is not \
                 supported; see book/src/drive/document-sum-trees.md's `Choosing What \
                 to Set` table for valid shapes.",
            )));
        }
    })
}

#[cfg(test)]
mod tests {
    //! Regression tests for the v0 routing table.
    //!
    //! The pure `(where_clauses, mode, prove)` shape lets us pin
    //! each row of the table from outside the dispatcher — without
    //! these tests, future router changes could silently re-route
    //! a shape (e.g., `GroupByIn + range + prove` from
    //! `RangeAggregateCarrierProof` to `RangeDistinctProof`) and
    //! break SDK + server in lock-step without tripping any other
    //! test. The SDK's `verify_sum_query` / `verify_average_query`
    //! both branch on `DocumentSumMode`, so a routing-table
    //! mistake would propagate to "verification fails" rather
    //! than "wrong answer", but the failure mode would still be
    //! a regression worth catching at routing-table level.
    use super::*;
    use crate::query::WhereOperator::{Between, Equal, In, LessThan};
    use dpp::platform_value::Value;

    fn equal_clause(field: &str) -> WhereClause {
        WhereClause {
            field: field.to_string(),
            operator: Equal,
            value: Value::U64(0),
        }
    }
    fn in_clause(field: &str) -> WhereClause {
        WhereClause {
            field: field.to_string(),
            operator: In,
            value: Value::Array(vec![Value::U64(0)]),
        }
    }
    fn range_clause(field: &str) -> WhereClause {
        WhereClause {
            field: field.to_string(),
            operator: LessThan,
            value: Value::U64(0),
        }
    }
    fn between_clause(field: &str) -> WhereClause {
        WhereClause {
            field: field.to_string(),
            operator: Between,
            value: Value::Array(vec![Value::U64(0), Value::U64(10)]),
        }
    }

    /// Empty-where + Aggregate + prove → PointLookupProof.
    /// This is the empty-where SUM fast path the SDK pre-empts
    /// with `verify_primary_key_sum_tree_proof`. Pinning the
    /// routing decision separately from the special-casing keeps
    /// the SDK gate honest if the router ever stopped using
    /// PointLookupProof for this corner.
    #[test]
    fn aggregate_empty_where_prove_routes_to_point_lookup() {
        let mode = detect_sum_mode_v0_from_inputs(&[], SumMode::Aggregate, true)
            .expect("empty-where aggregate prove must route");
        assert_eq!(mode, DocumentSumMode::PointLookupProof);
    }

    /// Equal-only + Aggregate + prove → PointLookupProof.
    /// Point-lookup is the prove-path target for Equal-only
    /// covering a `summable` index.
    #[test]
    fn aggregate_equal_only_prove_routes_to_point_lookup() {
        let mode =
            detect_sum_mode_v0_from_inputs(&[equal_clause("user_id")], SumMode::Aggregate, true)
                .expect("equal-only aggregate prove must route");
        assert_eq!(mode, DocumentSumMode::PointLookupProof);
    }

    /// In + Aggregate + prove → PointLookupProof (point-lookup
    /// dispatches over multiple In values via subquery; no
    /// dedicated carrier shape without range).
    #[test]
    fn aggregate_in_only_prove_routes_to_point_lookup() {
        let mode =
            detect_sum_mode_v0_from_inputs(&[in_clause("user_id")], SumMode::Aggregate, true)
                .expect("In-only aggregate prove must route");
        assert_eq!(mode, DocumentSumMode::PointLookupProof);
    }

    /// Range + Aggregate + prove → RangeProof
    /// (`AggregateSumOnRange` collapse → single i64).
    #[test]
    fn aggregate_range_prove_routes_to_range_proof() {
        let mode =
            detect_sum_mode_v0_from_inputs(&[between_clause("amount")], SumMode::Aggregate, true)
                .expect("range aggregate prove must route");
        assert_eq!(mode, DocumentSumMode::RangeProof);
    }

    /// Range + In + Aggregate + prove → UNSUPPORTED.
    /// `Aggregate` over `In + range` returns a single sum that
    /// requires SDK-side addition; the prove path rejects this
    /// because it'd defeat consensus-committed aggregation. The
    /// dispatcher returns NotSupported. Mirrors count's
    /// `validate_and_route` "use group_by = [in_field] for the
    /// carrier-ACOR" rejection.
    #[test]
    fn aggregate_range_with_in_prove_rejected() {
        let err = detect_sum_mode_v0_from_inputs(
            &[in_clause("user_id"), between_clause("amount")],
            SumMode::Aggregate,
            true,
        )
        .expect_err("Aggregate range+In prove must reject — use GroupByIn carrier shape");
        assert!(format!("{err:?}").contains("not supported"));
    }

    /// GroupByIn + range + prove → RangeAggregateCarrierProof.
    /// Carrier-ACOR shape — one (in_key, sum) per In branch via
    /// grovedb's subquery composition. Mode reconstruction here is
    /// the load-bearing one: without `detect_sum_mode_from_inputs`
    /// the SDK heuristic could mis-route this to an aggregate
    /// shape because the request also has `has_range && has_in`.
    #[test]
    fn group_by_in_with_range_prove_routes_to_carrier() {
        let mode = detect_sum_mode_v0_from_inputs(
            &[in_clause("user_id"), range_clause("amount")],
            SumMode::GroupByIn,
            true,
        )
        .expect("GroupByIn+range prove must route");
        assert_eq!(mode, DocumentSumMode::RangeAggregateCarrierProof);
    }

    /// GroupByRange + range + prove → RangeDistinctProof.
    /// Per-distinct-value SUM walk via grovedb's
    /// `distinct_sum_path_query`.
    #[test]
    fn group_by_range_prove_routes_to_range_distinct() {
        let mode =
            detect_sum_mode_v0_from_inputs(&[range_clause("amount")], SumMode::GroupByRange, true)
                .expect("GroupByRange prove must route");
        assert_eq!(mode, DocumentSumMode::RangeDistinctProof);
    }

    /// GroupByCompound + In + range + prove → RangeDistinctProof.
    /// Compound IS a distinct shape — one entry per `(in_key,
    /// range_key)` pair, walked via `distinct_sum_path_query`'s
    /// compound arm (which handles `In on prefix + range on
    /// terminator` via grovedb's subquery primitive). Pinning this
    /// to `RangeAggregateCarrierProof` would emit one entry PER In
    /// branch (collapsing the range axis), contradicting the
    /// no-proof semantics — which is the bug this test guards.
    /// Mirrors count's `GroupByCompound -> RangeDistinctProof`.
    #[test]
    fn group_by_compound_with_in_and_range_prove_routes_to_range_distinct() {
        let mode = detect_sum_mode_v0_from_inputs(
            &[in_clause("user_id"), range_clause("amount")],
            SumMode::GroupByCompound,
            true,
        )
        .expect("GroupByCompound In+range prove must route");
        assert_eq!(
            mode,
            DocumentSumMode::RangeDistinctProof,
            "GroupByCompound is a distinct shape — per (in_key, range_key) entries — \
             not a carrier-aggregate one. The carrier shape (one entry per In branch) \
             is reserved for GroupByIn."
        );
    }

    /// GroupByCompound + range (no In) + prove → UNSUPPORTED.
    /// Compound needs both axes; missing the In half is rejected.
    /// Pins that the table doesn't silently fall back to
    /// `RangeDistinctProof` for this shape.
    #[test]
    fn group_by_compound_without_in_prove_rejected() {
        let err = detect_sum_mode_v0_from_inputs(
            &[range_clause("amount")],
            SumMode::GroupByCompound,
            true,
        )
        .expect_err("GroupByCompound without In must reject");
        assert!(format!("{err:?}").contains("not supported"));
    }

    /// No-proof Aggregate Total fast-path: empty-where + no flags.
    #[test]
    fn aggregate_empty_where_noproof_routes_to_total() {
        let mode = detect_sum_mode_v0_from_inputs(&[], SumMode::Aggregate, false)
            .expect("empty-where aggregate no-proof must route");
        assert_eq!(mode, DocumentSumMode::Total);
    }

    /// No-proof Aggregate + In → PerInValue fan-out.
    #[test]
    fn aggregate_in_only_noproof_routes_to_per_in() {
        let mode =
            detect_sum_mode_v0_from_inputs(&[in_clause("user_id")], SumMode::Aggregate, false)
                .expect("In-only aggregate no-proof must route");
        assert_eq!(mode, DocumentSumMode::PerInValue);
    }
}
