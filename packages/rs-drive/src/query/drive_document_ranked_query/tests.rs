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
//! | doctype  | index                | axis  | aggregated property |
//! |----------|----------------------|-------|---------------------|
//! | `review` | `byRestaurant`       | Avg   | `grade`             |
//! | `visit`  | `byRestaurantVisits` | Count | — (`COUNT(*)`)      |
//! | `tip`    | `byRestaurantTips`   | Sum   | `amount`            |

use super::index_picker::find_ranked_index_for_axis;
use super::mode_detection::{detect_ranked_mode, detect_ranked_mode_v0};
use super::*;
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::having::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRanking,
    HavingRankingKind, HavingRightOperand,
};
use crate::query::projection::{SelectFunction, SelectProjection};
use crate::query::{WhereClause, WhereOperator};
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

/// A single `HAVING <aggregate> <op> <ranking>` clause, with the operator
/// that v0's grammar pairs with the ranking kind (`in` for the set-valued
/// `TOP` / `BOTTOM`, `=` for the scalar `MAX` / `MIN`).
fn having(
    function: HavingAggregateFunction,
    field: &str,
    kind: HavingRankingKind,
    n: Option<u64>,
) -> Vec<HavingClause> {
    let operator = match kind {
        HavingRankingKind::Top | HavingRankingKind::Bottom => HavingOperator::In,
        HavingRankingKind::Max | HavingRankingKind::Min => HavingOperator::Equal,
    };
    vec![HavingClause {
        aggregate: HavingAggregate {
            function,
            field: field.to_string(),
        },
        operator,
        right: HavingRightOperand::Ranking(HavingRanking { kind, n }),
    }]
}

/// `SELECT AVG(grade) … HAVING AVG(grade) <ranking>` — the fixture's
/// headline shape, parameterized on the ranking so the mapping tests can
/// sweep all four kinds.
fn detect_avg(kind: HavingRankingKind, n: Option<u64>) -> Result<DocumentRankedMode, Error> {
    detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &having(HavingAggregateFunction::Avg, "grade", kind, n),
        &[],
        RankedPaginationInputs::default(),
    )
}

/// The four ranking kinds map onto `(descending, k)` exactly as the SQL
/// reading demands: `TOP` is "highest first", `BOTTOM` is "lowest first",
/// and the scalar kinds are their `n = 1` special cases.
#[test]
fn ranking_kinds_map_to_direction_and_k() {
    let top = detect_avg(HavingRankingKind::Top, Some(5)).expect("TOP(5) is well-formed");
    assert!(top.descending, "TOP ranks highest-first");
    assert_eq!(top.k, 5);

    let bottom = detect_avg(HavingRankingKind::Bottom, Some(3)).expect("BOTTOM(3) is well-formed");
    assert!(!bottom.descending, "BOTTOM ranks lowest-first");
    assert_eq!(bottom.k, 3);

    let max = detect_avg(HavingRankingKind::Max, None).expect("MAX is well-formed");
    assert!(max.descending);
    assert_eq!(max.k, 1, "MAX is TOP(1)");

    let min = detect_avg(HavingRankingKind::Min, None).expect("MIN is well-formed");
    assert!(!min.descending);
    assert_eq!(min.k, 1, "MIN is BOTTOM(1)");
}

/// The resolved mode carries everything the index picker needs, not just
/// the ranking: which axis, which property to group on, and which field
/// the aggregate applies to.
#[test]
fn resolved_mode_carries_axis_group_property_and_field() {
    let avg = detect_avg(HavingRankingKind::Top, Some(2)).expect("well-formed");
    assert_eq!(avg.axis, RankedAxis::Avg);
    assert_eq!(avg.group_by_property, GROUP_PROPERTY);
    assert_eq!(avg.aggregate_field, "grade");

    let count = detect_ranked_mode_v0(
        &SelectProjection::count_star(),
        &group_by(),
        &having(
            HavingAggregateFunction::Count,
            "",
            HavingRankingKind::Top,
            Some(2),
        ),
        &[],
        RankedPaginationInputs::default(),
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
        &having(
            HavingAggregateFunction::Sum,
            "amount",
            HavingRankingKind::Bottom,
            Some(1),
        ),
        &[],
        RankedPaginationInputs::default(),
    )
    .expect("SUM is well-formed");
    assert_eq!(sum.axis, RankedAxis::Sum);
    assert_eq!(sum.aggregate_field, "amount");
}

