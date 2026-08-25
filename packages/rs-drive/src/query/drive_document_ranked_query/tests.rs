//! Tests for the ranked query surface.
//!
//! Three layers, in that order in this file:
//!
//! 1. **Grammar** — [`mode_detection`](super::mode_detection) and
//!    [`index_picker`](super::index_picker) are pure functions, so the
//!    accept/reject matrix is exercised without a Drive.
//! 2. **Behaviour** — the dispatcher run end to end against the
//!    `restaurants` fixture, on all three axes, no-proof and proved, with
//!    the proof round-tripped through
//!    [`DriveDocumentRankedQuery::verify_ranked_top_k_proof`] and checked
//!    against the live grovedb root hash.
//! 3. **Index picking under competition** — the same round trip against
//!    an inline contract whose single doctype carries two ranked indexes
//!    on different properties and axes plus a non-ranked compound
//!    sibling, so that *which* index is chosen becomes observable and the
//!    prover and the client have something to disagree about.
//!
//! The fixture (`tests/supporting_files/contract/restaurants/`) is shared
//! with the write-path e2e suite and is read-only here. Its three
//! doctypes give one doctype per ranking axis, because two indexes over
//! the same property set on one doctype is a `DuplicateIndexError`:
//!
//! | doctype      | index                     | axis  | aggregated property |
//! |--------------|---------------------------|-------|---------------------|
//! | `review`     | `byRestaurant`            | Avg   | `grade`             |
//! | `visit`      | `byRestaurantVisits`      | Count | — (`COUNT(*)`)      |
//! | `tip`        | `byRestaurantTips`        | Sum   | `amount`            |
//! | `adjustment` | `byRestaurantAdjustments` | Avg   | `delta` (signed)    |
//!
//! `adjustment` exists for the write-path suite's signed-average coverage
//! (`delta` admits negatives, `grade` does not) and is unused here.

use super::index_picker::{find_ranked_index_for_axis, resolve_ranked_query_for_mode};
use super::mode_detection::{detect_ranked_mode, detect_ranked_mode_v0};
use super::*;
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::having::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
};
use crate::query::projection::{SelectFunction, SelectProjection};
use crate::query::{OrderClause, WhereClause, WhereOperator};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::document_type::{Index, IndexCountability, IndexProperty};
use dpp::document::{Document, DocumentV0Setters};
use dpp::platform_value::Value;
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use grovedb::element::indexed::compute_avg_fixed_point;
use std::collections::BTreeMap;

/// The one property every fixture doctype groups by.
const GROUP_PROPERTY: &str = "restaurantId";

fn platform_version() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

// ===================================================================
// Grammar: mode detection
// ===================================================================

fn group_by() -> Vec<String> {
    vec![GROUP_PROPERTY.to_string()]
}

/// The single `ORDER BY <field> [ASC|DESC]` clause that carries the
/// ranking. `ascending = false` is the `DESC` / "highest first"
/// reading.
fn order_by(field: &str, ascending: bool) -> Vec<OrderClause> {
    vec![OrderClause {
        field: field.to_string(),
        ascending,
    }]
}

/// `LIMIT limit [OFFSET offset]`, with no cursor.
fn page(limit: Option<u32>, offset: Option<u32>) -> RankedPaginationInputs {
    RankedPaginationInputs {
        limit,
        offset,
        has_start_at: false,
    }
}

/// `SELECT AVG(grade) … GROUP BY restaurantId ORDER BY grade …` — the
/// fixture's headline shape, parameterized on direction and pagination
/// so the mapping tests can sweep them.
fn detect_avg(
    ascending: bool,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<DocumentRankedMode, Error> {
    detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &[],
        &order_by("grade", ascending),
        &[],
        page(limit, offset),
    )
}

/// The `ORDER BY` direction is the walk direction and the `LIMIT` is
/// `k`, exactly as the SQL reading demands: `DESC` is "highest first",
/// `ASC` is "lowest first", and `LIMIT 1` is how the single
/// best-ranked group is asked for.
#[test]
fn order_direction_maps_to_walk_direction_and_limit_is_k() {
    let desc = detect_avg(false, Some(5), None).expect("ORDER BY … DESC LIMIT 5 is well-formed");
    assert!(desc.descending, "DESC ranks highest-first");
    assert_eq!(desc.k, 5);

    let asc = detect_avg(true, Some(3), None).expect("ORDER BY … ASC LIMIT 3 is well-formed");
    assert!(!asc.descending, "ASC ranks lowest-first");
    assert_eq!(asc.k, 3);

    let desc_one = detect_avg(false, Some(1), None).expect("LIMIT 1 is well-formed");
    assert!(desc_one.descending);
    assert_eq!(
        desc_one.k, 1,
        "DESC LIMIT 1 is the single best-ranked group"
    );

    let asc_one = detect_avg(true, Some(1), None).expect("LIMIT 1 is well-formed");
    assert!(!asc_one.descending);
    assert_eq!(asc_one.k, 1, "ASC LIMIT 1 is the single worst-ranked group");
}

/// `OFFSET` is optional and defaults to rank 0; when present it is
/// carried through verbatim, with no ceiling of its own. The ceiling is
/// deliberately absent because grovedb's paginated proof is
/// `O(log n + k)` at any offset — the skipped region is attested by
/// counted subtree commitments rather than walked — so a large offset
/// is not a cost lever.
#[test]
fn offset_is_optional_defaults_to_zero_and_is_uncapped() {
    assert_eq!(
        detect_avg(false, Some(3), None)
            .expect("no OFFSET is well-formed")
            .offset,
        0,
        "an absent OFFSET means the page starts at rank 0"
    );

    // "the 5th best grade" — skip the four above it, take one.
    let fifth_best = detect_avg(false, Some(1), Some(4)).expect("LIMIT 1 OFFSET 4 is well-formed");
    assert_eq!(fifth_best.k, 1);
    assert_eq!(fifth_best.offset, 4);
    assert!(fifth_best.descending);

    // Far past any plausible population, and far past MAX_RANKED_LIMIT:
    // still accepted, because the skip is counted from subtree
    // aggregates — work bounded by tree depth, not by the offset.
    let deep = detect_avg(false, Some(10), Some(u32::MAX)).expect("a huge OFFSET is well-formed");
    assert_eq!(deep.offset, u32::MAX);
    assert_eq!(deep.k, 10);
}

/// The resolved mode carries everything the index picker needs, not just
/// the ranking: which axis, which property to group on, and which field
/// the aggregate applies to.
#[test]
fn resolved_mode_carries_axis_group_property_and_field() {
    let avg = detect_avg(false, Some(2), None).expect("well-formed");
    assert_eq!(avg.axis, RankedAxis::Avg);
    assert_eq!(avg.group_by_property, GROUP_PROPERTY);
    assert_eq!(avg.aggregate_field, "grade");

    let count = detect_ranked_mode_v0(
        &SelectProjection::count_star(),
        &group_by(),
        &[],
        &order_by(RANKED_COUNT_ORDER_KEY, false),
        &[],
        page(Some(2), None),
    )
    .expect("COUNT(*) is well-formed");
    assert_eq!(count.axis, RankedAxis::Count);
    assert_eq!(
        count.aggregate_field, "",
        "COUNT(*) has no field — the count is of documents, not of values"
    );

    let sum = detect_ranked_mode_v0(
        &SelectProjection::sum("amount"),
        &group_by(),
        &[],
        &order_by("amount", true),
        &[],
        page(Some(1), None),
    )
    .expect("SUM is well-formed");
    assert_eq!(sum.axis, RankedAxis::Sum);
    assert_eq!(sum.aggregate_field, "amount");
}

/// `COUNT(*)` is ordered by the `$count` sentinel and by nothing else.
///
/// The sentinel exists because `COUNT(*)` has no field to name. `$` is
/// DPP's system-property namespace, which a document schema cannot use,
/// so the token is collision-proof against every contract that could
/// ever be written — a bare `"count"` would hijack ordering for any
/// schema that happens to have a `count` column, which is exactly the
/// failure this test guards.
#[test]
fn count_star_is_ordered_by_the_dollar_count_sentinel() {
    assert!(
        RANKED_COUNT_ORDER_KEY.starts_with('$'),
        "the sentinel must live in the system-property namespace so it cannot collide \
         with a schema property"
    );

    let mode = detect_ranked_mode_v0(
        &SelectProjection::count_star(),
        &group_by(),
        &[],
        &order_by(RANKED_COUNT_ORDER_KEY, false),
        &[],
        page(Some(4), None),
    )
    .expect("`ORDER BY $count DESC LIMIT 4` is the COUNT(*) ranking");
    assert_eq!(mode.axis, RankedAxis::Count);
    assert_eq!(mode.k, 4);

    // A plain `count` column name is *not* the sentinel, even though a
    // reader might expect it to be.
    let error = detect_ranked_mode_v0(
        &SelectProjection::count_star(),
        &group_by(),
        &[],
        &order_by("count", false),
        &[],
        page(Some(4), None),
    )
    .expect_err("only the `$`-prefixed sentinel names the COUNT(*) axis");
    match error {
        Error::Query(QuerySyntaxError::Unsupported(message)) => {
            assert!(
                message.contains(RANKED_COUNT_ORDER_KEY),
                "the rejection must name the sentinel to write instead, got: {message}"
            );
        }
        other => panic!("expected an Unsupported query error, got {other}"),
    }
}

/// `LIMIT 0` selects nothing and `LIMIT > MAX_RANKED_LIMIT` is refused
/// rather than clamped — a clamp would produce a proof whose echoed `k`
/// the client's own reconstruction rejects. The boundary itself is
/// accepted.
#[test]
fn k_is_bounded_to_one_through_max_ranked_limit() {
    let zero = detect_avg(false, Some(0), None).expect_err("LIMIT 0 must be rejected");
    assert!(matches!(
        zero,
        Error::Query(QuerySyntaxError::InvalidLimit(_))
    ));

    let over = detect_avg(false, Some(MAX_RANKED_LIMIT as u32 + 1), None)
        .expect_err("LIMIT 101 must be rejected");
    assert!(matches!(
        over,
        Error::Query(QuerySyntaxError::InvalidLimit(_))
    ));

    let at_limit = detect_avg(false, Some(MAX_RANKED_LIMIT as u32), None)
        .expect("LIMIT 100 sits exactly on the ceiling and must be accepted");
    assert_eq!(at_limit.k, MAX_RANKED_LIMIT);
}

/// `LIMIT` is mandatory in ranked mode. There is no server-side default
/// because `k` is echoed inside the proof envelope and re-checked by the
/// verifier: a number the client never chose is a number it cannot
/// reproduce when rebuilding the query to verify.
#[test]
fn limit_is_required() {
    let error = detect_avg(false, None, None).expect_err(
        "a ranked query without a limit is \
                                                          incomplete",
    );
    match error {
        Error::Query(QuerySyntaxError::InvalidLimit(message)) => {
            assert!(
                message.contains("limit"),
                "the rejection must name the missing knob, got: {message}"
            );
        }
        other => panic!("expected InvalidLimit, got {other}"),
    }

    // And an OFFSET alone does not stand in for it.
    assert!(matches!(
        detect_avg(false, None, Some(4)),
        Err(Error::Query(QuerySyntaxError::InvalidLimit(_)))
    ));
}

/// Ranked indexes are single-property, so grouping is single-property
/// too — neither a missing nor a compound `GROUP BY` has a ranking to
/// resolve to.
#[test]
fn group_by_must_name_exactly_one_property() {
    for group_by in [vec![], vec!["a".to_string(), "b".to_string()]] {
        let error = detect_ranked_mode_v0(
            &SelectProjection::avg("grade"),
            &group_by,
            &[],
            &order_by("grade", false),
            &[],
            page(Some(2), None),
        )
        .expect_err("group_by arity other than 1 must be rejected");
        assert!(matches!(
            error,
            Error::Query(QuerySyntaxError::InvalidParameter(_))
        ));
    }
}

/// The `ORDER BY` field must be the *selected aggregate's* field:
/// ranking one aggregate while projecting another, or ordering groups by
/// a raw document property, would need a sort the axis secondary does
/// not maintain.
#[test]
fn order_by_must_name_the_selected_aggregate() {
    // A different (perfectly real) document property.
    let wrong_field = detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &[],
        &order_by(GROUP_PROPERTY, false),
        &[],
        page(Some(2), None),
    )
    .expect_err("ordering groups by the grouping property is not a ranking");
    match wrong_field {
        Error::Query(QuerySyntaxError::Unsupported(message)) => {
            assert!(
                message.contains("grade"),
                "the rejection must name the ordering that *is* supported, got: {message}"
            );
        }
        other => panic!("expected an Unsupported query error, got {other}"),
    }

    // Another numeric property the select does not aggregate.
    assert!(matches!(
        detect_ranked_mode_v0(
            &SelectProjection::avg("grade"),
            &group_by(),
            &[],
            &order_by("price", false),
            &[],
            page(Some(2), None),
        ),
        Err(Error::Query(QuerySyntaxError::Unsupported(_)))
    ));

    // The COUNT(*) sentinel is not a stand-in for an AVG ordering.
    assert!(matches!(
        detect_ranked_mode_v0(
            &SelectProjection::avg("grade"),
            &group_by(),
            &[],
            &order_by(RANKED_COUNT_ORDER_KEY, false),
            &[],
            page(Some(2), None),
        ),
        Err(Error::Query(QuerySyntaxError::Unsupported(_)))
    ));
}