/// `n = 0` selects nothing and `n > MAX_RANKED_LIMIT` is refused rather
/// than clamped — a clamp would produce a proof whose echoed `k` the
/// client's own reconstruction rejects. The boundary itself is accepted.
#[test]
fn k_is_bounded_to_one_through_max_ranked_limit() {
    let zero = detect_avg(HavingRankingKind::Top, Some(0)).expect_err("TOP(0) must be rejected");
    assert!(matches!(
        zero,
        Error::Query(QuerySyntaxError::InvalidLimit(_))
    ));

    let over = detect_avg(HavingRankingKind::Top, Some(MAX_RANKED_LIMIT as u64 + 1))
        .expect_err("TOP(101) must be rejected");
    assert!(matches!(
        over,
        Error::Query(QuerySyntaxError::InvalidLimit(_))
    ));

    let at_limit = detect_avg(HavingRankingKind::Top, Some(MAX_RANKED_LIMIT as u64))
        .expect("TOP(100) sits exactly on the ceiling and must be accepted");
    assert_eq!(at_limit.k, MAX_RANKED_LIMIT);
}

/// `TOP` / `BOTTOM` need an `n`; `MAX` / `MIN` must not carry one. The
/// wire permits `n` on the scalar kinds for forward compatibility, so
/// evaluation is where it gets rejected (see `HavingRanking::n`).
#[test]
fn n_presence_must_match_the_ranking_kind() {
    assert!(
        detect_avg(HavingRankingKind::Top, None).is_err(),
        "TOP without n is malformed"
    );
    assert!(
        detect_avg(HavingRankingKind::Max, Some(3)).is_err(),
        "MAX with n is malformed"
    );
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
            &having(
                HavingAggregateFunction::Avg,
                "grade",
                HavingRankingKind::Top,
                Some(2),
            ),
            &[],
            RankedPaginationInputs::default(),
        )
        .expect_err("group_by arity other than 1 must be rejected");
        assert!(matches!(
            error,
            Error::Query(QuerySyntaxError::InvalidParameter(_))
        ));
    }
}

/// The `HAVING` aggregate must be the *same* aggregate as the `SELECT`:
/// ranking one aggregate while projecting another would need a second
/// axis the storage does not maintain.
#[test]
fn having_aggregate_must_match_the_select() {
    // Same field, different function.
    let function_mismatch = detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &having(
            HavingAggregateFunction::Sum,
            "grade",
            HavingRankingKind::Top,
            Some(2),
        ),
        &[],
        RankedPaginationInputs::default(),
    )
    .expect_err("AVG select with SUM having must be rejected");
    assert!(matches!(
        function_mismatch,
        Error::Query(QuerySyntaxError::InvalidParameter(_))
    ));

    // Same function, different field.
    let field_mismatch = detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &having(
            HavingAggregateFunction::Avg,
            "price",
            HavingRankingKind::Top,
            Some(2),
        ),
        &[],
        RankedPaginationInputs::default(),
    )
    .expect_err("AVG(grade) select with AVG(price) having must be rejected");
    assert!(matches!(
        field_mismatch,
        Error::Query(QuerySyntaxError::InvalidParameter(_))
    ));
}

/// Exactly one `HAVING` clause. Multiple clauses are implicitly ANDed and
/// there is no ranked primitive for the intersection of two rankings.
#[test]
fn exactly_one_having_clause_is_required() {
    let mut two = having(
        HavingAggregateFunction::Avg,
        "grade",
        HavingRankingKind::Top,
        Some(2),
    );
    two.extend(having(
        HavingAggregateFunction::Avg,
        "grade",
        HavingRankingKind::Bottom,
        Some(2),
    ));
    for clauses in [Vec::new(), two] {
        let error = detect_ranked_mode_v0(
            &SelectProjection::avg("grade"),
            &group_by(),
            &clauses,
            &[],
            RankedPaginationInputs::default(),
        )
        .expect_err("having must carry exactly one clause");
        assert!(matches!(
            error,
            Error::Query(QuerySyntaxError::InvalidParameter(_))
        ));
    }
}