/// Exactly one `ORDER BY` clause. The axis secondary is a single-key
/// ordering — a second sort key would need a second axis the storage
/// does not maintain — and zero clauses is not a ranking at all.
#[test]
fn exactly_one_order_by_clause_is_required() {
    let mut two = order_by("grade", false);
    two.extend(order_by(GROUP_PROPERTY, true));
    for clauses in [Vec::new(), two] {
        let error = detect_ranked_mode_v0(
            &SelectProjection::avg("grade"),
            &group_by(),
            &[],
            &clauses,
            &[],
            page(Some(2), None),
        )
        .expect_err("order_by must carry exactly one clause");
        assert!(matches!(
            error,
            Error::Query(QuerySyntaxError::InvalidParameter(_))
        ));
    }
}

/// A boolean `HAVING` cannot yet be combined with an aggregate
/// ordering: ranking reads a pre-sorted secondary, and filtering groups
/// first would mean walking every group to test the predicate — exactly
/// the cost the ranked surface exists to avoid.
#[test]
fn having_is_rejected_alongside_an_aggregate_ordering() {
    let clauses = vec![HavingClause {
        aggregate: HavingAggregate {
            function: HavingAggregateFunction::Avg,
            field: "grade".to_string(),
        },
        operator: HavingOperator::GreaterThan,
        right: HavingRightOperand::Value(Value::U64(80)),
    }];
    let error = detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &clauses,
        &order_by("grade", false),
        &[],
        page(Some(2), None),
    )
    .expect_err("boolean having is not implemented on the ranked path");
    match error {
        Error::Query(QuerySyntaxError::Unsupported(message)) => {
            assert!(
                message.contains("having"),
                "the rejection must name the clause it is refusing, got: {message}"
            );
        }
        other => panic!("expected an Unsupported query error, got {other}"),
    }
}

/// Only the three maintained axes can be ranked, and `COUNT` must be
/// `COUNT(*)`: a ranked count axis counts documents per group.
#[test]
fn only_count_star_sum_and_avg_selects_are_rankable() {
    for select in [
        SelectProjection::documents(),
        SelectProjection::min("grade"),
        SelectProjection::max("grade"),
        SelectProjection::count_field("grade"),
    ] {
        let function = select.function;
        let error = detect_ranked_mode_v0(
            &select,
            &group_by(),
            &[],
            &order_by("grade", false),
            &[],
            page(Some(2), None),
        )
        .expect_err("only COUNT(*) / SUM(f) / AVG(f) rank");
        assert!(
            matches!(error, Error::Query(QuerySyntaxError::Unsupported(_))),
            "{function:?} must be rejected as unsupported, got {error}"
        );
    }
}

/// `SUM` / `AVG` need a field to aggregate; the index's `summable`
/// declaration is what they are matched against.
#[test]
fn sum_and_avg_selects_require_a_field() {
    for function in [SelectFunction::Sum, SelectFunction::Avg] {
        let select = SelectProjection {
            function,
            field: String::new(),
        };
        let error = detect_ranked_mode_v0(
            &select,
            &group_by(),
            &[],
            &order_by("amount", false),
            &[],
            page(Some(2), None),
        )
        .expect_err("a fieldless SUM/AVG has nothing to aggregate");
        assert!(matches!(
            error,
            Error::Query(QuerySyntaxError::InvalidParameter(_))
        ));
    }
}

/// `where` clauses are equality pins on a compound index's leading
/// properties. Detection is shape-only: an equality clause becomes a
/// pin (whether a compound index actually covers it is the resolver's
/// call — see `pins_without_a_covering_compound_index_are_rejected`);
/// anything that is not a distinct-property equality — or the one
/// permitted `IN`, which resolves to a multi-value branching pin — is
/// refused loudly.
#[test]
fn where_clauses_resolve_to_equality_pins_and_reject_everything_else() {
    // Equality pin: accepted at detection, carried in the mode.
    let pinned = vec![WhereClause {
        field: "chefId".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text("alpha".to_string()),
    }];
    let mode = detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &[],
        &order_by("grade", false),
        &pinned,
        page(Some(2), None),
    )
    .expect("an equality pin is a well-formed prefix pin");
    assert_eq!(
        mode.prefix_pins,
        vec![PrefixPin {
            field: "chefId".to_string(),
            values: vec![Value::Text("alpha".to_string())],
        }]
    );

    // A range operator can never pin a single prefix value tree.
    let ranged = vec![WhereClause {
        field: "chefId".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("alpha".to_string()),
    }];
    let error = detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &[],
        &order_by("grade", false),
        &ranged,
        page(Some(2), None),
    )
    .expect_err("a range operator cannot pin a prefix");
    assert!(matches!(
        error,
        Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(_))
    ));

    // `IN` resolves to a branching pin — one value per element — and a
    // single-element `IN` is exactly an equality pin.
    let in_clause = vec![WhereClause {
        field: "chefId".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![
            Value::Text("beta".to_string()),
            Value::Text("alpha".to_string()),
        ]),
    }];
    let mode = detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &[],
        &order_by("grade", false),
        &in_clause,
        page(Some(2), None),
    )
    .expect("an IN pin is a well-formed branching pin");
    assert_eq!(
        mode.prefix_pins,
        vec![PrefixPin {
            field: "chefId".to_string(),
            values: vec![
                Value::Text("beta".to_string()),
                Value::Text("alpha".to_string()),
            ],
        }],
        "the pin carries the elements verbatim; canonical branch order \
         is the encoder's job, not the grammar's"
    );

    // The same property pinned twice is a caller error, not a silent
    // last-write-wins.
    let duplicated = vec![
        WhereClause {
            field: "chefId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("alpha".to_string()),
        },
        WhereClause {
            field: "chefId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("beta".to_string()),
        },
    ];
    let error = detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &[],
        &order_by("grade", false),
        &duplicated,
        page(Some(2), None),
    )
    .expect_err("duplicate pins on one property must fail");
    assert!(matches!(
        error,
        Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(_))
    ));
}

/// `start_at` / `start_after` name a document id, and document ids do
/// not appear in a keyspace sorted by aggregate — so the cursor is
/// refused while `OFFSET`, which *is* meaningful there, is accepted.
#[test]
fn start_at_is_rejected_while_offset_is_accepted() {
    let error = detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &[],
        &order_by("grade", false),
        &[],
        RankedPaginationInputs {
            limit: Some(2),
            offset: None,
            has_start_at: true,
        },
    )
    .expect_err("ranked queries take no cursor");
    match error {
        Error::Query(QuerySyntaxError::InvalidLimit(message)) => {
            assert!(
                message.contains("OFFSET"),
                "the rejection must point at the pagination that does work, got: {message}"
            );
        }
        other => panic!("expected InvalidLimit, got {other}"),
    }

    // The same request paginated by offset instead is accepted.
    assert_eq!(
        detect_avg(false, Some(2), Some(6))
            .expect("OFFSET is the ranked pagination")
            .offset,
        6
    );
}

/// The versioned wrapper routes v0 to the v0 table and fails closed on an
/// unknown slot value rather than silently falling back.
#[test]
fn versioned_detection_routes_v0_and_rejects_unknown_versions() {
    let versioned = detect_ranked_mode(
        &SelectProjection::avg("grade"),
        &group_by(),
        &[],
        &order_by("grade", false),
        &[],
        page(Some(4), None),
        platform_version(),
    )
    .expect("PV14's detect_ranked_mode slot is 0");
    assert_eq!(versioned, detect_avg(false, Some(4), None).unwrap());

    let mut future = platform_version().clone();
    future.drive.methods.document.query.detect_ranked_mode = 1;
    let error = detect_ranked_mode(
        &SelectProjection::avg("grade"),
        &group_by(),
        &[],
        &order_by("grade", false),
        &[],
        page(Some(4), None),
        &future,
    )
    .expect_err("an unknown routing-table version must fail closed");
    assert!(matches!(
        error,
        Error::Query(QuerySyntaxError::Unsupported(_))
    ));
}

// ===================================================================
// Grammar: index picking
// ===================================================================

fn test_index(name: &str, properties: &[&str], summable: Option<&str>) -> Index {
    Index {
        name: name.to_string(),
        properties: properties
            .iter()
            .map(|property| IndexProperty {
                name: property.to_string(),
                ascending: true,
            })
            .collect(),
        unique: false,
        null_searchable: true,
        contested_index: None,
        countable: IndexCountability::Countable,
        range_countable: true,
        summable: summable.map(String::from),
        range_summable: summable.is_some(),
        ranked_countable: false,
        ranked_summable: false,
        ranked_averageable: false,
    }
}

fn index_map(indexes: Vec<Index>) -> BTreeMap<String, Index> {
    indexes
        .into_iter()
        .map(|index| (index.name.clone(), index))
        .collect()
}

/// The axis must be declared by the index, and it is the `ranked_*` flags
/// that decide — not the stored element variant. A `rankedCountable`
/// index that also declares `rangeSummable` is laid down as a PCPSIT
/// (which *can* host a sum secondary) yet carries only the Count axis, so
/// reading rankability off the element would over-report it.
#[test]
fn picker_requires_the_index_to_declare_the_requested_axis() {
    let mut index = test_index("byRestaurant", &[GROUP_PROPERTY], Some("grade"));
    index.ranked_countable = true;
    let indexes = index_map(vec![index]);

    assert!(
        find_ranked_index_for_axis(&indexes, GROUP_PROPERTY, &[], RankedAxis::Count, "").is_some(),
        "the declared axis resolves"
    );
    for (axis, field) in [(RankedAxis::Sum, "grade"), (RankedAxis::Avg, "grade")] {
        assert!(
            find_ranked_index_for_axis(&indexes, GROUP_PROPERTY, &[], axis, field).is_none(),
            "{axis:?} is not declared even though the index is summable and the stored \
             element could host that secondary"
        );
    }
}

/// `SUM` / `AVG` rank the sum the index accumulates. Asking for a
/// different field must not resolve — it would answer about the wrong
/// property with no indication anything was substituted.
#[test]
fn picker_requires_the_select_field_to_be_the_indexed_summable() {
    let mut index = test_index("byRestaurant", &[GROUP_PROPERTY], Some("grade"));
    index.ranked_summable = true;
    index.ranked_averageable = true;
    let indexes = index_map(vec![index]);

    for axis in [RankedAxis::Sum, RankedAxis::Avg] {
        assert!(
            find_ranked_index_for_axis(&indexes, GROUP_PROPERTY, &[], axis, "grade").is_some(),
            "{axis:?} on the indexed summable resolves"
        );
        assert!(
            find_ranked_index_for_axis(&indexes, GROUP_PROPERTY, &[], axis, "tipAmount").is_none(),
            "{axis:?} on a different field must not resolve"
        );
    }
}

/// A property no ranked index groups on — whether it exists on the
/// doctype or not — has no ranking to serve.
#[test]
fn picker_rejects_an_unknown_group_property() {
    let mut index = test_index("byRestaurant", &[GROUP_PROPERTY], Some("grade"));
    index.ranked_averageable = true;
    let indexes = index_map(vec![index]);

    assert!(
        find_ranked_index_for_axis(&indexes, "chefId", &[], RankedAxis::Avg, "grade").is_none(),
        "no index groups by `chefId`"
    );
}

/// Compound indexes cannot serve a ranked query even if they carry the
/// flags: their terminal property-name tree sits under a prefix value
/// tree, and naming that prefix would need a `where` clause the surface
/// does not accept. rs-dpp rejects such contracts at parse time; the
/// picker is the query-side backstop.
#[test]
fn picker_rejects_compound_indexes() {
    let mut index = test_index(
        "byRestaurantCourse",
        &[GROUP_PROPERTY, "course"],
        Some("price"),
    );
    index.ranked_averageable = true;
    let indexes = index_map(vec![index]);

    assert!(
        find_ranked_index_for_axis(&indexes, GROUP_PROPERTY, &[], RankedAxis::Avg, "price")
            .is_none()
    );
}

// ===================================================================
// Behaviour: end to end against the restaurants fixture
// ===================================================================

/// Load and apply the restaurants fixture.
fn setup_restaurants() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/restaurants/restaurants-contract.json",
        false,
        pv,
    )
    .expect("expected to parse the restaurants contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the restaurants contract");
    (drive, contract)
}

/// Insert `(restaurant, value)` rows as documents of `document_type_name`
/// through the ordinary document-insert path, so the ranked secondaries
/// are maintained by the real write path.
///
/// `first_seed` seeds the random-document generator, which is what
/// derives each document's id. Two calls in the same test must pass
/// disjoint seed ranges or the second one collides on an existing id.
fn insert_docs(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    aggregated_property: &str,
    first_seed: u64,
    rows: &[(&str, i64)],
) {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(document_type_name)
        .unwrap_or_else(|_| panic!("{document_type_name} doctype exists"));
    for (i, (restaurant, value)) in rows.iter().enumerate() {
        let mut doc: Document = document_type
            .random_document(Some(first_seed + i as u64), pv)
            .expect("random document");
        let mut props = BTreeMap::new();
        props.insert(
            GROUP_PROPERTY.to_string(),
            Value::Text(restaurant.to_string()),
        );
        props.insert(aggregated_property.to_string(), Value::I64(*value));
        doc.set_properties(props);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .unwrap_or_else(|e| panic!("expected to insert a {document_type_name} document: {e}"));
    }
}

/// One ranked request, minus the `prove` flag and the pieces the
/// dispatcher derives.
#[derive(Clone)]
struct RankedCase {
    document_type_name: &'static str,
    /// The single `GROUP BY` property. Every fixture doctype groups by
    /// [`GROUP_PROPERTY`]; the multi-index contract below is the one that
    /// varies it, which is what makes the index picker's choice
    /// observable.
    group_by_property: &'static str,
    select: SelectProjection,
    /// The `ORDER BY` field. Must name the selected aggregate:
    /// [`RANKED_COUNT_ORDER_KEY`] for `COUNT(*)`, otherwise the select's
    /// own field.
    order_field: &'static str,
    /// `false` is `DESC` — the "highest first" ranking.
    ascending: bool,
    limit: Option<u32>,
    offset: Option<u32>,
}

impl RankedCase {
    fn avg(ascending: bool, limit: Option<u32>) -> Self {
        Self {
            document_type_name: "review",
            group_by_property: GROUP_PROPERTY,
            select: SelectProjection::avg("grade"),
            order_field: "grade",
            ascending,
            limit,
            offset: None,
        }
    }

    fn count(ascending: bool, limit: Option<u32>) -> Self {
        Self {
            document_type_name: "visit",
            group_by_property: GROUP_PROPERTY,
            select: SelectProjection::count_star(),
            order_field: RANKED_COUNT_ORDER_KEY,
            ascending,
            limit,
            offset: None,
        }
    }

    fn sum(ascending: bool, limit: Option<u32>) -> Self {
        Self {
            document_type_name: "tip",
            group_by_property: GROUP_PROPERTY,
            select: SelectProjection::sum("amount"),
            order_field: "amount",
            ascending,
            limit,
            offset: None,
        }
    }

    /// The same case, paginated: `… LIMIT limit OFFSET offset`.
    fn at_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    fn group_by(&self) -> Vec<String> {
        vec![self.group_by_property.to_string()]
    }

    fn order_by(&self) -> Vec<OrderClause> {
        vec![OrderClause {
            field: self.order_field.to_string(),
            ascending: self.ascending,
        }]
    }
}

/// Run a case through the public dispatcher entry point — the same call
/// drive-abci's routing layer will make.
fn run(
    drive: &Drive,
    contract: &DataContract,
    case: &RankedCase,
    prove: bool,
) -> Result<DocumentRankedResponse, Error> {
    let group_by = case.group_by();
    let order_by = case.order_by();
    let document_type = contract
        .document_type_for_name(case.document_type_name)
        .expect("doctype exists");
    drive.execute_document_ranked_request(
        DocumentRankedRequest {
            contract,
            document_type,
            group_by: &group_by,
            select: case.select.clone(),
            having: &[],
            order_by: &order_by,
            where_clauses: &[],
            limit: case.limit,
            offset: case.offset,
            has_start_at: false,
            prove,
        },
        None,
        platform_version(),
    )
}

fn page_of(response: DocumentRankedResponse) -> RankedPage {
    match response {
        DocumentRankedResponse::Entries(page) => page,
        DocumentRankedResponse::Proof(_) => panic!("expected entries, got a proof"),
    }
}

fn entries_of(response: DocumentRankedResponse) -> Vec<RankedEntry> {
    page_of(response).entries
}

fn proof_of(response: DocumentRankedResponse) -> Vec<u8> {
    match response {
        DocumentRankedResponse::Proof(proof) => proof,
        DocumentRankedResponse::Entries(_) => panic!("expected a proof, got entries"),
    }
}

fn keys_of(entries: &[RankedEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| String::from_utf8(entry.key.clone()).expect("fixture group keys are utf-8"))
        .collect()
}

/// Rebuild the query the way a client would: re-run the same versioned
/// validation, then resolve the index off the contract. This is the shape
/// the SDK's proof helper will take, so exercising it here pins that the
/// verifier can reach everything it needs from public API.
fn client_side_query<'a>(
    contract: &'a DataContract,
    case: &RankedCase,
) -> DriveDocumentRankedQuery<'a> {
    let group_by = case.group_by();
    let order_by = case.order_by();
    let mode = detect_ranked_mode(
        &case.select,
        &group_by,
        &[],
        &order_by,
        &[],
        RankedPaginationInputs {
            limit: case.limit,
            offset: case.offset,
            has_start_at: false,
        },
        platform_version(),
    )
    .expect("the case is well-formed");
    // Taken off the contract's document-type map rather than off a local
    // `DocumentTypeRef`, so the borrow lives as long as `contract`.
    let indexes = contract
        .document_types()
        .get(case.document_type_name)
        .expect("doctype exists")
        .indexes();
    resolve_ranked_query_for_mode(
        contract.id_ref().to_buffer(),
        contract
            .document_type_for_name(case.document_type_name)
            .expect("doctype exists"),
        case.document_type_name.to_string(),
        indexes,
        &mode,
        platform_version(),
    )
    .expect("the fixture declares the axis")
}

fn grovedb_root_hash(drive: &Drive) -> [u8; 32] {
    drive
        .grove
        .root_hash(None, &platform_version().drive.grove_version)
        .unwrap()
        .expect("root hash must be readable")
}

/// Prove the case, verify the proof, and assert the verified page and
/// root hash match the live database. Returns the verified page so
/// callers can assert on the attested `skipped` rank.
fn assert_proof_round_trips(
    drive: &Drive,
    contract: &DataContract,
    case: &RankedCase,
    expected: &[RankedEntry],
) -> RankedPage {
    let proof = proof_of(run(drive, contract, case, true).expect("prove must succeed"));
    let query = client_side_query(contract, case);
    let (root_hash, verified) = query
        .verify_ranked_top_k_proof(&proof, platform_version())
        .expect("the proof must verify");
    assert_eq!(
        verified.entries, expected,
        "verified entries must equal what the unproven read returned"
    );
    assert_eq!(
        root_hash,
        grovedb_root_hash(drive),
        "the proof must reconstruct the live grovedb root hash"
    );
    verified
}

/// Averages: alpha (90+80)/2 = 85, beta (60+70+50)/3 = 60, gamma 95,
/// delta (40+20)/2 = 30. The fixed-point values are asserted explicitly
/// rather than compared to a re-derived helper's output, so a change to
/// the scale or the rounding shows up here.
#[test]
fn avg_axis_ranks_reads_and_proves_consistently() {
    let (drive, contract) = setup_restaurants();
    insert_docs(
        &drive,
        &contract,
        "review",
        "grade",
        1,
        &[
            ("alpha", 90),
            ("alpha", 80),
            ("beta", 60),
            ("beta", 70),
            ("beta", 50),
            ("gamma", 95),
            ("delta", 40),
            ("delta", 20),
        ],
    );

    let top_three = RankedCase::avg(false, Some(3));
    let entries = entries_of(run(&drive, &contract, &top_three, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&entries),
        vec!["gamma", "alpha", "beta"],
        "descending by average: gamma(95) > alpha(85) > beta(60) > delta(30)"
    );
    assert_eq!(
        entries[0].value,
        RankedEntryValue::AvgFixedPoint(95 * RANKED_AVG_SCALE)
    );
    assert_eq!(
        entries[1].value,
        RankedEntryValue::AvgFixedPoint(85 * RANKED_AVG_SCALE)
    );
    assert_eq!(
        entries[2].value,
        RankedEntryValue::AvgFixedPoint(60 * RANKED_AVG_SCALE)
    );
    assert_proof_round_trips(&drive, &contract, &top_three, &entries);

    // A non-integral average exercises the fixed-point floor: beta's
    // 180/3 is exact, so use a group whose sum doesn't divide evenly.
    insert_docs(
        &drive,
        &contract,
        "review",
        "grade",
        500,
        &[("epsilon", 10), ("epsilon", 11)],
    );
    let bottom_one = RankedCase::avg(true, Some(1));
    let entries =
        entries_of(run(&drive, &contract, &bottom_one, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&entries),
        vec!["epsilon"],
        "21/2 = 10.5 is the lowest average"
    );
    assert_eq!(
        entries[0].value,
        RankedEntryValue::AvgFixedPoint(21 * RANKED_AVG_SCALE / 2),
        "the Avg axis sorts by floor(sum * RANKED_AVG_SCALE / count)"
    );
    assert_eq!(
        entries[0].value,
        RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(21, 2)),
        "and that is exactly grovedb's own fixed-point computation"
    );
    assert_eq!(
        entries[0].value.as_f64(),
        10.5,
        "as_f64 divides the fixed point back down by the scale"
    );
    assert_proof_round_trips(&drive, &contract, &bottom_one, &entries);
}

/// The Count axis ranks by group size; `COUNT(*)` needs no field.
#[test]
fn count_axis_ranks_reads_and_proves_consistently() {
    let (drive, contract) = setup_restaurants();
    insert_docs(
        &drive,
        &contract,
        "visit",
        "guests",
        1,
        &[
            ("alpha", 2),
            ("beta", 4),
            ("beta", 2),
            ("beta", 6),
            ("gamma", 3),
            ("gamma", 1),
            ("delta", 5),
            ("delta", 5),
            ("delta", 5),
            ("delta", 5),
        ],
    );

    let top_two = RankedCase::count(false, Some(2));
    let entries = entries_of(run(&drive, &contract, &top_two, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&entries),
        vec!["delta", "beta"],
        "descending by document count: delta(4) > beta(3) > gamma(2) > alpha(1)"
    );
    assert_eq!(entries[0].value, RankedEntryValue::Count(4));
    assert_eq!(entries[1].value, RankedEntryValue::Count(3));
    assert_proof_round_trips(&drive, &contract, &top_two, &entries);

    let bottom_one = RankedCase::count(true, Some(1));
    let entries =
        entries_of(run(&drive, &contract, &bottom_one, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&entries),
        vec!["alpha"],
        "ASC LIMIT 1 is the single smallest group"
    );
    assert_eq!(entries[0].value, RankedEntryValue::Count(1));
    assert_proof_round_trips(&drive, &contract, &bottom_one, &entries);

    // A missing LIMIT is refused end to end, not just in the pure
    // detector: `k` is echoed in the proof envelope, so there is no
    // server-side default a verifying client could reproduce.
    let mut no_limit = RankedCase::count(true, None);
    no_limit.limit = None;
    let error = run(&drive, &contract, &no_limit, false)
        .expect_err("a ranked request without a limit is incomplete");
    assert!(matches!(
        error,
        Error::Query(QuerySyntaxError::InvalidLimit(_))
    ));
}

/// The Sum axis ranks by the running sum of the index's summable property.
#[test]
fn sum_axis_ranks_reads_and_proves_consistently() {
    let (drive, contract) = setup_restaurants();
    insert_docs(
        &drive,
        &contract,
        "tip",
        "amount",
        1,
        &[
            ("alpha", 10),
            ("alpha", 15),
            ("beta", 100),
            ("gamma", 7),
            ("gamma", 8),
            ("gamma", 9),
            ("delta", 1),
        ],
    );

    let top_one = RankedCase::sum(false, Some(1));
    let entries = entries_of(run(&drive, &contract, &top_one, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&entries),
        vec!["beta"],
        "DESC LIMIT 1 is the single largest sum"
    );
    assert_eq!(entries[0].value, RankedEntryValue::Sum(100));
    assert_proof_round_trips(&drive, &contract, &top_one, &entries);

    let bottom_three = RankedCase::sum(true, Some(3));
    let entries =
        entries_of(run(&drive, &contract, &bottom_three, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&entries),
        vec!["delta", "gamma", "alpha"],
        "ascending by sum: delta(1) < gamma(24) < alpha(25) < beta(100)"
    );
    assert_eq!(
        entries.iter().map(|entry| entry.value).collect::<Vec<_>>(),
        vec![
            RankedEntryValue::Sum(1),
            RankedEntryValue::Sum(24),
            RankedEntryValue::Sum(25)
        ]
    );
    assert_proof_round_trips(&drive, &contract, &bottom_three, &entries);
}

/// Asking for more groups than exist returns them all — a short result is
/// the index having fewer groups, not an error, and the proof round-trips
/// just the same.
#[test]
fn top_k_larger_than_the_group_count_returns_every_group() {
    let (drive, contract) = setup_restaurants();
    insert_docs(
        &drive,
        &contract,
        "tip",
        "amount",
        1,
        &[("alpha", 10), ("beta", 20)],
    );

    let case = RankedCase::sum(false, Some(MAX_RANKED_LIMIT as u32));
    let entries = entries_of(run(&drive, &contract, &case, false).expect("read must succeed"));
    assert_eq!(keys_of(&entries), vec!["beta", "alpha"]);
    assert_proof_round_trips(&drive, &contract, &case, &entries);
}