/// A threshold right-operand (`HAVING AVG(grade) > 80`) is a range walk
/// over the axis secondary — a different primitive, deliberately deferred.
#[test]
fn having_value_operand_is_rejected_as_unsupported() {
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
        &[],
        RankedPaginationInputs::default(),
    )
    .expect_err("value right-operands are not implemented");
    assert!(matches!(
        error,
        Error::Query(QuerySyntaxError::Unsupported(_))
    ));
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
            &having(
                HavingAggregateFunction::Count,
                "",
                HavingRankingKind::Top,
                Some(2),
            ),
            &[],
            RankedPaginationInputs::default(),
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
            &having(
                HavingAggregateFunction::Sum,
                "",
                HavingRankingKind::Top,
                Some(2),
            ),
            &[],
            RankedPaginationInputs::default(),
        )
        .expect_err("a fieldless SUM/AVG has nothing to aggregate");
        assert!(matches!(
            error,
            Error::Query(QuerySyntaxError::InvalidParameter(_))
        ));
    }
}

/// A `where` clause is refused, not ignored. The axis secondary is
/// ordered by aggregate, not by group key, so it cannot rank a filtered
/// subset — silently dropping the filter would answer the unfiltered
/// question under the guise of the filtered one.
#[test]
fn where_clauses_are_rejected() {
    let where_clauses = vec![WhereClause {
        field: GROUP_PROPERTY.to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text("alpha".to_string()),
    }];
    let error = detect_ranked_mode_v0(
        &SelectProjection::avg("grade"),
        &group_by(),
        &having(
            HavingAggregateFunction::Avg,
            "grade",
            HavingRankingKind::Top,
            Some(2),
        ),
        &where_clauses,
        RankedPaginationInputs::default(),
    )
    .expect_err("ranked queries take no where clauses");
    assert!(matches!(
        error,
        Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(_))
    ));
}

/// `limit` / `offset` / `start_at` all conflict with, or are meaningless
/// against, an aggregate-ordered walk whose size is the ranking's `n`.
#[test]
fn pagination_inputs_are_rejected() {
    for pagination in [
        RankedPaginationInputs {
            limit: Some(10),
            ..Default::default()
        },
        RankedPaginationInputs {
            offset: Some(5),
            ..Default::default()
        },
        RankedPaginationInputs {
            has_start_at: true,
            ..Default::default()
        },
    ] {
        let error = detect_ranked_mode_v0(
            &SelectProjection::avg("grade"),
            &group_by(),
            &having(
                HavingAggregateFunction::Avg,
                "grade",
                HavingRankingKind::Top,
                Some(2),
            ),
            &[],
            pagination,
        )
        .expect_err("ranked queries take no pagination inputs");
        assert!(
            matches!(error, Error::Query(QuerySyntaxError::InvalidLimit(_))),
            "expected InvalidLimit for {pagination:?}, got {error}"
        );
    }
}