/// **`OFFSET` pages through the ranking, and the proof attests where the
/// page starts.**
///
/// Five groups by average grade, descending:
/// `gamma(95) > alpha(85) > beta(60) > delta(30) > epsilon(10)`.
///
/// Three shapes are covered, because they fail differently:
///
/// 1. **A one-entry window in the middle** — `LIMIT 1 OFFSET 4` is how
///    "the 5th best grade" is spelled. The entry alone does not say
///    which rank it is; `skipped` does, and it comes back attested.
/// 2. **A window running off the end** — asking for three groups from
///    rank 3 yields the two that exist. A short page is the index
///    having fewer groups, not an error, exactly as a short unpaginated
///    page is.
/// 3. **A window entirely past the end** — the page is empty *and*
///    `skipped` collapses to the secondary's true population, which is
///    the counted walk's way of saying "there is nothing here, and here
///    is how much there is in total". Both paths report it: the counted
///    descent tracks how far the skip got, so an unproven read reports a
///    population rather than the offset it was asked for.
#[test]
fn offset_pages_through_the_ranking_and_the_proof_attests_the_starting_rank() {
    let (drive, contract) = setup_restaurants();
    insert_docs(
        &drive,
        &contract,
        "review",
        "grade",
        1,
        &[
            ("gamma", 95),
            ("alpha", 90),
            ("alpha", 80),
            ("beta", 60),
            ("delta", 30),
            ("epsilon", 10),
        ],
    );

    // (1) The 5th best grade: skip the four above it, take one.
    let fifth_best = RankedCase::avg(false, Some(1)).at_offset(4);
    let page = page_of(run(&drive, &contract, &fifth_best, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&page.entries),
        vec!["epsilon"],
        "gamma > alpha > beta > delta > epsilon — rank 4 (0-based) is epsilon"
    );
    assert_eq!(page.entries[0].value.as_f64(), 10.0);
    let verified = assert_proof_round_trips(&drive, &contract, &fifth_best, &page.entries);
    assert_eq!(
        verified.skipped, 4,
        "the proof must attest that this entry really is the 5th-ranked group, not just \
         that it is *a* group"
    );

    // (2) A window that runs off the end returns the tail, short.
    let tail = RankedCase::avg(false, Some(3)).at_offset(3);
    let page = page_of(run(&drive, &contract, &tail, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&page.entries),
        vec!["delta", "epsilon"],
        "only two groups remain from rank 3; a short page is not an error"
    );
    let verified = assert_proof_round_trips(&drive, &contract, &tail, &page.entries);
    assert_eq!(
        verified.skipped, 3,
        "the skip itself succeeded, so `skipped` is the requested offset"
    );

    // (3) A window entirely past the end: empty page, and `skipped`
    //     attests the population.
    let past_end = RankedCase::avg(false, Some(2)).at_offset(9);
    let page = page_of(run(&drive, &contract, &past_end, false).expect("read must succeed"));
    assert!(
        page.entries.is_empty(),
        "there is no rank 9 in a five-group ranking"
    );
    assert_eq!(
        page.skipped, 5,
        "the unproven read reports the population it actually reached, not the requested \
         offset of 9: grovedb's counted descent knows how far the walk got and returns it \
         on the page"
    );
    let verified = assert_proof_round_trips(&drive, &contract, &past_end, &page.entries);
    assert_eq!(
        verified.skipped, 5,
        "the *proved* path re-derives the real skip from the counted subtree \
         commitments, and `skipped < offset` with an empty page is a proof that the \
         ranking holds exactly five groups"
    );

    // Paging with `ASC` walks the same five groups from the other end,
    // so offset 4 there is the *best* group rather than the worst.
    let worst_first_fifth = RankedCase::avg(true, Some(1)).at_offset(4);
    let page =
        page_of(run(&drive, &contract, &worst_first_fifth, false).expect("read must succeed"));
    assert_eq!(keys_of(&page.entries), vec!["gamma"]);
    let verified = assert_proof_round_trips(&drive, &contract, &worst_first_fifth, &page.entries);
    assert_eq!(verified.skipped, 4);
}

/// A proof of one page must not verify as another page of the same
/// ranking. `offset` is echoed in the envelope and re-checked, which is
/// what stops a server from answering "the 5th best" with a proof of
/// "the best" — the entries would look perfectly valid, and only the
/// offset binding distinguishes them.
#[test]
fn a_proof_does_not_verify_under_a_different_offset() {
    let (drive, contract) = setup_restaurants();
    insert_docs(
        &drive,
        &contract,
        "review",
        "grade",
        1,
        &[("alpha", 90), ("beta", 60), ("gamma", 30), ("delta", 10)],
    );

    let at_two = RankedCase::avg(false, Some(1)).at_offset(2);
    let proof = proof_of(run(&drive, &contract, &at_two, true).expect("prove must succeed"));
    assert!(client_side_query(&contract, &at_two)
        .verify_ranked_top_k_proof(&proof, platform_version())
        .is_ok());

    for other_offset in [0u32, 1, 3] {
        let other = RankedCase::avg(false, Some(1)).at_offset(other_offset);
        assert!(
            client_side_query(&contract, &other)
                .verify_ranked_top_k_proof(&proof, platform_version())
                .is_err(),
            "a proof for OFFSET 2 must not verify as OFFSET {other_offset}"
        );
    }

    // And an unpaginated query is not the same query as `OFFSET 0`'s
    // sibling either — it *is* `OFFSET 0`, so that one must verify.
    let unpaginated_proof = proof_of(
        run(&drive, &contract, &RankedCase::avg(false, Some(1)), true).expect("prove must succeed"),
    );
    assert!(
        client_side_query(&contract, &RankedCase::avg(false, Some(1)).at_offset(0))
            .verify_ranked_top_k_proof(&unpaginated_proof, platform_version())
            .is_ok(),
        "an absent OFFSET and `OFFSET 0` are the same request, so one's proof verifies \
         under the other"
    );
}

/// **An empty ranking now proves, and that is a consequence of moving to
/// the paginated primitive.**
///
/// The unproven read returns an empty list — the indexed tree exists from
/// contract registration, its secondary is simply empty. Under the old
/// non-paginated prover the prove path *errored* here ("Cannot create
/// proof for empty tree"): there was no envelope shape for "this ranking
/// has no entries", so a freshly registered contract queried with
/// `prove = true` got an error until the first document landed.
///
/// `prove_indexed_axis_top_k_paginated` closes that gap — it emits a
/// guaranteed-empty range against the secondary rather than refusing —
/// so the two paths now agree on empty state, and this test is the
/// tripwire that says so. The attested `skipped` is `0`, which for an
/// `OFFSET 0` request is the proof that the ranking is empty rather than
/// merely unread.
#[test]
fn ranking_an_empty_index_reads_empty_and_proves_empty() {
    let (drive, contract) = setup_restaurants();
    let case = RankedCase::avg(false, Some(5));

    let entries = entries_of(run(&drive, &contract, &case, false).expect("read must succeed"));
    assert!(
        entries.is_empty(),
        "an index with no documents has no groups to rank"
    );

    let verified = assert_proof_round_trips(&drive, &contract, &case, &[]);
    assert_eq!(
        verified.skipped, 0,
        "nothing was skipped because there was nothing to skip"
    );

    // Asking past the end of an empty ranking is equally provable, and
    // `skipped` collapses to the (zero) population.
    let verified = assert_proof_round_trips(
        &drive,
        &contract,
        &RankedCase::avg(false, Some(5)).at_offset(3),
        &[],
    );
    assert_eq!(
        verified.skipped, 0,
        "the walk found nothing to skip, which attests a population of 0"
    );

    // And the same request keeps working once a document lands.
    insert_docs(&drive, &contract, "review", "grade", 1, &[("alpha", 42)]);
    let entries = entries_of(run(&drive, &contract, &case, false).expect("read must succeed"));
    assert_eq!(keys_of(&entries), vec!["alpha"]);
    assert_proof_round_trips(&drive, &contract, &case, &entries);
}

/// **Tie ordering, pinned against observed grovedb behaviour.**
///
/// The secondary's keys are `(sort_key ‖ group_key)` and the walk is a
/// plain directional scan of that keyspace, so equal aggregates come back
/// in group-key order *in the direction of the walk*: ascending group key
/// going up, descending group key going down. The descending case is the
/// counter-intuitive one — a caller expecting "ties always break
/// ascending by key" would read the list backwards — so both directions
/// are asserted here rather than only the natural one.
#[test]
fn ties_break_by_group_key_in_the_walk_direction() {
    let (drive, contract) = setup_restaurants();
    // Four groups, all summing to 50: only the group key distinguishes them.
    insert_docs(
        &drive,
        &contract,
        "tip",
        "amount",
        1,
        &[("alpha", 50), ("beta", 50), ("gamma", 50), ("delta", 50)],
    );

    let descending = entries_of(
        run(&drive, &contract, &RankedCase::sum(false, Some(4)), false).expect("read must succeed"),
    );
    assert_eq!(
        keys_of(&descending),
        vec!["gamma", "delta", "beta", "alpha"],
        "descending walk breaks ties in DESCENDING group-key order"
    );

    let ascending = entries_of(
        run(&drive, &contract, &RankedCase::sum(true, Some(4)), false).expect("read must succeed"),
    );
    assert_eq!(
        keys_of(&ascending),
        vec!["alpha", "beta", "delta", "gamma"],
        "ascending walk breaks ties in ascending group-key order"
    );
    assert_eq!(
        ascending.iter().rev().map(|e| &e.key).collect::<Vec<_>>(),
        descending.iter().map(|e| &e.key).collect::<Vec<_>>(),
        "the two directions are exact reverses of each other under a full-width k"
    );

    // A tie-truncating `k` therefore selects a *specific* subset, not an
    // arbitrary one — which is what makes a truncated LIMIT k reproducible.
    let top_two = entries_of(
        run(&drive, &contract, &RankedCase::sum(false, Some(2)), false).expect("read must succeed"),
    );
    assert_eq!(keys_of(&top_two), vec!["gamma", "delta"]);
    assert_proof_round_trips(
        &drive,
        &contract,
        &RankedCase::sum(false, Some(2)),
        &top_two,
    );
}

/// A tampered proof must never verify **to the honest root hash**.
///
/// That phrasing is the actual security property, and it is stronger than
/// "must return `Err`". Sweeping every byte of a real envelope shows why:
/// most flips do error out, but ~9% of them (the bytes of sibling-subtree
/// hashes inside the ancestor layer proofs) verify cleanly and return the
/// correct entries — with a *different* reconstructed root hash. Those
/// tampers are caught downstream, when the caller compares the returned
/// root against the tenderdash-signed app hash.
///
/// So: `verify_ranked_top_k_proof` returning `Ok` is not by itself
/// evidence of anything. The returned `RootHash` is load-bearing and
/// every consumer must bind it to consensus — see
/// `rs-drive-proof-verifier` for the composition that does. This test
/// exists to keep that from being forgotten, and to prove the envelope
/// leaves no byte that can be changed while *both* the entries and the
/// root hash survive.
#[test]
fn a_tampered_proof_never_verifies_to_the_honest_root_hash() {
    let (drive, contract) = setup_restaurants();
    insert_docs(
        &drive,
        &contract,
        "review",
        "grade",
        1,
        &[("alpha", 90), ("beta", 60), ("gamma", 30)],
    );

    let case = RankedCase::avg(false, Some(2));
    let proof = proof_of(run(&drive, &contract, &case, true).expect("prove must succeed"));
    let query = client_side_query(&contract, &case);
    let (_, honest) = query
        .verify_ranked_top_k_proof(&proof, platform_version())
        .expect("the untampered proof verifies");

    let honest_root = grovedb_root_hash(&drive);
    let mut verified_at_all = 0usize;
    for index in 0..proof.len() {
        for bit in 0..8 {
            let mut tampered = proof.clone();
            tampered[index] ^= 1 << bit;
            if let Ok((root, entries)) =
                query.verify_ranked_top_k_proof(&tampered, platform_version())
            {
                verified_at_all += 1;
                assert_ne!(
                    root, honest_root,
                    "flipping bit {bit} of byte {index} produced a proof that verified to \
                     the honest root hash with entries {entries:?} (honest result was \
                     {honest:?}) — the envelope must not have a byte that can be changed \
                     while the reconstructed root survives"
                );
            }
        }
    }
    // Sanity-check the test itself: if *every* mutation errored out, the
    // assertion above would be vacuous and a regression that made
    // verification accept-everything-with-a-wrong-root would slip past.
    assert!(
        verified_at_all > 0,
        "expected some mutations (sibling-hash bytes) to verify with a diverged root hash"
    );
}

/// The verifier must be checking the ranking it was asked about: a proof
/// generated for one `(axis, k, descending)` triple must not verify under
/// another. grovedb echoes all three in the envelope and re-checks them,
/// and this pins that drive passes each of them through faithfully — a
/// dropped argument here would let a client accept a proof of a
/// different question.
#[test]
fn a_proof_does_not_verify_under_a_different_ranking() {
    let (drive, contract) = setup_restaurants();
    insert_docs(
        &drive,
        &contract,
        "review",
        "grade",
        1,
        &[("alpha", 90), ("beta", 60), ("gamma", 30)],
    );

    let case = RankedCase::avg(false, Some(2));
    let proof = proof_of(run(&drive, &contract, &case, true).expect("prove must succeed"));
    let honest = client_side_query(&contract, &case);
    assert!(honest
        .verify_ranked_top_k_proof(&proof, platform_version())
        .is_ok());

    let mut wrong_k = honest.clone();
    wrong_k.k = 3;
    assert!(
        wrong_k
            .verify_ranked_top_k_proof(&proof, platform_version())
            .is_err(),
        "a proof for k = 2 must not verify as k = 3"
    );

    let mut wrong_direction = honest.clone();
    wrong_direction.descending = false;
    assert!(
        wrong_direction
            .verify_ranked_top_k_proof(&proof, platform_version())
            .is_err(),
        "a TOP proof must not verify as a BOTTOM one"
    );

    // The `review` index declares only the Avg axis, so Count / Sum are
    // not even provable against it — but a client that mislabels the axis
    // must still be rejected rather than mis-decoding the entries.
    for axis in [RankedAxis::Count, RankedAxis::Sum] {
        let mut wrong_axis = honest.clone();
        wrong_axis.axis = axis;
        assert!(
            wrong_axis
                .verify_ranked_top_k_proof(&proof, platform_version())
                .is_err(),
            "an Avg proof must not verify as {axis:?}"
        );
    }
}

/// An unknown verify-method version fails closed rather than silently
/// falling back to v0.
#[test]
fn verify_rejects_an_unknown_method_version() {
    let (drive, contract) = setup_restaurants();
    insert_docs(&drive, &contract, "review", "grade", 1, &[("alpha", 90)]);

    let case = RankedCase::avg(false, Some(1));
    let proof = proof_of(run(&drive, &contract, &case, true).expect("prove must succeed"));
    let query = client_side_query(&contract, &case);

    let mut future = platform_version().clone();
    future
        .drive
        .methods
        .verify
        .document_ranked
        .verify_ranked_top_k_proof = 1;
    assert!(matches!(
        query.verify_ranked_top_k_proof(&proof, &future),
        Err(Error::Drive(
            crate::error::drive::DriveError::UnknownVersionMismatch { .. }
        ))
    ));
}

/// A request whose axis no index declares is rejected at dispatch with a
/// message naming the contract keyword that is missing — the `visit`
/// doctype ranks by count only, so asking it for a sum ranking cannot be
/// served.
#[test]
fn a_request_for_an_undeclared_axis_is_rejected_at_dispatch() {
    let (drive, contract) = setup_restaurants();
    insert_docs(&drive, &contract, "visit", "guests", 1, &[("alpha", 2)]);

    let case = RankedCase {
        document_type_name: "visit",
        group_by_property: GROUP_PROPERTY,
        select: SelectProjection::sum("guests"),
        order_field: "guests",
        ascending: false,
        limit: Some(2),
        offset: None,
    };
    let error = run(&drive, &contract, &case, false)
        .expect_err("the visit index declares only rankedCountable");
    match error {
        Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(message)) => {
            assert!(
                message.contains("rankedSummable"),
                "the error must name the missing keyword, got: {message}"
            );
        }
        other => panic!("expected WhereClauseOnNonIndexedProperty, got {other}"),
    }
}

// ===================================================================
// Behaviour: picking between several indexes on one doctype
// ===================================================================

/// The second grouping property of the multi-index contract.
const CHEF_PROPERTY: &str = "chefId";

/// A doctype carrying three indexes at once — the shape the shared
/// `restaurants` fixture cannot express, since it spends one doctype per
/// axis to dodge `DuplicateIndexError`:
///
/// | index              | properties               | ranking          |
/// |--------------------|--------------------------|------------------|
/// | `byChef`           | `chefId`                 | Count            |
/// | `byChefRestaurant` | `chefId`, `restaurantId` | none             |
/// | `byRestaurant`     | `restaurantId`           | Avg over `grade` |
///
/// Three things about this contract are load-bearing and none of them
/// are reachable from the fixture:
///
/// 1. **Two ranked indexes, different axes, different properties.** The
///    picker has to choose on `(group property, axis, field)` rather
///    than "the doctype's one ranked index", and the two indexes land on
///    *different* grove paths and different stored tree types —
///    `byChef` is a `ProvableCountIndexedTree`, `byRestaurant` a
///    `ProvableCountProvableSumIndexedTree` carrying the Avg axis.
/// 2. **A non-ranked index over a ranked index's property.** Two indexes
///    on `[chefId]` alone would be a `DuplicateIndexError`, so the
///    non-ranked sibling is the compound `[chefId, restaurantId]` —
///    which is the sharper case anyway: index levels are merged into one
///    trie keyed by property name, so `byChefRestaurant` hangs a
///    `restaurantId` property-name tree underneath the very value trees
///    whose counts `byChef`'s ranked secondary is ranking. The write path
///    wraps those continuations `NonCounted`; if that wrapping were
///    wrong, the counts asserted below would drift.
/// 3. **The picker must not answer from the compound index.** It carries
///    `chefId` as its first property but no ranking, and it is
///    single-property-ness — not mere presence of the property — that
///    qualifies an index.
///
/// The compound sibling hangs off `byChef` (count-only) rather than off
/// `byRestaurant` (count+sum) because that is the narrower shape for a
/// *picker* test: a count-only parent takes the `NonCounted` wrapper on
/// both sides of the v14 boundary, so this fixture reads identically
/// whichever index walker generation runs it.
///
/// Hanging it off `byRestaurant` is no longer impossible, though. Until
/// v14 a plain `NormalTree` continuation under a count+sum value tree
/// was refused outright (the wrapper matrix had only its diagonal), which
/// made *any* summable index terminating at `[a]` incompatible with a
/// compound index `[a, …]` on the same doctype; the v14 shared-prefix fix
/// completes the matrix and demotes the value trees, so ranked and
/// compound indexes can now share a count+sum prefix. That combination is
/// covered end to end by
/// `ranked_index_ranks_correctly_next_to_a_compound_index_sharing_its_property`
/// in `drive::contract::insert::insert_contract::v0::tests::ranked_index_e2e_tests`;
/// keeping it out of this fixture keeps the picker assertions below about
/// index selection only.
fn multi_index_contract() -> DataContract {
    use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;

    let json = serde_json::json!({
        "$formatVersion": "0",
        "id": "94zNLp7A1ZcYG3Egqf2YmQk4DQr9P8D543GwXyCJRz4",
        "ownerId": "AcYUCSvAmUwryNsQqkqqD1o3BnFuzepGtR3Mhh2swLk6",
        "version": 1,
        "documentSchemas": {
            "dish": {
                "type": "object",
                "documentsMutable": true,
                "canBeDeleted": true,
                "indices": [
                    {
                        "name": "byRestaurant",
                        "properties": [{ "restaurantId": "asc" }],
                        "countable": "countable",
                        "summable": "grade",
                        "averageable": "grade",
                        "rangeCountable": true,
                        "rangeSummable": true,
                        "rangeAverageable": true,
                        "rankedAverageable": true
                    },
                    {
                        "name": "byChef",
                        "properties": [{ "chefId": "asc" }],
                        "countable": "countable",
                        "rangeCountable": true,
                        "rankedCountable": true
                    },
                    {
                        "name": "byChefRestaurant",
                        "properties": [
                            { "chefId": "asc" },
                            { "restaurantId": "asc" }
                        ]
                    }
                ],
                "properties": {
                    "restaurantId": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 32,
                        "position": 0
                    },
                    "chefId": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 32,
                        "position": 1
                    },
                    "grade": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 100,
                        "position": 2
                    }
                },
                "required": ["restaurantId", "chefId", "grade"],
                "additionalProperties": false
            }
        }
    });

    // Same non-validating load `setup_restaurants` gets through
    // `json_document_to_contract`: the meta-schema acceptance of the
    // ranked keywords is rs-dpp's to test, and re-running it here would
    // only couple this test to the meta schema.
    DataContract::from_json(json, false, platform_version())
        .expect("expected to parse the multi-index contract")
}

/// Apply [`multi_index_contract`] to a fresh Drive.
fn setup_multi_index() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = multi_index_contract();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version(),
        )
        .expect("expected to apply the multi-index contract");
    (drive, contract)
}

/// Insert `(restaurant, chef, grade)` rows as `dish` documents through
/// the ordinary write path, so every index — ranked and not — is
/// maintained by the code under test.
fn insert_dishes(drive: &Drive, contract: &DataContract, rows: &[(&str, &str, i64)]) {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("dish")
        .expect("dish doctype exists");
    for (i, (restaurant, chef, grade)) in rows.iter().enumerate() {
        let mut doc: Document = document_type
            .random_document(Some(i as u64 + 1), pv)
            .expect("random document");
        let mut props = BTreeMap::new();
        props.insert(
            GROUP_PROPERTY.to_string(),
            Value::Text(restaurant.to_string()),
        );
        props.insert(CHEF_PROPERTY.to_string(), Value::Text(chef.to_string()));
        props.insert("grade".to_string(), Value::I64(*grade));
        doc.set_properties(props);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .unwrap_or_else(|e| panic!("expected to insert a dish document: {e}"));
    }
}

/// `SELECT AVG(grade) … GROUP BY restaurantId` against the multi-index
/// doctype.
fn dish_avg_case(ascending: bool, limit: Option<u32>) -> RankedCase {
    RankedCase {
        document_type_name: "dish",
        group_by_property: GROUP_PROPERTY,
        select: SelectProjection::avg("grade"),
        order_field: "grade",
        ascending,
        limit,
        offset: None,
    }
}

/// `SELECT COUNT(*) … GROUP BY chefId` against the multi-index doctype.
fn dish_count_case(ascending: bool, limit: Option<u32>) -> RankedCase {
    RankedCase {
        document_type_name: "dish",
        group_by_property: CHEF_PROPERTY,
        select: SelectProjection::count_star(),
        order_field: RANKED_COUNT_ORDER_KEY,
        ascending,
        limit,
        offset: None,
    }
}

fn dish_indexes(contract: &DataContract) -> &BTreeMap<String, Index> {
    contract
        .document_types()
        .get("dish")
        .expect("dish doctype exists")
        .indexes()
}

/// **The multi-index picking seam.**
///
/// One doctype, three indexes, two of them ranked on different
/// properties and different axes. Each ranked request must resolve to
/// its own index — and the client, rebuilding the query from nothing but
/// the contract and the request, must resolve the *same* one, because
/// the index is what fixes the grove path the proof is read against. A
/// disagreement here is not a wrong answer, it is an unverifiable one.
///
/// Data (restaurant, chef, grade):
///
/// - alpha: 90, 80          → avg 85
/// - beta:  60, 70, 50      → avg 60
/// - gamma: 95              → avg 95
///
/// which by chef is ann = 3 dishes, bob = 2, cid = 1. Both rankings are
/// tie-free so the assertions pin the ranking, not the tie-break rule
/// (that is `ties_break_by_group_key_in_the_walk_direction`'s job).
#[test]
fn a_doctype_with_several_indexes_ranks_each_group_property_on_its_own_index() {
    let (drive, contract) = setup_multi_index();
    insert_dishes(
        &drive,
        &contract,
        &[
            ("alpha", "ann", 90),
            ("alpha", "bob", 80),
            ("beta", "ann", 60),
            ("beta", "ann", 70),
            ("beta", "bob", 50),
            ("gamma", "cid", 95),
        ],
    );

    // (a) Grouping by `restaurantId` resolves the Avg-ranked index and
    //     ranks by the average of `grade`.
    let by_restaurant = dish_avg_case(false, Some(3));
    let avg_entries =
        entries_of(run(&drive, &contract, &by_restaurant, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&avg_entries),
        vec!["gamma", "alpha", "beta"],
        "descending by average: gamma(95) > alpha(85) > beta(60)"
    );
    assert_eq!(
        avg_entries[1].value,
        RankedEntryValue::AvgFixedPoint(85 * RANKED_AVG_SCALE),
        "alpha's two dishes average 85"
    );

    // (b) Grouping by `chefId` resolves the Count-ranked index instead —
    //     a different property, a different axis, a different stored
    //     tree type, on the same doctype and the same documents.
    let by_chef = dish_count_case(false, Some(3));
    let count_entries =
        entries_of(run(&drive, &contract, &by_chef, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&count_entries),
        vec!["ann", "bob", "cid"],
        "descending by dish count: ann(3) > bob(2) > cid(1)"
    );
    assert_eq!(
        count_entries[0].value,
        RankedEntryValue::Count(3),
        "ann's count stays 3 despite `byChefRestaurant` hanging a \
         `restaurantId` continuation tree inside the same value tree — the \
         write path wraps it `NonCounted`, and a missing wrapper would show \
         up right here as an inflated count"
    );
    assert_eq!(count_entries[1].value, RankedEntryValue::Count(2));
    assert_eq!(count_entries[2].value, RankedEntryValue::Count(1));

    // (c) The client-side rebuild lands on the same index for the same
    //     request — asserted by name, then proved by the round trip:
    //     the verifier reads the proof against the path this index
    //     determines, so a divergent pick could not reconstruct the live
    //     root hash.
    let indexes = dish_indexes(&contract);
    assert_eq!(
        client_side_query(&contract, &by_restaurant).index.name,
        "byRestaurant"
    );
    assert_eq!(client_side_query(&contract, &by_chef).index.name, "byChef");
    assert_eq!(
        find_ranked_index_for_axis(indexes, GROUP_PROPERTY, &[], RankedAxis::Avg, "grade")
            .expect("the Avg ranking resolves")
            .name,
        "byRestaurant",
    );
    assert_eq!(
        find_ranked_index_for_axis(indexes, CHEF_PROPERTY, &[], RankedAxis::Count, "")
            .expect("the Count ranking resolves")
            .name,
        "byChef",
    );

    assert_proof_round_trips(&drive, &contract, &by_restaurant, &avg_entries);
    assert_proof_round_trips(&drive, &contract, &by_chef, &count_entries);

    // (d) The same agreement under an offset. This is the sharper form
    //     of (c): a paginated proof binds `offset` as well as the path,
    //     so a client that resolved a *different* index would fail to
    //     reconstruct the root hash, and one that resolved the right
    //     index but the wrong page would fail the offset check. Both
    //     failure modes are live here because two ranked indexes exist.
    let second_best_restaurant = dish_avg_case(false, Some(1)).at_offset(1);
    let page =
        page_of(run(&drive, &contract, &second_best_restaurant, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&page.entries),
        vec!["alpha"],
        "gamma(95) > alpha(85) > beta(60) — rank 1 is alpha"
    );
    let verified =
        assert_proof_round_trips(&drive, &contract, &second_best_restaurant, &page.entries);
    assert_eq!(verified.skipped, 1);

    let second_busiest_chef = dish_count_case(false, Some(1)).at_offset(1);
    let page =
        page_of(run(&drive, &contract, &second_busiest_chef, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&page.entries),
        vec!["bob"],
        "ann(3) > bob(2) > cid(1) — rank 1 is bob"
    );
    let verified = assert_proof_round_trips(&drive, &contract, &second_busiest_chef, &page.entries);
    assert_eq!(verified.skipped, 1);
    assert_eq!(
        client_side_query(&contract, &second_busiest_chef)
            .index
            .name,
        "byChef",
        "the paginated request resolves the same index as the unpaginated one"
    );

    // The non-ranked compound index is never a candidate even though it
    // leads with `chefId`, which is exactly the property this request
    // groups by. `byChef` declares only the Count axis, so an Avg
    // ranking over chefs has no index — and `byChefRestaurant` must not
    // be press-ganged into serving it.
    assert!(
        find_ranked_index_for_axis(indexes, CHEF_PROPERTY, &[], RankedAxis::Avg, "grade").is_none(),
        "`byChefRestaurant` leads with chefId but is compound and unranked"
    );
    let avg_by_chef = RankedCase {
        group_by_property: CHEF_PROPERTY,
        ..dish_avg_case(false, Some(2))
    };
    let error = run(&drive, &contract, &avg_by_chef, false)
        .expect_err("ranking chefs by average grade is not served by any index");
    assert!(
        matches!(
            &error,
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(message))
                if message.contains("rankedAverageable")
        ),
        "the error must name the missing keyword, got: {error}"
    );
}