/// The versioned wrapper routes v0 to the v0 table and fails closed on an
/// unknown slot value rather than silently falling back.
#[test]
fn versioned_detection_routes_v0_and_rejects_unknown_versions() {
    let versioned = detect_ranked_mode(
        &SelectProjection::avg("grade"),
        &group_by(),
        &having(
            HavingAggregateFunction::Avg,
            "grade",
            HavingRankingKind::Top,
            Some(4),
        ),
        &[],
        RankedPaginationInputs::default(),
        platform_version(),
    )
    .expect("PV14's detect_ranked_mode slot is 0");
    assert_eq!(
        versioned,
        detect_avg(HavingRankingKind::Top, Some(4)).unwrap()
    );

    let mut future = platform_version().clone();
    future.drive.methods.document.query.detect_ranked_mode = 1;
    let error = detect_ranked_mode(
        &SelectProjection::avg("grade"),
        &group_by(),
        &having(
            HavingAggregateFunction::Avg,
            "grade",
            HavingRankingKind::Top,
            Some(4),
        ),
        &[],
        RankedPaginationInputs::default(),
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
        find_ranked_index_for_axis(&indexes, GROUP_PROPERTY, RankedAxis::Count, "").is_some(),
        "the declared axis resolves"
    );
    for (axis, field) in [(RankedAxis::Sum, "grade"), (RankedAxis::Avg, "grade")] {
        assert!(
            find_ranked_index_for_axis(&indexes, GROUP_PROPERTY, axis, field).is_none(),
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
            find_ranked_index_for_axis(&indexes, GROUP_PROPERTY, axis, "grade").is_some(),
            "{axis:?} on the indexed summable resolves"
        );
        assert!(
            find_ranked_index_for_axis(&indexes, GROUP_PROPERTY, axis, "tipAmount").is_none(),
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
        find_ranked_index_for_axis(&indexes, "chefId", RankedAxis::Avg, "grade").is_none(),
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
        find_ranked_index_for_axis(&indexes, GROUP_PROPERTY, RankedAxis::Avg, "price").is_none()
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
struct RankedCase {
    document_type_name: &'static str,
    /// The single `GROUP BY` property. Every fixture doctype groups by
    /// [`GROUP_PROPERTY`]; the multi-index contract below is the one that
    /// varies it, which is what makes the index picker's choice
    /// observable.
    group_by_property: &'static str,
    select: SelectProjection,
    aggregate_function: HavingAggregateFunction,
    aggregate_field: &'static str,
    kind: HavingRankingKind,
    n: Option<u64>,
}

impl RankedCase {
    fn avg(kind: HavingRankingKind, n: Option<u64>) -> Self {
        Self {
            document_type_name: "review",
            group_by_property: GROUP_PROPERTY,
            select: SelectProjection::avg("grade"),
            aggregate_function: HavingAggregateFunction::Avg,
            aggregate_field: "grade",
            kind,
            n,
        }
    }

    fn count(kind: HavingRankingKind, n: Option<u64>) -> Self {
        Self {
            document_type_name: "visit",
            group_by_property: GROUP_PROPERTY,
            select: SelectProjection::count_star(),
            aggregate_function: HavingAggregateFunction::Count,
            aggregate_field: "",
            kind,
            n,
        }
    }

    fn sum(kind: HavingRankingKind, n: Option<u64>) -> Self {
        Self {
            document_type_name: "tip",
            group_by_property: GROUP_PROPERTY,
            select: SelectProjection::sum("amount"),
            aggregate_function: HavingAggregateFunction::Sum,
            aggregate_field: "amount",
            kind,
            n,
        }
    }

    fn group_by(&self) -> Vec<String> {
        vec![self.group_by_property.to_string()]
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
    let having_clauses = having(
        case.aggregate_function,
        case.aggregate_field,
        case.kind,
        case.n,
    );
    let document_type = contract
        .document_type_for_name(case.document_type_name)
        .expect("doctype exists");
    drive.execute_document_ranked_request(
        DocumentRankedRequest {
            contract,
            document_type,
            group_by: &group_by,
            select: case.select.clone(),
            having: &having_clauses,
            where_clauses: &[],
            limit: None,
            offset: None,
            has_start_at: false,
            prove,
        },
        None,
        platform_version(),
    )
}

fn entries_of(response: DocumentRankedResponse) -> Vec<RankedEntry> {
    match response {
        DocumentRankedResponse::Entries(entries) => entries,
        DocumentRankedResponse::Proof(_) => panic!("expected entries, got a proof"),
    }
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
    let having_clauses = having(
        case.aggregate_function,
        case.aggregate_field,
        case.kind,
        case.n,
    );
    let mode = detect_ranked_mode(
        &case.select,
        &group_by,
        &having_clauses,
        &[],
        RankedPaginationInputs::default(),
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
    let index = find_ranked_index_for_axis(
        indexes,
        &mode.group_by_property,
        mode.axis,
        &mode.aggregate_field,
    )
    .expect("the fixture declares the axis");
    DriveDocumentRankedQuery {
        document_type: contract
            .document_type_for_name(case.document_type_name)
            .expect("doctype exists"),
        contract_id: contract.id_ref().to_buffer(),
        document_type_name: case.document_type_name.to_string(),
        index,
        axis: mode.axis,
        descending: mode.descending,
        k: mode.k,
    }
}

fn grovedb_root_hash(drive: &Drive) -> [u8; 32] {
    drive
        .grove
        .root_hash(None, &platform_version().drive.grove_version)
        .unwrap()
        .expect("root hash must be readable")
}

/// Prove the case, verify the proof, and assert the verified entries and
/// root hash match the live database.
fn assert_proof_round_trips(
    drive: &Drive,
    contract: &DataContract,
    case: &RankedCase,
    expected: &[RankedEntry],
) {
    let proof = proof_of(run(drive, contract, case, true).expect("prove must succeed"));
    let query = client_side_query(contract, case);
    let (root_hash, verified) = query
        .verify_ranked_top_k_proof(&proof, platform_version())
        .expect("the proof must verify");
    assert_eq!(
        verified, expected,
        "verified entries must equal what the unproven read returned"
    );
    assert_eq!(
        root_hash,
        grovedb_root_hash(drive),
        "the proof must reconstruct the live grovedb root hash"
    );
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

    let top_three = RankedCase::avg(HavingRankingKind::Top, Some(3));
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
    let bottom_one = RankedCase::avg(HavingRankingKind::Bottom, Some(1));
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

    let top_two = RankedCase::count(HavingRankingKind::Top, Some(2));
    let entries = entries_of(run(&drive, &contract, &top_two, false).expect("read must succeed"));
    assert_eq!(
        keys_of(&entries),
        vec!["delta", "beta"],
        "descending by document count: delta(4) > beta(3) > gamma(2) > alpha(1)"
    );
    assert_eq!(entries[0].value, RankedEntryValue::Count(4));
    assert_eq!(entries[1].value, RankedEntryValue::Count(3));
    assert_proof_round_trips(&drive, &contract, &top_two, &entries);

    let min = RankedCase::count(HavingRankingKind::Min, None);
    let entries = entries_of(run(&drive, &contract, &min, false).expect("read must succeed"));
    assert_eq!(keys_of(&entries), vec!["alpha"], "MIN is BOTTOM(1)");
    assert_eq!(entries[0].value, RankedEntryValue::Count(1));
    assert_proof_round_trips(&drive, &contract, &min, &entries);
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

    let max = RankedCase::sum(HavingRankingKind::Max, None);
    let entries = entries_of(run(&drive, &contract, &max, false).expect("read must succeed"));
    assert_eq!(keys_of(&entries), vec!["beta"], "MAX is TOP(1)");
    assert_eq!(entries[0].value, RankedEntryValue::Sum(100));
    assert_proof_round_trips(&drive, &contract, &max, &entries);

    let bottom_three = RankedCase::sum(HavingRankingKind::Bottom, Some(3));
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

    let case = RankedCase::sum(HavingRankingKind::Top, Some(MAX_RANKED_LIMIT as u64));
    let entries = entries_of(run(&drive, &contract, &case, false).expect("read must succeed"));
    assert_eq!(keys_of(&entries), vec!["beta", "alpha"]);
    assert_proof_round_trips(&drive, &contract, &case, &entries);
}

/// **A ranked index with no documents behaves asymmetrically between the
/// two paths, and that is a grovedb limitation, not a drive choice.**
///
/// The unproven read returns an empty list — the indexed tree exists from
/// contract registration, its secondary is simply empty. The prove path
/// *errors*: grovedb's merk prover cannot emit a proof for an empty tree
/// ("Cannot create proof for empty tree"), and there is no absence-proof
/// shape for "this ranking has no entries".
///
/// Pinned as a test rather than left undiscovered because it is reachable
/// by any client: a freshly registered contract with `prove = true`
/// returns an error until the first document lands. Callers that must
/// tolerate empty state should read unproven, or treat this specific
/// error as "empty". Fixing it properly means teaching grovedb to emit an
/// empty-tree envelope; when that lands, this test flips to a successful
/// round trip and is the tripwire that says so.
#[test]
fn ranking_an_empty_index_reads_empty_but_cannot_be_proved() {
    let (drive, contract) = setup_restaurants();
    let case = RankedCase::avg(HavingRankingKind::Top, Some(5));

    let entries = entries_of(run(&drive, &contract, &case, false).expect("read must succeed"));
    assert!(
        entries.is_empty(),
        "an index with no documents has no groups to rank"
    );

    let error = run(&drive, &contract, &case, true)
        .expect_err("grovedb cannot prove an empty secondary today");
    let message = error.to_string();
    assert!(
        message.contains("empty tree"),
        "expected the empty-tree prover limitation, got: {message}"
    );

    // One document is enough to make the same request provable.
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
        run(
            &drive,
            &contract,
            &RankedCase::sum(HavingRankingKind::Top, Some(4)),
            false,
        )
        .expect("read must succeed"),
    );
    assert_eq!(
        keys_of(&descending),
        vec!["gamma", "delta", "beta", "alpha"],
        "descending walk breaks ties in DESCENDING group-key order"
    );

    let ascending = entries_of(
        run(
            &drive,
            &contract,
            &RankedCase::sum(HavingRankingKind::Bottom, Some(4)),
            false,
        )
        .expect("read must succeed"),
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
    // arbitrary one — which is what makes TOP(k) reproducible.
    let top_two = entries_of(
        run(
            &drive,
            &contract,
            &RankedCase::sum(HavingRankingKind::Top, Some(2)),
            false,
        )
        .expect("read must succeed"),
    );
    assert_eq!(keys_of(&top_two), vec!["gamma", "delta"]);
    assert_proof_round_trips(
        &drive,
        &contract,
        &RankedCase::sum(HavingRankingKind::Top, Some(2)),
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

    let case = RankedCase::avg(HavingRankingKind::Top, Some(2));
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

    let case = RankedCase::avg(HavingRankingKind::Top, Some(2));
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

    let case = RankedCase::avg(HavingRankingKind::Max, None);
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
        aggregate_function: HavingAggregateFunction::Sum,
        aggregate_field: "guests",
        kind: HavingRankingKind::Top,
        n: Some(2),
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
/// The compound sibling deliberately hangs off `byChef` (count-only)
/// rather than off `byRestaurant` (count+sum): a continuation tree under
/// a **sum-bearing** value tree has to be wrapped `NotCountedOrSummed`,
/// and that wrapper accepts only sum-bearing inners, so a plain
/// `NormalTree` continuation is refused outright. That refusal predates
/// this feature — it reproduces with the `ranked*` keywords removed — and
/// it means *any* summable index terminating at `[a]` is currently
/// incompatible with a compound index `[a, …]` on the same doctype.
/// Reusing that shape here would test the pre-existing gap, not the
/// picker.
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
fn dish_avg_case(kind: HavingRankingKind, n: Option<u64>) -> RankedCase {
    RankedCase {
        document_type_name: "dish",
        group_by_property: GROUP_PROPERTY,
        select: SelectProjection::avg("grade"),
        aggregate_function: HavingAggregateFunction::Avg,
        aggregate_field: "grade",
        kind,
        n,
    }
}

/// `SELECT COUNT(*) … GROUP BY chefId` against the multi-index doctype.
fn dish_count_case(kind: HavingRankingKind, n: Option<u64>) -> RankedCase {
    RankedCase {
        document_type_name: "dish",
        group_by_property: CHEF_PROPERTY,
        select: SelectProjection::count_star(),
        aggregate_function: HavingAggregateFunction::Count,
        aggregate_field: "",
        kind,
        n,
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
    let by_restaurant = dish_avg_case(HavingRankingKind::Top, Some(3));
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
    let by_chef = dish_count_case(HavingRankingKind::Top, Some(3));
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
        find_ranked_index_for_axis(indexes, GROUP_PROPERTY, RankedAxis::Avg, "grade")
            .expect("the Avg ranking resolves")
            .name,
        "byRestaurant",
    );
    assert_eq!(
        find_ranked_index_for_axis(indexes, CHEF_PROPERTY, RankedAxis::Count, "")
            .expect("the Count ranking resolves")
            .name,
        "byChef",
    );

    assert_proof_round_trips(&drive, &contract, &by_restaurant, &avg_entries);
    assert_proof_round_trips(&drive, &contract, &by_chef, &count_entries);

    // The non-ranked compound index is never a candidate even though it
    // leads with `chefId`, which is exactly the property this request
    // groups by. `byChef` declares only the Count axis, so an Avg
    // ranking over chefs has no index — and `byChefRestaurant` must not
    // be press-ganged into serving it.
    assert!(
        find_ranked_index_for_axis(indexes, CHEF_PROPERTY, RankedAxis::Avg, "grade").is_none(),
        "`byChefRestaurant` leads with chefId but is compound and unranked"
    );
    let avg_by_chef = RankedCase {
        group_by_property: CHEF_PROPERTY,
        ..dish_avg_case(HavingRankingKind::Top, Some(2))
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