mod pinned_prefix {
    //! `SELECT AVG(grade) FROM grades WHERE identityId = X GROUP BY
    //! class ORDER BY AVG(grade) DESC LIMIT k` — the ranked top-k
    //! surface over a compound ranked index `[identityId, class]` with
    //! its leading property pinned. Per-prefix semantics: the walk
    //! reads (and proves) the pinned identity's own secondary, so two
    //! identities' class rankings never mix. The having-path sibling
    //! (`drive_document_having_query::tests::pinned_prefix`) shares the
    //! fixture and pins the write path; this module pins the rank walk
    //! and its paginated proof.

    use super::super::drive_dispatcher::{DocumentRankedRequest, DocumentRankedResponse};
    use super::super::index_picker::{encode_prefix_branches, resolve_ranked_query_for_mode};
    use super::super::mode_detection::{detect_ranked_mode, detect_ranked_mode_v0};
    use super::super::PrefixPin;
    use super::super::{DriveDocumentRankedQuery, RankedEntry, RankedEntryValue};
    use crate::drive::Drive;
    use crate::error::query::QuerySyntaxError;
    use crate::error::Error;
    use crate::query::drive_document_ranked_query::RankedPaginationInputs;
    use crate::query::projection::SelectProjection;
    use crate::query::{OrderClause, WhereClause, WhereOperator};
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::{Document, DocumentV0Setters};
    use dpp::platform_value::Value;
    use dpp::prelude::DataContract;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::PlatformVersion;
    use grovedb::element::indexed::compute_avg_fixed_point;
    use grovedb::TransactionArg;
    use std::collections::BTreeMap;

    const PREFIX_PROPERTY: &str = "identityId";
    const CLASS_PROPERTY: &str = "class";
    const DOCUMENT_TYPE: &str = "grade";
    const IDENTITY_X: [u8; 32] = [1u8; 32];
    const IDENTITY_Y: [u8; 32] = [2u8; 32];

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    fn setup_grades_compound_ranked() -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = platform_version();
        let contract = json_document_to_contract(
            "tests/supporting_files/contract/grades/grades-compound-ranked-contract.json",
            false,
            pv,
        )
        .expect("expected to parse the compound ranked grades contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply the compound ranked grades contract");
        (drive, contract)
    }

    fn insert_grades(drive: &Drive, contract: &DataContract, rows: &[([u8; 32], &str, i64)]) {
        let pv = platform_version();
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("grade doctype exists");
        for (i, (identity, class, grade)) in rows.iter().enumerate() {
            let mut doc: Document = document_type
                .random_document(Some(4000 + i as u64), pv)
                .expect("random document");
            let mut props = BTreeMap::new();
            props.insert(PREFIX_PROPERTY.to_string(), Value::Identifier(*identity));
            props.insert(CLASS_PROPERTY.to_string(), Value::Text(class.to_string()));
            props.insert("grade".to_string(), Value::I64(*grade));
            doc.set_properties(props);
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert a grade document");
        }
    }

    fn pin(identity: [u8; 32]) -> Vec<WhereClause> {
        vec![WhereClause {
            field: PREFIX_PROPERTY.to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(identity),
        }]
    }

    fn run(
        drive: &Drive,
        contract: &DataContract,
        where_clauses: &[WhereClause],
        limit: u32,
        prove: bool,
    ) -> Result<DocumentRankedResponse, Error> {
        let group_by = vec![CLASS_PROPERTY.to_string()];
        let order_by = vec![OrderClause {
            field: "grade".to_string(),
            ascending: false,
        }];
        drive.execute_document_ranked_request(
            DocumentRankedRequest {
                contract,
                document_type: contract
                    .document_type_for_name(DOCUMENT_TYPE)
                    .expect("grade doctype exists"),
                group_by: &group_by,
                select: SelectProjection::avg("grade"),
                having: &[],
                order_by: &order_by,
                where_clauses,
                limit: Some(limit),
                offset: None,
                has_start_at: false,
                prove,
            },
            None,
            platform_version(),
        )
    }

    fn client_side_query<'a>(
        contract: &'a DataContract,
        where_clauses: &[WhereClause],
        limit: u32,
    ) -> DriveDocumentRankedQuery<'a> {
        let group_by = vec![CLASS_PROPERTY.to_string()];
        let order_by = vec![OrderClause {
            field: "grade".to_string(),
            ascending: false,
        }];
        let mode = detect_ranked_mode(
            &SelectProjection::avg("grade"),
            &group_by,
            &[],
            &order_by,
            where_clauses,
            RankedPaginationInputs {
                limit: Some(limit),
                offset: None,
                has_start_at: false,
            },
            platform_version(),
        )
        .expect("the case is well-formed");
        let indexes = contract
            .document_types()
            .get(DOCUMENT_TYPE)
            .expect("grade doctype exists")
            .indexes();
        resolve_ranked_query_for_mode(
            contract.id_ref().to_buffer(),
            contract
                .document_type_for_name(DOCUMENT_TYPE)
                .expect("grade doctype exists"),
            DOCUMENT_TYPE.to_string(),
            indexes,
            &mode,
            platform_version(),
        )
        .expect("the fixture's compound index covers the pinned request")
    }

    /// Top-k per pinned prefix, with the paginated proof round-tripped
    /// against the live root hash — and isolation between prefixes:
    /// X's and Y's class rankings come off different secondaries.
    #[test]
    fn top_k_over_pinned_prefix_reads_and_proves() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_grades(
            &drive,
            &contract,
            &[
                // X: art 90, english 80.5, math 80, history 70.
                (IDENTITY_X, "math", 80),
                (IDENTITY_X, "math", 80),
                (IDENTITY_X, "english", 80),
                (IDENTITY_X, "english", 81),
                (IDENTITY_X, "art", 85),
                (IDENTITY_X, "art", 95),
                (IDENTITY_X, "history", 60),
                (IDENTITY_X, "history", 80),
                // Y: science 95, math 92.5.
                (IDENTITY_Y, "math", 90),
                (IDENTITY_Y, "math", 95),
                (IDENTITY_Y, "science", 95),
                (IDENTITY_Y, "science", 95),
            ],
        );

        // X's top 2 by average, best first: art (90) then english (80.5).
        // Y's science (95) and math (92.5) would both outrank them if the
        // prefixes shared a secondary.
        let x_pin = pin(IDENTITY_X);
        let page = match run(&drive, &contract, &x_pin, 2, false).expect("read succeeds") {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries, got a proof"),
        };
        assert_eq!(page.skipped, 0);
        assert_eq!(
            page.entries,
            vec![
                RankedEntry {
                    in_key: None,
                    key: b"art".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(180, 2)),
                },
                RankedEntry {
                    in_key: None,
                    key: b"english".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(161, 2)),
                },
            ],
            "X's own top 2: art 90 then english 80.5"
        );

        let proof = match run(&drive, &contract, &x_pin, 2, true).expect("prove succeeds") {
            DocumentRankedResponse::Proof(proof) => proof,
            DocumentRankedResponse::Entries(_) => panic!("expected a proof, got entries"),
        };
        let (root_hash, verified) = client_side_query(&contract, &x_pin, 2)
            .verify_ranked_top_k_proof(&proof, platform_version())
            .expect("the proof must verify");
        assert_eq!(
            verified.entries, page.entries,
            "verified entries must equal the unproven read"
        );
        assert_eq!(
            root_hash,
            drive
                .grove
                .root_hash(None, &platform_version().drive.grove_version)
                .unwrap()
                .expect("root hash must be readable"),
            "the proof must reconstruct the live grovedb root hash"
        );

        // Y's own ranking, proving the prefixes are separate secondaries.
        let y_pin = pin(IDENTITY_Y);
        let y_page = match run(&drive, &contract, &y_pin, 2, false).expect("read succeeds") {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries, got a proof"),
        };
        assert_eq!(
            y_page
                .entries
                .iter()
                .map(|e| e.key.as_slice())
                .collect::<Vec<_>>(),
            vec![b"science".as_slice(), b"math".as_slice()],
            "Y's own top 2: science 95 then math 92.5"
        );
    }

    /// A **null** pin addresses the prefix subtree the write path
    /// creates for an *absent* optional leading property: the walkers
    /// encode a missing value as an empty path segment
    /// (`get_raw_for_document_type(..).unwrap_or_default()`), so
    /// `WHERE tag == null` must resolve to that empty segment — read,
    /// proof, and client-side verification all reconstructing the same
    /// stored path. Documents that *do* carry a tag live under their
    /// own prefix and must not leak into the null prefix's ranking.
    #[test]
    fn a_null_pin_addresses_the_absent_value_prefix() {
        const TAGGED_DOCTYPE: &str = "taggedGrade";
        let (drive, contract) = setup_grades_compound_ranked();
        let pv = platform_version();
        let document_type = contract
            .document_type_for_name(TAGGED_DOCTYPE)
            .expect("taggedGrade doctype exists");

        // Tagless rows land under the empty-segment prefix; the
        // "honors" row must stay in its own prefix.
        for (i, (tag, class, grade)) in [
            (None, "math", 80i64),
            (None, "math", 90),
            (None, "science", 60),
            (Some("honors"), "math", 100),
        ]
        .iter()
        .enumerate()
        {
            let mut doc: Document = document_type
                .random_document(Some(7000 + i as u64), pv)
                .expect("random document");
            let mut props = BTreeMap::new();
            if let Some(tag) = tag {
                props.insert("tag".to_string(), Value::Text(tag.to_string()));
            }
            props.insert(CLASS_PROPERTY.to_string(), Value::Text(class.to_string()));
            props.insert("grade".to_string(), Value::I64(*grade));
            doc.set_properties(props);
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert a tagged grade document");
        }

        let null_pin = vec![WhereClause {
            field: "tag".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Null,
        }];
        let group_by = vec![CLASS_PROPERTY.to_string()];
        let order_by = vec![OrderClause {
            field: "grade".to_string(),
            ascending: false,
        }];
        let request = |prove: bool| DocumentRankedRequest {
            contract: &contract,
            document_type,
            group_by: &group_by,
            select: SelectProjection::avg("grade"),
            having: &[],
            order_by: &order_by,
            where_clauses: &null_pin,
            limit: Some(2),
            offset: None,
            has_start_at: false,
            prove,
        };

        let page = match drive
            .execute_document_ranked_request(request(false), None, pv)
            .expect("the null-pinned read succeeds")
        {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries, got a proof"),
        };
        assert_eq!(
            page.entries,
            vec![
                RankedEntry {
                    in_key: None,
                    key: b"math".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(170, 2)),
                },
                RankedEntry {
                    in_key: None,
                    key: b"science".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(60, 1)),
                },
            ],
            "the tagless ranking: math 85 then science 60 — the honors row \
             (math 100) must not leak into the null prefix"
        );

        // Proof round trip through the shared resolver, so the verifier
        // reconstructs the same empty-segment path the prover walked.
        let proof = match drive
            .execute_document_ranked_request(request(true), None, pv)
            .expect("the null-pinned prove succeeds")
        {
            DocumentRankedResponse::Proof(proof) => proof,
            DocumentRankedResponse::Entries(_) => panic!("expected a proof, got entries"),
        };
        let mode = detect_ranked_mode(
            &SelectProjection::avg("grade"),
            &group_by,
            &[],
            &order_by,
            &null_pin,
            RankedPaginationInputs {
                limit: Some(2),
                offset: None,
                has_start_at: false,
            },
            pv,
        )
        .expect("the null-pinned case is well-formed");
        let query = resolve_ranked_query_for_mode(
            contract.id_ref().to_buffer(),
            document_type,
            TAGGED_DOCTYPE.to_string(),
            contract
                .document_types()
                .get(TAGGED_DOCTYPE)
                .expect("taggedGrade doctype exists")
                .indexes(),
            &mode,
            pv,
        )
        .expect("the compound index covers the null-pinned request");
        assert_eq!(
            query.prefix_branches,
            vec![vec![Vec::<u8>::new()]],
            "a null pin must encode as the write path's empty segment"
        );
        let (root_hash, verified) = query
            .verify_ranked_top_k_proof(&proof, pv)
            .expect("the null-pinned proof must verify");
        assert_eq!(verified.entries, page.entries);
        assert_eq!(
            root_hash,
            drive
                .grove
                .root_hash(None, &pv.drive.grove_version)
                .unwrap()
                .expect("root hash must be readable"),
        );
    }

    fn in_pin(identities: &[[u8; 32]]) -> Vec<WhereClause> {
        vec![WhereClause {
            field: PREFIX_PROPERTY.to_string(),
            operator: WhereOperator::In,
            value: Value::Array(
                identities
                    .iter()
                    .map(|identity| Value::Identifier(*identity))
                    .collect(),
            ),
        }]
    }

    /// `identityId IN [X, Y]` walks each identity's own secondary and
    /// merges: descending aggregate order, entries tagged with their
    /// branch's `in_key`, and a **cross-prefix aggregate tie** breaking
    /// by encoded prefix ascending (X's 32 `1`-bytes before Y's `2`s) —
    /// the comparator's middle term, observable only here. The proof is
    /// one branched `PathQuery` envelope, round-tripped through the
    /// shared resolver against the live root hash.
    #[test]
    fn in_pinned_top_k_merges_branches_and_proves() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_grades(
            &drive,
            &contract,
            &[
                // X: art 90, math 80. Y: science 95, history 90 — the
                // two 90s are the cross-prefix tie.
                (IDENTITY_X, "art", 90),
                (IDENTITY_X, "math", 80),
                (IDENTITY_Y, "science", 95),
                (IDENTITY_Y, "history", 90),
            ],
        );

        // Request order [Y, X] deliberately reversed: canonical branch
        // order is by encoded prefix, not by element order.
        let pins = in_pin(&[IDENTITY_Y, IDENTITY_X]);
        let page = match run(&drive, &contract, &pins, 3, false).expect("read succeeds") {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries, got a proof"),
        };
        assert_eq!(page.skipped, 0);
        assert_eq!(
            page.entries,
            vec![
                RankedEntry {
                    in_key: Some(IDENTITY_Y.to_vec()),
                    key: b"science".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(95, 1)),
                },
                RankedEntry {
                    in_key: Some(IDENTITY_X.to_vec()),
                    key: b"art".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(90, 1)),
                },
                RankedEntry {
                    in_key: Some(IDENTITY_Y.to_vec()),
                    key: b"history".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(90, 1)),
                },
            ],
            "top 3 across the union: science 95, then the 90–90 tie broken \
             by encoded prefix ascending (X's art before Y's history)"
        );

        let proof = match run(&drive, &contract, &pins, 3, true).expect("prove succeeds") {
            DocumentRankedResponse::Proof(proof) => proof,
            DocumentRankedResponse::Entries(_) => panic!("expected a proof, got entries"),
        };
        let query = client_side_query(&contract, &pins, 3);
        assert_eq!(query.prefix_branches.len(), 2, "two branches resolved");
        let (root_hash, verified) = query
            .verify_ranked_top_k_proof(&proof, platform_version())
            .expect("the branched envelope must verify");
        assert_eq!(verified.entries, page.entries);
        assert_eq!(
            root_hash,
            drive
                .grove
                .root_hash(None, &platform_version().drive.grove_version)
                .unwrap()
                .expect("root hash must be readable"),
        );
    }

    /// Platform-level tamper cases on the branched envelope. The deep
    /// matrix — reordered branch keys, duplicated or dropped branch
    /// tails, echo mismatches — is pinned in grovedb's
    /// `indexed_axis_branched_proof_tests`, since the envelope is one
    /// grovedb proof now; here we pin what the platform layer itself
    /// must not confuse: corrupted bytes, and the single-branch /
    /// multi-branch envelope shapes never cross-verifying.
    #[test]
    fn tampered_or_mismatched_branched_proofs_do_not_verify() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_grades(
            &drive,
            &contract,
            &[(IDENTITY_X, "art", 90), (IDENTITY_Y, "science", 95)],
        );
        let pins = in_pin(&[IDENTITY_X, IDENTITY_Y]);
        let proof = match run(&drive, &contract, &pins, 2, true).expect("prove succeeds") {
            DocumentRankedResponse::Proof(proof) => proof,
            DocumentRankedResponse::Entries(_) => panic!("expected a proof, got entries"),
        };
        let query = client_side_query(&contract, &pins, 2);

        // Baseline sanity: untampered verifies.
        query
            .verify_ranked_top_k_proof(&proof, platform_version())
            .expect("untampered branched envelope verifies");

        // Corrupted bytes.
        let mut corrupted = proof.clone();
        let mid = corrupted.len() / 2;
        corrupted[mid] ^= 0xFF;
        assert!(
            query
                .verify_ranked_top_k_proof(&corrupted, platform_version())
                .is_err(),
            "a flipped byte must not verify"
        );

        // Truncated bytes.
        let truncated = &proof[..proof.len() - 8];
        assert!(
            query
                .verify_ranked_top_k_proof(truncated, platform_version())
                .is_err(),
            "a truncated envelope must not verify"
        );

        // A single-pin proof must not verify under the branched query,
        // nor a branched proof under a single-pin query — the two
        // envelope shapes are distinct grovedb types.
        let single = pin(IDENTITY_X);
        let single_proof = match run(&drive, &contract, &single, 2, true).expect("prove") {
            DocumentRankedResponse::Proof(proof) => proof,
            DocumentRankedResponse::Entries(_) => panic!("expected a proof"),
        };
        assert!(
            query
                .verify_ranked_top_k_proof(&single_proof, platform_version())
                .is_err(),
            "a single-branch envelope must not verify under an IN query"
        );
        let single_query = client_side_query(&contract, &single, 2);
        assert!(
            single_query
                .verify_ranked_top_k_proof(&proof, platform_version())
                .is_err(),
            "a branched envelope must not verify under a single-pin query"
        );
    }

    /// A single-element `IN` is normalized to an equality pin at
    /// grammar time: same resolved branch set, same entries, and the
    /// **same proof bytes** — the degenerate case is byte-identical to
    /// `==`, so no client can observe which spelling was used.
    #[test]
    fn single_element_in_is_byte_identical_to_an_equality_pin() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_grades(&drive, &contract, &[(IDENTITY_X, "art", 90)]);

        let equality = pin(IDENTITY_X);
        let single_in = in_pin(&[IDENTITY_X]);

        let eq_query = client_side_query(&contract, &equality, 2);
        let in_query = client_side_query(&contract, &single_in, 2);
        assert_eq!(eq_query.prefix_branches, in_query.prefix_branches);

        let eq_proof = match run(&drive, &contract, &equality, 2, true).expect("prove") {
            DocumentRankedResponse::Proof(proof) => proof,
            DocumentRankedResponse::Entries(_) => panic!("expected a proof"),
        };
        let in_proof = match run(&drive, &contract, &single_in, 2, true).expect("prove") {
            DocumentRankedResponse::Proof(proof) => proof,
            DocumentRankedResponse::Entries(_) => panic!("expected a proof"),
        };
        assert_eq!(eq_proof, in_proof, "no container for a single branch");

        let eq_page = match run(&drive, &contract, &equality, 2, false).expect("read") {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries"),
        };
        let in_page = match run(&drive, &contract, &single_in, 2, false).expect("read") {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries"),
        };
        assert_eq!(
            eq_page.entries, in_page.entries,
            "a singleton IN reads exactly what the equality pin reads"
        );
        assert!(
            eq_page.entries.iter().all(|e| e.in_key.is_none()),
            "single-branch entries carry no in_key"
        );
    }

    /// `OFFSET` is rejected together with an `IN` pin — rank-skip is
    /// attested per-secondary and has no meaning across a branch union.
    #[test]
    fn offset_is_rejected_with_an_in_pin() {
        let error = detect_ranked_mode_v0(
            &SelectProjection::avg("grade"),
            &[CLASS_PROPERTY.to_string()],
            &[],
            &[OrderClause {
                field: "grade".to_string(),
                ascending: false,
            }],
            &in_pin(&[IDENTITY_X, IDENTITY_Y]),
            RankedPaginationInputs {
                limit: Some(2),
                offset: Some(1),
                has_start_at: false,
            },
        )
        .expect_err("offset cannot combine with IN");
        match error {
            Error::Query(QuerySyntaxError::InvalidLimit(message)) => {
                assert!(
                    message.contains("OFFSET") && message.contains("IN"),
                    "the rejection must name the offset × IN exclusion, got: {message}"
                );
            }
            other => panic!("expected InvalidLimit, got {other:?}"),
        }
    }

    /// Grammar rejections around the `IN` pin: over-cap element lists,
    /// empty lists, a second `IN`, a non-array operand, and elements
    /// that encode to the same segment.
    #[test]
    fn in_pin_shape_rejections() {
        let detect = |where_clauses: &[WhereClause]| {
            detect_ranked_mode_v0(
                &SelectProjection::avg("grade"),
                &[CLASS_PROPERTY.to_string()],
                &[],
                &[OrderClause {
                    field: "grade".to_string(),
                    ascending: false,
                }],
                where_clauses,
                RankedPaginationInputs {
                    limit: Some(2),
                    offset: None,
                    has_start_at: false,
                },
            )
        };

        // Over the branch ceiling.
        let identities: Vec<[u8; 32]> = (0..11).map(|i| [i as u8 + 1; 32]).collect();
        let error = detect(&in_pin(&identities)).expect_err("11 branches is over the cap");
        match error {
            Error::Query(QuerySyntaxError::InvalidParameter(message)) => {
                assert!(
                    message.contains("ceiling of 10"),
                    "the rejection must name the ceiling, got: {message}"
                );
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }

        // Empty element list.
        let empty = vec![WhereClause {
            field: PREFIX_PROPERTY.to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![]),
        }];
        assert!(detect(&empty).is_err(), "an empty IN list must be rejected");

        // Two *branching* INs are rejected in either clause order.
        let multi = |field: &str| WhereClause {
            field: field.to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![Value::U64(1), Value::U64(2)]),
        };
        for two_ins in [
            vec![multi(PREFIX_PROPERTY), multi("other")],
            vec![multi("other"), multi(PREFIX_PROPERTY)],
        ] {
            let error = detect(&two_ins).expect_err("two branching INs must be rejected");
            assert!(
                matches!(error, Error::Query(QuerySyntaxError::Unsupported(_))),
                "expected Unsupported for a second branching IN, got {error:?}"
            );
        }

        // A singleton IN is an equality pin and never counts against
        // the one-`IN` budget — in either clause order relative to the
        // branching one.
        let singleton = WhereClause {
            field: "other".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![Value::U64(1)]),
        };
        for mixed in [
            vec![multi(PREFIX_PROPERTY), singleton.clone()],
            vec![singleton.clone(), multi(PREFIX_PROPERTY)],
        ] {
            let mode = detect(&mixed)
                .expect("a singleton IN alongside a branching IN is well-formed either way");
            assert_eq!(
                mode.prefix_pins
                    .iter()
                    .map(|pin| pin.values.len())
                    .collect::<Vec<_>>(),
                if mixed[0].field == "other" {
                    vec![1, 2]
                } else {
                    vec![2, 1]
                },
                "one branching pin and one singleton pin, in clause order"
            );
        }

        // Non-array operand.
        let scalar = vec![WhereClause {
            field: PREFIX_PROPERTY.to_string(),
            operator: WhereOperator::In,
            value: Value::U64(7),
        }];
        assert!(
            detect(&scalar).is_err(),
            "a scalar IN operand must be rejected"
        );

        // Duplicate elements surface at the encoder (post-encoding
        // distinctness), through the resolver.
        let (_, contract) = setup_grades_compound_ranked();
        let duplicated = in_pin(&[IDENTITY_X, IDENTITY_X]);
        let mode = detect(&duplicated).expect("shape-valid; duplicates are the encoder's call");
        let error = resolve_ranked_query_for_mode(
            contract.id_ref().to_buffer(),
            contract
                .document_type_for_name(DOCUMENT_TYPE)
                .expect("grade doctype exists"),
            DOCUMENT_TYPE.to_string(),
            contract
                .document_types()
                .get(DOCUMENT_TYPE)
                .expect("grade doctype exists")
                .indexes(),
            &mode,
            platform_version(),
        )
        .expect_err("duplicate encoded elements must be rejected");
        assert!(
            matches!(
                error,
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(_))
            ),
            "expected the duplicate-encoding rejection, got {error:?}"
        );
    }

    /// An `IN` element whose prefix was never written contributes the
    /// **empty branch** — union semantics — while the single-`==`-pin
    /// contract keeps erroring on an unknown value (pinned separately
    /// by `unknown_prefix_value_errors_rather_than_fabricating_an_empty_page`).
    /// The proved path authenticates the absence inside the branched
    /// envelope, so read, proof, and verification agree.
    #[test]
    fn an_absent_in_element_contributes_an_empty_branch() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_grades(&drive, &contract, &[(IDENTITY_X, "art", 90)]);

        let never_written = [9u8; 32];
        let pins = in_pin(&[IDENTITY_X, never_written]);
        let page = match run(&drive, &contract, &pins, 2, false).expect("read succeeds") {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries, got a proof"),
        };
        assert_eq!(
            page.entries,
            vec![RankedEntry {
                in_key: Some(IDENTITY_X.to_vec()),
                key: b"art".to_vec(),
                value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(90, 1)),
            }],
            "only the existing branch contributes; the absent one is empty, not an error"
        );

        let proof = match run(&drive, &contract, &pins, 2, true).expect("prove succeeds") {
            DocumentRankedResponse::Proof(proof) => proof,
            DocumentRankedResponse::Entries(_) => panic!("expected a proof, got entries"),
        };
        let (root_hash, verified) = client_side_query(&contract, &pins, 2)
            .verify_ranked_top_k_proof(&proof, platform_version())
            .expect("the envelope authenticates the absent branch");
        assert_eq!(verified.entries, page.entries);
        assert_eq!(
            root_hash,
            drive
                .grove
                .root_hash(None, &platform_version().drive.grove_version)
                .unwrap()
                .expect("root hash must be readable"),
        );
    }

    /// An unpinned request over the compound-only contract has no
    /// covering index — there is no global cross-prefix ordering to
    /// serve, so the rejection names the missing coverage.
    #[test]
    fn unpinned_prefix_is_rejected() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_grades(&drive, &contract, &[(IDENTITY_X, "math", 80)]);

        let error = run(&drive, &contract, &[], 2, false)
            .expect_err("an unpinned compound prefix must not resolve");
        assert!(
            matches!(
                error,
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(_))
            ),
            "expected a no-covering-index rejection, got {error:?}"
        );
    }

    const DUAL_DOCTYPE: &str = "dualGrade";

    fn insert_dual_grades(
        drive: &Drive,
        contract: &DataContract,
        rows: &[([u8; 32], &str, &str, i64)],
    ) {
        let pv = platform_version();
        let document_type = contract
            .document_type_for_name(DUAL_DOCTYPE)
            .expect("dualGrade doctype exists");
        for (i, (identity, tag, class, grade)) in rows.iter().enumerate() {
            let mut doc: Document = document_type
                .random_document(Some(8000 + i as u64), pv)
                .expect("random document");
            let mut props = BTreeMap::new();
            props.insert(PREFIX_PROPERTY.to_string(), Value::Identifier(*identity));
            props.insert("tag".to_string(), Value::Text(tag.to_string()));
            props.insert(CLASS_PROPERTY.to_string(), Value::Text(class.to_string()));
            props.insert("grade".to_string(), Value::I64(*grade));
            doc.set_properties(props);
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert a dualGrade document");
        }
    }

    fn run_dual(
        drive: &Drive,
        contract: &DataContract,
        where_clauses: &[WhereClause],
        limit: u32,
        prove: bool,
        transaction: TransactionArg,
    ) -> Result<DocumentRankedResponse, Error> {
        let group_by = vec![CLASS_PROPERTY.to_string()];
        let order_by = vec![OrderClause {
            field: "grade".to_string(),
            ascending: false,
        }];
        drive.execute_document_ranked_request(
            DocumentRankedRequest {
                contract,
                document_type: contract
                    .document_type_for_name(DUAL_DOCTYPE)
                    .expect("dualGrade doctype exists"),
                group_by: &group_by,
                select: SelectProjection::avg("grade"),
                having: &[],
                order_by: &order_by,
                where_clauses,
                limit: Some(limit),
                offset: None,
                has_start_at: false,
                prove,
            },
            transaction,
            platform_version(),
        )
    }

    /// An `IN` element whose branch-key tree EXISTS (another pin value was
    /// written under it) but whose deeper pinned path was never written is
    /// an ABSENT branch — empty page, union semantics — not an error that
    /// discards the other branches' results. grovedb's branched reader and
    /// prover authenticate absence at any depth of the branch chain, so the
    /// unproved executor must walk the whole chain too.
    #[test]
    fn an_in_element_with_an_absent_deeper_pin_contributes_an_empty_branch() {
        let (drive, contract) = setup_grades_compound_ranked();
        // Y's tree exists (via tag "t2"), but Y/t1 was never written.
        insert_dual_grades(
            &drive,
            &contract,
            &[
                (IDENTITY_X, "t1", "math", 90),
                (IDENTITY_Y, "t2", "math", 80),
            ],
        );

        let mut pins = in_pin(&[IDENTITY_X, IDENTITY_Y]);
        pins.push(WhereClause {
            field: "tag".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("t1".to_string()),
        });

        let page = match run_dual(&drive, &contract, &pins, 2, false, None)
            .expect("a present branch key with an absent deeper pin must not error")
        {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries, got a proof"),
        };
        assert_eq!(
            page.entries,
            vec![RankedEntry {
                in_key: Some(IDENTITY_X.to_vec()),
                key: b"math".to_vec(),
                value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(90, 1)),
            }],
            "X/t1 contributes; Y (present key, absent t1 suffix) is an empty branch"
        );

        // The proved path must agree byte-for-byte with the unproved one.
        let proof = match run_dual(&drive, &contract, &pins, 2, true, None)
            .expect("the branched envelope authenticates the absent suffix")
        {
            DocumentRankedResponse::Proof(proof) => proof,
            DocumentRankedResponse::Entries(_) => panic!("expected a proof, got entries"),
        };
        let group_by = vec![CLASS_PROPERTY.to_string()];
        let order_by = vec![OrderClause {
            field: "grade".to_string(),
            ascending: false,
        }];
        let pv = platform_version();
        let mode = detect_ranked_mode(
            &SelectProjection::avg("grade"),
            &group_by,
            &[],
            &order_by,
            &pins,
            RankedPaginationInputs {
                limit: Some(2),
                offset: None,
                has_start_at: false,
            },
            pv,
        )
        .expect("well-formed");
        let query = resolve_ranked_query_for_mode(
            contract.id_ref().to_buffer(),
            contract
                .document_type_for_name(DUAL_DOCTYPE)
                .expect("dualGrade doctype exists"),
            DUAL_DOCTYPE.to_string(),
            contract
                .document_types()
                .get(DUAL_DOCTYPE)
                .expect("dualGrade doctype exists")
                .indexes(),
            &mode,
            pv,
        )
        .expect("covered");
        let (root_hash, verified) = query
            .verify_ranked_top_k_proof(&proof, pv)
            .expect("the proof verifies");
        assert_eq!(verified.entries, page.entries);
        assert_eq!(
            root_hash,
            drive
                .grove
                .root_hash(None, &pv.drive.grove_version)
                .unwrap()
                .expect("root hash must be readable"),
        );
    }

    /// A `null` pin addresses its prefix through an EMPTY path segment,
    /// which the branched proof grammar cannot express — so combining it
    /// with an `IN` is rejected at the grammar instead of serving the
    /// unproved read and failing the prove.
    #[test]
    fn a_null_pin_cannot_combine_with_an_in() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_dual_grades(&drive, &contract, &[(IDENTITY_X, "t1", "math", 90)]);

        let mut pins = in_pin(&[IDENTITY_X, IDENTITY_Y]);
        pins.push(WhereClause {
            field: "tag".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Null,
        });

        let error = run_dual(&drive, &contract, &pins, 2, false, None)
            .expect_err("null x IN must be rejected");
        assert!(
            matches!(
                &error,
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(message))
                    if message.contains("`null` pin")
            ),
            "the rejection must name the null x IN exclusion, got {error:?}"
        );
    }

    /// The branched unproved read is ONE grovedb call executed under the
    /// caller's transaction — every absence decision and branch page from
    /// one snapshot. A branch written only inside the transaction is
    /// visible through it and invisible without it.
    #[test]
    fn a_branched_unproved_read_honors_the_transaction() {
        let (drive, contract) = setup_grades_compound_ranked();
        let pv = platform_version();
        insert_grades(&drive, &contract, &[(IDENTITY_X, "math", 80)]);

        let transaction = drive.grove.start_transaction();
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("grade doctype exists");
        let mut doc: Document = document_type
            .random_document(Some(9000), pv)
            .expect("random document");
        let mut props = BTreeMap::new();
        props.insert(PREFIX_PROPERTY.to_string(), Value::Identifier(IDENTITY_Y));
        props.insert(
            CLASS_PROPERTY.to_string(),
            Value::Text("science".to_string()),
        );
        props.insert("grade".to_string(), Value::I64(95));
        doc.set_properties(props);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&transaction),
                pv,
                None,
            )
            .expect("expected to insert Y's grade inside the transaction");

        let pins = in_pin(&[IDENTITY_X, IDENTITY_Y]);
        let group_by = vec![CLASS_PROPERTY.to_string()];
        let order_by = vec![OrderClause {
            field: "grade".to_string(),
            ascending: false,
        }];
        let request = || DocumentRankedRequest {
            contract: &contract,
            document_type,
            group_by: &group_by,
            select: SelectProjection::avg("grade"),
            having: &[],
            order_by: &order_by,
            where_clauses: &pins,
            limit: Some(4),
            offset: None,
            has_start_at: false,
            prove: false,
        };

        let with_tx = match drive
            .execute_document_ranked_request(request(), Some(&transaction), pv)
            .expect("the transactional branched read serves")
        {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries"),
        };
        assert_eq!(
            with_tx
                .entries
                .iter()
                .map(|e| e.in_key.clone())
                .collect::<Vec<_>>(),
            vec![Some(IDENTITY_Y.to_vec()), Some(IDENTITY_X.to_vec())],
            "under the transaction both branches contribute (Y's 95 outranks X's 80)"
        );

        let without_tx = match drive
            .execute_document_ranked_request(request(), None, pv)
            .expect("the committed branched read serves")
        {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries"),
        };
        assert_eq!(
            without_tx
                .entries
                .iter()
                .map(|e| e.in_key.clone())
                .collect::<Vec<_>>(),
            vec![Some(IDENTITY_X.to_vec())],
            "without the transaction Y's uncommitted branch is authenticated absent"
        );
    }

    /// grovedb's unified `prove_query` proves committed state only, so an
    /// `IN`-pinned prove under a caller transaction fails closed instead of
    /// silently proving a different snapshot than the unproved read serves.
    #[test]
    fn a_branched_prove_rejects_a_transaction() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_grades(&drive, &contract, &[(IDENTITY_X, "math", 80)]);

        let transaction = drive.grove.start_transaction();
        let pins = in_pin(&[IDENTITY_X, IDENTITY_Y]);
        let group_by = vec![CLASS_PROPERTY.to_string()];
        let order_by = vec![OrderClause {
            field: "grade".to_string(),
            ascending: false,
        }];
        let error = drive
            .execute_document_ranked_request(
                DocumentRankedRequest {
                    contract: &contract,
                    document_type: contract
                        .document_type_for_name(DOCUMENT_TYPE)
                        .expect("grade doctype exists"),
                    group_by: &group_by,
                    select: SelectProjection::avg("grade"),
                    having: &[],
                    order_by: &order_by,
                    where_clauses: &pins,
                    limit: Some(2),
                    offset: None,
                    has_start_at: false,
                    prove: true,
                },
                Some(&transaction),
                platform_version(),
            )
            .expect_err("a transactional branched prove must fail closed");
        assert!(
            matches!(
                &error,
                Error::Drive(crate::error::drive::DriveError::NotSupported(message))
                    if message.contains("committed state")
            ),
            "expected the committed-state-only rejection, got {error:?}"
        );

        // The unproved read is unaffected by the failed prove; it serves
        // committed state (the transactional case is pinned by
        // `a_branched_unproved_read_honors_the_transaction`).
        let page = match run(&drive, &contract, &pins, 2, false)
            .expect("the committed unproved read is unaffected")
        {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries"),
        };
        assert_eq!(page.entries.len(), 1);
    }

    /// The public encoder enforces the documented branch ceiling itself:
    /// its callers' grammar checks are not the only line of defense.
    #[test]
    fn the_prefix_encoder_enforces_the_branch_ceiling() {
        let (_drive, contract) = setup_grades_compound_ranked();
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("grade doctype exists");
        let index = contract
            .document_types()
            .get(DOCUMENT_TYPE)
            .expect("grade doctype exists")
            .indexes()
            .values()
            .next()
            .expect("the compound ranked index exists");
        let pins = vec![PrefixPin {
            field: PREFIX_PROPERTY.to_string(),
            values: (0u8..=10).map(|i| Value::Identifier([i; 32])).collect(),
        }];
        let error = encode_prefix_branches(document_type, index, &pins, platform_version())
            .expect_err("11 branches must exceed the ceiling");
        assert!(
            matches!(
                &error,
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(message))
                    if message.contains("more branches")
            ),
            "expected the fan-out ceiling rejection, got {error:?}"
        );
    }

    /// The `IN` union is served from ONE committed state: a `None` read
    /// runs under a grovedb snapshot read transaction internally, and
    /// the same primitive is exercised here explicitly — a branched read
    /// under a snapshot transaction taken before a commit returns the
    /// pre-commit union (the new branch still absent, the changed page
    /// unchanged), while a fresh committed read returns the post-commit
    /// union.
    #[test]
    fn a_branched_read_is_pinned_to_one_committed_state() {
        let (drive, contract) = setup_grades_compound_ranked();
        let pv = platform_version();
        insert_grades(&drive, &contract, &[(IDENTITY_X, "math", 80)]);

        let snapshot_transaction = drive.grove.start_snapshot_read_transaction();

        // A "block commit" lands after the snapshot: Y's branch springs
        // into existence and X gains a class. Documents are built with
        // distinct seeds so they cannot collide with `insert_grades`'.
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("grade doctype exists");
        for (seed, identity, class, grade) in [
            (9100u64, IDENTITY_Y, "science", 95i64),
            (9101u64, IDENTITY_X, "art", 90i64),
        ] {
            let mut doc: Document = document_type
                .random_document(Some(seed), pv)
                .expect("random document");
            let mut props = BTreeMap::new();
            props.insert(PREFIX_PROPERTY.to_string(), Value::Identifier(identity));
            props.insert(CLASS_PROPERTY.to_string(), Value::Text(class.to_string()));
            props.insert("grade".to_string(), Value::I64(grade));
            doc.set_properties(props);
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to commit the post-snapshot grade");
        }

        let pins = in_pin(&[IDENTITY_X, IDENTITY_Y]);
        let group_by = vec![CLASS_PROPERTY.to_string()];
        let order_by = vec![OrderClause {
            field: "grade".to_string(),
            ascending: false,
        }];
        let request = || DocumentRankedRequest {
            contract: &contract,
            document_type,
            group_by: &group_by,
            select: SelectProjection::avg("grade"),
            having: &[],
            order_by: &order_by,
            where_clauses: &pins,
            limit: Some(4),
            offset: None,
            has_start_at: false,
            prove: false,
        };

        // Under the pre-commit snapshot the union is the pre-commit
        // state: Y's branch is still absent (empty, not an error) and X
        // has only math.
        let pinned = match drive
            .execute_document_ranked_request(request(), Some(&snapshot_transaction), pv)
            .expect("the snapshot branched read serves")
        {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries"),
        };
        assert_eq!(
            pinned.entries.len(),
            1,
            "the snapshot union is the pre-commit state"
        );
        assert_eq!(pinned.entries[0].key, b"math".to_vec());

        // A fresh committed read sees the post-commit union.
        let fresh = match run(&drive, &contract, &pins, 4, false)
            .expect("the committed branched read serves")
        {
            DocumentRankedResponse::Entries(page) => page,
            DocumentRankedResponse::Proof(_) => panic!("expected entries"),
        };
        assert_eq!(
            fresh.entries.len(),
            3,
            "the committed union reflects the commit"
        );
    }
}
