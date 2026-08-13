//! Unit + integration tests for the having-range query surface.
//!
//! Mirrors the structure of
//! [`super::super::drive_document_ranked_query::tests`]: grammar and
//! bounds tests are pure (no Drive), execution tests run against a real
//! Drive with the shared `restaurants` fixture (see that module's docs
//! for the doctype → axis table) and documents inserted through the
//! real write path, with every proof round-tripped through
//! [`DriveDocumentHavingQuery::verify_having_range_proof`] and checked
//! against the live grovedb root hash.

use super::mode_detection::detect_having_mode_v0;
use super::{AxisRangeBounds, MAX_HAVING_LIMIT};
use crate::query::drive_document_ranked_query::RankedPaginationInputs;
use crate::query::having::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
};
use crate::query::projection::SelectProjection;
use crate::query::OrderClause;
use dpp::platform_value::Value;

fn clause(
    function: HavingAggregateFunction,
    field: &str,
    operator: HavingOperator,
    right: Value,
) -> HavingClause {
    HavingClause {
        aggregate: HavingAggregate {
            function,
            field: field.to_string(),
        },
        operator,
        right: HavingRightOperand::Value(right),
    }
}

fn pagination(limit: u32) -> RankedPaginationInputs {
    RankedPaginationInputs {
        limit: Some(limit),
        offset: None,
        has_start_at: false,
    }
}

mod grammar {
    use super::*;

    #[test]
    fn count_greater_than_resolves_to_exclusive_lower_bound() {
        let mode = detect_having_mode_v0(
            &SelectProjection::count_star(),
            &["hashtag".to_string()],
            &[clause(
                HavingAggregateFunction::Count,
                "",
                HavingOperator::GreaterThan,
                Value::U64(100),
            )],
            &[],
            &[],
            pagination(10),
        )
        .expect("should resolve");
        assert_eq!(
            mode.bounds,
            AxisRangeBounds::Count {
                lo: 101,
                hi: u64::MAX
            }
        );
        assert!(!mode.descending);
        assert_eq!(mode.limit, 10);
        assert_eq!(mode.group_by_property, "hashtag");
        assert_eq!(mode.aggregate_field, "");
    }

    #[test]
    fn sum_between_is_inclusive_on_both_ends() {
        let mode = detect_having_mode_v0(
            &SelectProjection::sum("amount"),
            &["donorId".to_string()],
            &[clause(
                HavingAggregateFunction::Sum,
                "amount",
                HavingOperator::Between,
                Value::Array(vec![Value::I64(1000), Value::I64(5000)]),
            )],
            &[],
            &[],
            pagination(100),
        )
        .expect("should resolve");
        assert_eq!(mode.bounds, AxisRangeBounds::Sum { lo: 1000, hi: 5000 });
    }

    #[test]
    fn between_exclude_bounds_moves_both_ends_inward() {
        let mode = detect_having_mode_v0(
            &SelectProjection::count_star(),
            &["hashtag".to_string()],
            &[clause(
                HavingAggregateFunction::Count,
                "",
                HavingOperator::BetweenExcludeBounds,
                Value::Array(vec![Value::U64(5), Value::U64(10)]),
            )],
            &[],
            &[],
            pagination(10),
        )
        .expect("should resolve");
        assert_eq!(mode.bounds, AxisRangeBounds::Count { lo: 6, hi: 9 });
    }

    #[test]
    fn avg_integer_threshold_scales_exactly_into_fixed_point() {
        use crate::query::drive_document_ranked_query::RANKED_AVG_SCALE;
        let mode = detect_having_mode_v0(
            &SelectProjection::avg("grade"),
            &["restaurantId".to_string()],
            &[clause(
                HavingAggregateFunction::Avg,
                "grade",
                HavingOperator::GreaterThanOrEquals,
                Value::U64(4),
            )],
            &[],
            &[],
            pagination(50),
        )
        .expect("should resolve");
        assert_eq!(
            mode.bounds,
            AxisRangeBounds::Avg {
                lo: 4 * RANKED_AVG_SCALE,
                hi: i128::MAX
            }
        );
    }

    /// Resolve one `AVG(grade)` clause and return the bounds.
    fn avg_bounds(
        operator: HavingOperator,
        right: Value,
    ) -> Result<AxisRangeBounds, crate::error::Error> {
        detect_having_mode_v0(
            &SelectProjection::avg("grade"),
            &["restaurantId".to_string()],
            &[clause(
                HavingAggregateFunction::Avg,
                "grade",
                operator,
                right,
            )],
            &[],
            &[],
            pagination(50),
        )
        .map(|mode| mode.bounds)
    }

    /// Float thresholds translate through the **exact** IEEE-754 value
    /// with operator-aware floor/ceiling — never through truncation.
    /// `80.5` is exactly representable (`161 × 2⁻¹`), so its scaled
    /// product lands on a tick and the inclusive/exclusive translations
    /// differ by exactly one, on both ends.
    #[test]
    fn avg_float_threshold_on_a_tick_translates_like_an_integer() {
        use crate::query::drive_document_ranked_query::RANKED_AVG_SCALE;
        let tick = 161 * RANKED_AVG_SCALE / 2; // 80.5 × SCALE, exact
        let max = i128::MAX;
        let min = i128::MIN;
        for (operator, expected_lo, expected_hi) in [
            (HavingOperator::GreaterThanOrEquals, tick, max),
            (HavingOperator::GreaterThan, tick + 1, max),
            (HavingOperator::LessThanOrEquals, min, tick),
            (HavingOperator::LessThan, min, tick - 1),
            (HavingOperator::Equal, tick, tick),
        ] {
            assert_eq!(
                avg_bounds(operator, Value::Float(80.5)).expect("80.5 scales exactly"),
                AxisRangeBounds::Avg {
                    lo: expected_lo,
                    hi: expected_hi
                },
                "wrong translation for {operator:?} 80.5"
            );
        }
    }

    /// A float threshold that falls **between** two ticks: the
    /// inclusive and exclusive translations collapse onto the same
    /// integer bound — the ceiling for lower bounds, the floor for
    /// upper bounds. `5e-20` scales to ≈0.5 of a tick, the exact case
    /// truncation used to get wrong (`AVG >= 0.5-tick` must start at
    /// tick 1, not 0), and its negation exercises the
    /// negative-threshold direction (`AVG > -0.5-tick` must start at
    /// tick 0, not 1 — truncate-then-increment lands on 1).
    #[test]
    fn avg_float_threshold_between_ticks_takes_operator_aware_bounds() {
        let half_tick = Value::Float(5e-20); // ≈ 0.5 of a fixed-point tick
        let neg_half_tick = Value::Float(-5e-20);
        let max = i128::MAX;
        let min = i128::MIN;
        for (operator, right, expected_lo, expected_hi) in [
            (
                HavingOperator::GreaterThanOrEquals,
                half_tick.clone(),
                1,
                max,
            ),
            (HavingOperator::GreaterThan, half_tick.clone(), 1, max),
            (HavingOperator::LessThanOrEquals, half_tick.clone(), min, 0),
            (HavingOperator::LessThan, half_tick.clone(), min, 0),
            (HavingOperator::GreaterThan, neg_half_tick.clone(), 0, max),
            (
                HavingOperator::GreaterThanOrEquals,
                neg_half_tick.clone(),
                0,
                max,
            ),
            (HavingOperator::LessThan, neg_half_tick.clone(), min, -1),
            (
                HavingOperator::LessThanOrEquals,
                neg_half_tick.clone(),
                min,
                -1,
            ),
        ] {
            assert_eq!(
                avg_bounds(operator, right.clone()).expect("between-tick thresholds resolve"),
                AxisRangeBounds::Avg {
                    lo: expected_lo,
                    hi: expected_hi
                },
                "wrong translation for {operator:?} {right:?}"
            );
        }

        // An equality on a value between ticks can never match a
        // group's average; it is rejected loudly rather than silently
        // converted into a point lookup on the truncated tick.
        let error = avg_bounds(HavingOperator::Equal, half_tick)
            .expect_err("equality between ticks matches nothing");
        assert!(
            format!("{error}").contains("does not land on a fixed-point tick"),
            "the rejection must explain the tick mismatch, got: {error}"
        );
    }

    #[test]
    fn order_by_the_selected_aggregate_sets_direction() {
        let mode = detect_having_mode_v0(
            &SelectProjection::count_star(),
            &["hashtag".to_string()],
            &[clause(
                HavingAggregateFunction::Count,
                "",
                HavingOperator::GreaterThan,
                Value::U64(100),
            )],
            &[OrderClause {
                field: "$count".to_string(),
                ascending: false,
            }],
            &[],
            pagination(10),
        )
        .expect("should resolve");
        assert!(mode.descending);
    }

    #[test]
    fn ordering_by_anything_else_is_rejected() {
        let result = detect_having_mode_v0(
            &SelectProjection::count_star(),
            &["hashtag".to_string()],
            &[clause(
                HavingAggregateFunction::Count,
                "",
                HavingOperator::GreaterThan,
                Value::U64(100),
            )],
            &[OrderClause {
                field: "hashtag".to_string(),
                ascending: true,
            }],
            &[],
            pagination(10),
        );
        assert!(result.is_err(), "ordering by a schema property must fail");
    }

    #[test]
    fn clause_on_a_different_aggregate_than_the_select_is_rejected() {
        let result = detect_having_mode_v0(
            &SelectProjection::count_star(),
            &["hashtag".to_string()],
            &[clause(
                HavingAggregateFunction::Sum,
                "amount",
                HavingOperator::GreaterThan,
                Value::I64(100),
            )],
            &[],
            &[],
            pagination(10),
        );
        assert!(result.is_err(), "cross-aggregate having must fail");
    }

    /// `GROUP BY identityId, class HAVING AVG(grade) > 80` — an
    /// **unpinned** two-field grouping — is still rejected: a compound
    /// ranked index bounds each prefix's groups separately, so the
    /// served form pins the leading property with an equality `where`
    /// and groups over the trailing one (`WHERE identityId = X GROUP BY
    /// class …` — exercised end to end in the `pinned_prefix` suite).
    /// The rejection must steer the caller to that form.
    #[test]
    fn compound_group_by_is_rejected() {
        let result = detect_having_mode_v0(
            &SelectProjection::avg("grade"),
            &["identityId".to_string(), "class".to_string()],
            &[clause(
                HavingAggregateFunction::Avg,
                "grade",
                HavingOperator::GreaterThan,
                Value::U64(80),
            )],
            &[],
            &[],
            pagination(10),
        );
        let error = result.expect_err("unpinned compound group_by must fail");
        let message = format!("{error}");
        assert!(
            message.contains("exactly one `group_by` property")
                && message.contains("equality `where` clause"),
            "the rejection must steer to the pinned-prefix form, got: {error}"
        );
    }

    #[test]
    fn multiple_clauses_are_rejected() {
        let single = clause(
            HavingAggregateFunction::Count,
            "",
            HavingOperator::GreaterThan,
            Value::U64(100),
        );
        let result = detect_having_mode_v0(
            &SelectProjection::count_star(),
            &["hashtag".to_string()],
            &[single.clone(), single],
            &[],
            &[],
            pagination(10),
        );
        assert!(result.is_err(), "multi-clause having must fail");
    }

    #[test]
    fn not_equal_and_in_are_rejected_as_non_contiguous() {
        for operator in [HavingOperator::NotEqual, HavingOperator::In] {
            let result = detect_having_mode_v0(
                &SelectProjection::count_star(),
                &["hashtag".to_string()],
                &[clause(
                    HavingAggregateFunction::Count,
                    "",
                    operator,
                    Value::U64(100),
                )],
                &[],
                &[],
                pagination(10),
            );
            assert!(result.is_err(), "{operator:?} must fail");
        }
    }

    #[test]
    fn greater_than_the_type_maximum_is_rejected_not_served_empty() {
        let result = detect_having_mode_v0(
            &SelectProjection::count_star(),
            &["hashtag".to_string()],
            &[clause(
                HavingAggregateFunction::Count,
                "",
                HavingOperator::GreaterThan,
                Value::U64(u64::MAX),
            )],
            &[],
            &[],
            pagination(10),
        );
        assert!(result.is_err(), "> u64::MAX must fail loudly");
    }

    #[test]
    fn inverted_between_is_rejected() {
        let result = detect_having_mode_v0(
            &SelectProjection::count_star(),
            &["hashtag".to_string()],
            &[clause(
                HavingAggregateFunction::Count,
                "",
                HavingOperator::Between,
                Value::Array(vec![Value::U64(10), Value::U64(5)]),
            )],
            &[],
            &[],
            pagination(10),
        );
        assert!(result.is_err(), "inverted bounds must fail loudly");
    }

    #[test]
    fn negative_count_bound_is_rejected() {
        let result = detect_having_mode_v0(
            &SelectProjection::count_star(),
            &["hashtag".to_string()],
            &[clause(
                HavingAggregateFunction::Count,
                "",
                HavingOperator::GreaterThan,
                Value::I64(-1),
            )],
            &[],
            &[],
            pagination(10),
        );
        assert!(result.is_err(), "a negative COUNT bound must fail");
    }

    #[test]
    fn limit_is_required_and_capped() {
        let having = [clause(
            HavingAggregateFunction::Count,
            "",
            HavingOperator::GreaterThan,
            Value::U64(100),
        )];
        let select = SelectProjection::count_star();
        let group_by = ["hashtag".to_string()];

        let missing = detect_having_mode_v0(
            &select,
            &group_by,
            &having,
            &[],
            &[],
            RankedPaginationInputs::default(),
        );
        assert!(missing.is_err(), "a missing limit must fail");

        let over = detect_having_mode_v0(
            &select,
            &group_by,
            &having,
            &[],
            &[],
            pagination(MAX_HAVING_LIMIT as u32 + 1),
        );
        assert!(over.is_err(), "an over-ceiling limit must fail");
    }

    #[test]
    fn offset_and_start_at_are_rejected() {
        let having = [clause(
            HavingAggregateFunction::Count,
            "",
            HavingOperator::GreaterThan,
            Value::U64(100),
        )];
        let select = SelectProjection::count_star();
        let group_by = ["hashtag".to_string()];

        let with_offset = detect_having_mode_v0(
            &select,
            &group_by,
            &having,
            &[],
            &[],
            RankedPaginationInputs {
                limit: Some(10),
                offset: Some(0),
                has_start_at: false,
            },
        );
        assert!(with_offset.is_err(), "any offset (even 0) must fail");

        let with_start = detect_having_mode_v0(
            &select,
            &group_by,
            &having,
            &[],
            &[],
            RankedPaginationInputs {
                limit: Some(10),
                offset: None,
                has_start_at: true,
            },
        );
        assert!(with_start.is_err(), "start_at must fail");
    }
}

mod bounds {
    use super::*;
    use grovedb::element::indexed::{
        encode_avg_sort_key, encode_count_sort_key, encode_sum_sort_key,
    };

    #[test]
    fn count_byte_bounds_bracket_the_inclusive_range() {
        let bounds = AxisRangeBounds::Count { lo: 101, hi: 200 };
        let (lower, upper) = bounds.secondary_key_bounds();
        assert_eq!(lower, encode_count_sort_key(101).to_vec());
        assert_eq!(upper, Some(encode_count_sort_key(201).to_vec()));
    }

    #[test]
    fn unbounded_above_uses_range_from() {
        let bounds = AxisRangeBounds::Count {
            lo: 101,
            hi: u64::MAX,
        };
        let (_, upper) = bounds.secondary_key_bounds();
        assert_eq!(upper, None, "hi == MAX has no representable successor");

        let sum_bounds = AxisRangeBounds::Sum {
            lo: 0,
            hi: i64::MAX,
        };
        assert_eq!(sum_bounds.secondary_key_bounds().1, None);

        let avg_bounds = AxisRangeBounds::Avg {
            lo: 0,
            hi: i128::MAX,
        };
        assert_eq!(avg_bounds.secondary_key_bounds().1, None);
    }

    #[test]
    fn sum_and_avg_bounds_use_the_sign_flipped_encodings() {
        let sum_bounds = AxisRangeBounds::Sum { lo: -5, hi: 5 };
        let (lower, upper) = sum_bounds.secondary_key_bounds();
        assert_eq!(lower, encode_sum_sort_key(-5).to_vec());
        assert_eq!(upper, Some(encode_sum_sort_key(6).to_vec()));

        let avg_bounds = AxisRangeBounds::Avg { lo: -5, hi: 5 };
        let (lower, upper) = avg_bounds.secondary_key_bounds();
        assert_eq!(lower, encode_avg_sort_key(-5).to_vec());
        assert_eq!(upper, Some(encode_avg_sort_key(6).to_vec()));
    }

    #[test]
    fn merk_query_direction_follows_descending() {
        let bounds = AxisRangeBounds::Count { lo: 101, hi: 200 };
        assert!(bounds.merk_query(false).left_to_right);
        assert!(!bounds.merk_query(true).left_to_right);
    }
}

mod execution {
    //! End-to-end behaviour against the `restaurants` fixture: the
    //! dispatcher run through its public entry point (the same call
    //! drive-abci makes), no-proof and proved, on all three axes.

    use super::super::drive_dispatcher::{DocumentHavingRequest, DocumentHavingResponse};
    use super::super::mode_detection::detect_having_mode;
    use super::super::{AxisRangeBounds, DriveDocumentHavingQuery};
    use super::clause;
    use crate::drive::Drive;
    use crate::error::Error;
    use crate::query::drive_document_having_query::resolve_having_query_for_mode;
    use crate::query::drive_document_ranked_query::{
        RankedEntry, RankedEntryValue, RankedPaginationInputs, RANKED_COUNT_ORDER_KEY,
    };
    use crate::query::having::{HavingAggregateFunction, HavingClause, HavingOperator};
    use crate::query::projection::SelectProjection;
    use crate::query::OrderClause;
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
    use std::collections::BTreeMap;

    const GROUP_PROPERTY: &str = "restaurantId";

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

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

    /// Same real-write-path insertion as the ranked suite; see its docs
    /// for the disjoint-seed requirement.
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
                .unwrap_or_else(|e| {
                    panic!("expected to insert a {document_type_name} document: {e}")
                });
        }
    }

    /// One having request, minus the `prove` flag.
    #[derive(Clone)]
    struct HavingCase {
        document_type_name: &'static str,
        select: SelectProjection,
        having: HavingClause,
        /// `None` means no `ORDER BY` (ascending default);
        /// `Some(ascending)` orders by the selected aggregate.
        order_ascending: Option<bool>,
        limit: Option<u32>,
    }

    impl HavingCase {
        fn count(operator: HavingOperator, right: Value, limit: u32) -> Self {
            Self {
                document_type_name: "visit",
                select: SelectProjection::count_star(),
                having: clause(HavingAggregateFunction::Count, "", operator, right),
                order_ascending: None,
                limit: Some(limit),
            }
        }

        fn sum(operator: HavingOperator, right: Value, limit: u32) -> Self {
            Self {
                document_type_name: "tip",
                select: SelectProjection::sum("amount"),
                having: clause(HavingAggregateFunction::Sum, "amount", operator, right),
                order_ascending: None,
                limit: Some(limit),
            }
        }

        fn avg(operator: HavingOperator, right: Value, limit: u32) -> Self {
            Self {
                document_type_name: "review",
                select: SelectProjection::avg("grade"),
                having: clause(HavingAggregateFunction::Avg, "grade", operator, right),
                order_ascending: None,
                limit: Some(limit),
            }
        }

        fn ordered(mut self, ascending: bool) -> Self {
            self.order_ascending = Some(ascending);
            self
        }

        fn order_by(&self) -> Vec<OrderClause> {
            match self.order_ascending {
                None => Vec::new(),
                Some(ascending) => {
                    let field = match self.select.field.as_str() {
                        "" => RANKED_COUNT_ORDER_KEY.to_string(),
                        field => field.to_string(),
                    };
                    vec![OrderClause { field, ascending }]
                }
            }
        }
    }

    /// Run a case through the public dispatcher entry point — the same
    /// call drive-abci's routing layer makes.
    fn run(
        drive: &Drive,
        contract: &DataContract,
        case: &HavingCase,
        prove: bool,
    ) -> Result<DocumentHavingResponse, Error> {
        let group_by = vec![GROUP_PROPERTY.to_string()];
        let having = vec![case.having.clone()];
        let order_by = case.order_by();
        let document_type = contract
            .document_type_for_name(case.document_type_name)
            .expect("doctype exists");
        drive.execute_document_having_request(
            DocumentHavingRequest {
                contract,
                document_type,
                group_by: &group_by,
                select: case.select.clone(),
                having: &having,
                order_by: &order_by,
                where_clauses: &[],
                limit: case.limit,
                offset: None,
                has_start_at: false,
                prove,
            },
            None,
            platform_version(),
        )
    }

    fn entries_of(response: DocumentHavingResponse) -> Vec<RankedEntry> {
        match response {
            DocumentHavingResponse::Entries(entries) => entries,
            DocumentHavingResponse::Proof(_) => panic!("expected entries, got a proof"),
        }
    }

    fn proof_of(response: DocumentHavingResponse) -> Vec<u8> {
        match response {
            DocumentHavingResponse::Proof(proof) => proof,
            DocumentHavingResponse::Entries(_) => panic!("expected a proof, got entries"),
        }
    }

    fn keys_of(entries: &[RankedEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| {
                String::from_utf8(entry.key.clone()).expect("fixture group keys are utf-8")
            })
            .collect()
    }

    /// Rebuild the query the way a client would: re-run the same
    /// versioned validation (which resolves the bounds), then resolve
    /// the index off the contract — the shape the SDK's proof helper
    /// takes.
    fn client_side_query<'a>(
        contract: &'a DataContract,
        case: &HavingCase,
    ) -> DriveDocumentHavingQuery<'a> {
        let group_by = vec![GROUP_PROPERTY.to_string()];
        let having = vec![case.having.clone()];
        let order_by = case.order_by();
        let mode = detect_having_mode(
            &case.select,
            &group_by,
            &having,
            &order_by,
            &[],
            RankedPaginationInputs {
                limit: case.limit,
                offset: None,
                has_start_at: false,
            },
            platform_version(),
        )
        .expect("the case is well-formed");
        let indexes = contract
            .document_types()
            .get(case.document_type_name)
            .expect("doctype exists")
            .indexes();
        resolve_having_query_for_mode(
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

    /// Prove the case, verify the proof, and assert the verified
    /// entries and root hash match the live database.
    fn assert_proof_round_trips(
        drive: &Drive,
        contract: &DataContract,
        case: &HavingCase,
        expected: &[RankedEntry],
    ) {
        let proof = proof_of(run(drive, contract, case, true).expect("prove must succeed"));
        let query = client_side_query(contract, case);
        let (root_hash, verified) = query
            .verify_having_range_proof(&proof, platform_version())
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

    /// Visits per restaurant: alpha 1, beta 3, gamma 2, delta 4.
    /// `HAVING COUNT(*) > 2` must return exactly beta and delta, in
    /// ascending count order (no ORDER BY), and the proof must commit
    /// the same page.
    #[test]
    fn count_threshold_reads_and_proves_consistently() {
        let (drive, contract) = setup_restaurants();
        insert_docs(
            &drive,
            &contract,
            "visit",
            "guests",
            100,
            &[
                ("alpha", 1),
                ("beta", 1),
                ("beta", 2),
                ("beta", 3),
                ("gamma", 1),
                ("gamma", 2),
                ("delta", 1),
                ("delta", 2),
                ("delta", 3),
                ("delta", 4),
            ],
        );

        let case = HavingCase::count(HavingOperator::GreaterThan, Value::U64(2), 10);
        let entries = entries_of(run(&drive, &contract, &case, false).expect("read succeeds"));
        assert_eq!(
            keys_of(&entries),
            vec!["beta", "delta"],
            "ascending count order: beta (3) before delta (4)"
        );
        assert_eq!(
            entries.iter().map(|e| e.value).collect::<Vec<_>>(),
            vec![RankedEntryValue::Count(3), RankedEntryValue::Count(4)]
        );

        assert_proof_round_trips(&drive, &contract, &case, &entries);
    }

    /// Same state, descending: `HAVING $count >= 2 ORDER BY $count
    /// DESC` walks from the largest matching count down.
    #[test]
    fn descending_walk_returns_biggest_matches_first() {
        let (drive, contract) = setup_restaurants();
        insert_docs(
            &drive,
            &contract,
            "visit",
            "guests",
            200,
            &[
                ("alpha", 1),
                ("beta", 1),
                ("beta", 2),
                ("gamma", 1),
                ("gamma", 2),
                ("gamma", 3),
            ],
        );

        let case = HavingCase::count(HavingOperator::GreaterThanOrEquals, Value::U64(2), 10)
            .ordered(false);
        let entries = entries_of(run(&drive, &contract, &case, false).expect("read succeeds"));
        assert_eq!(keys_of(&entries), vec!["gamma", "beta"]);

        assert_proof_round_trips(&drive, &contract, &case, &entries);
    }

    /// Tips per restaurant: alpha 150, beta 900, gamma 400.
    /// `HAVING SUM(amount) BETWEEN 100 AND 500` returns alpha and gamma
    /// — bounds inclusive on both ends.
    #[test]
    fn sum_between_reads_and_proves_consistently() {
        let (drive, contract) = setup_restaurants();
        insert_docs(
            &drive,
            &contract,
            "tip",
            "amount",
            300,
            &[("alpha", 100), ("alpha", 50), ("beta", 900), ("gamma", 400)],
        );

        let case = HavingCase::sum(
            HavingOperator::Between,
            Value::Array(vec![Value::I64(100), Value::I64(500)]),
            10,
        );
        let entries = entries_of(run(&drive, &contract, &case, false).expect("read succeeds"));
        assert_eq!(keys_of(&entries), vec!["alpha", "gamma"]);
        assert_eq!(
            entries.iter().map(|e| e.value).collect::<Vec<_>>(),
            vec![RankedEntryValue::Sum(150), RankedEntryValue::Sum(400)]
        );

        assert_proof_round_trips(&drive, &contract, &case, &entries);
    }

    /// Reviews: alpha (90+80)/2 = 85, beta (60+70+50)/3 = 60, gamma 95.
    /// `HAVING AVG(grade) >= 85` returns alpha and gamma; the entries
    /// carry the exact fixed points.
    #[test]
    fn avg_threshold_reads_and_proves_consistently() {
        let (drive, contract) = setup_restaurants();
        insert_docs(
            &drive,
            &contract,
            "review",
            "grade",
            400,
            &[
                ("alpha", 90),
                ("alpha", 80),
                ("beta", 60),
                ("beta", 70),
                ("beta", 50),
                ("gamma", 95),
            ],
        );

        let case = HavingCase::avg(HavingOperator::GreaterThanOrEquals, Value::U64(85), 10);
        let entries = entries_of(run(&drive, &contract, &case, false).expect("read succeeds"));
        assert_eq!(keys_of(&entries), vec!["alpha", "gamma"]);
        assert_eq!(
            entries.iter().map(|e| e.value).collect::<Vec<_>>(),
            vec![
                RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(170, 2)),
                RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(95, 1)),
            ]
        );

        assert_proof_round_trips(&drive, &contract, &case, &entries);
    }

    /// The limit cuts an over-long match set — and the cut page still
    /// proves. With ascending order and `LIMIT 2`, the two *smallest*
    /// matching counts come back.
    #[test]
    fn limit_cuts_the_match_set_and_the_cut_page_proves() {
        let (drive, contract) = setup_restaurants();
        insert_docs(
            &drive,
            &contract,
            "visit",
            "guests",
            500,
            &[
                ("alpha", 1),
                ("alpha", 2),
                ("beta", 1),
                ("beta", 2),
                ("beta", 3),
                ("gamma", 1),
                ("gamma", 2),
                ("gamma", 3),
                ("gamma", 4),
            ],
        );

        let case = HavingCase::count(HavingOperator::GreaterThanOrEquals, Value::U64(2), 2);
        let entries = entries_of(run(&drive, &contract, &case, false).expect("read succeeds"));
        assert_eq!(
            keys_of(&entries),
            vec!["alpha", "beta"],
            "three groups match but the limit keeps the two smallest"
        );

        assert_proof_round_trips(&drive, &contract, &case, &entries);
    }

    /// A bound matching nothing is a legitimate, provable answer —
    /// both against populated state and against a freshly registered
    /// contract whose secondary is empty.
    #[test]
    fn an_empty_match_set_reads_empty_and_proves_empty() {
        let (drive, contract) = setup_restaurants();

        // Empty secondary (no documents at all): the unproven read
        // returns the empty list, but grovedb's range prover — unlike
        // the ranked surface's paginated prover — has no absence-proof
        // shape for a completely empty tree and refuses. drive-abci
        // maps this exact failure class onto an `InvalidArgument`
        // telling the caller to retry unproved
        // (`empty_ranking_proof_rejection`); at the drive level it
        // surfaces as the grovedb error asserted here. If a future
        // grovedb pin makes empty range proofs work, this arm should
        // flip to a round-trip assertion.
        let case = HavingCase::count(HavingOperator::GreaterThan, Value::U64(100), 10);
        let entries = entries_of(run(&drive, &contract, &case, false).expect("read succeeds"));
        assert!(entries.is_empty());
        let error = run(&drive, &contract, &case, true)
            .expect_err("proving against an empty secondary is refused by grovedb");
        assert!(
            format!("{error}").contains("Cannot create proof for empty tree"),
            "the failure must be the recognized empty-tree class, got: {error}"
        );

        // Populated secondary, bound above every count: a genuine
        // absence proof, which works — the tree has content to anchor
        // the boundary commitments to.
        insert_docs(
            &drive,
            &contract,
            "visit",
            "guests",
            600,
            &[("alpha", 1), ("beta", 1), ("beta", 2)],
        );
        let entries = entries_of(run(&drive, &contract, &case, false).expect("read succeeds"));
        assert!(entries.is_empty());
        assert_proof_round_trips(&drive, &contract, &case, &entries);
    }

    /// A proof generated for one bound must not verify as a different
    /// bound **whose answer differs**: verification re-runs the Merk
    /// query against the proof, so a wider bound demands proof of a
    /// group the narrower proof never committed (gamma, count 2, below)
    /// and fails.
    ///
    /// The state is chosen so the two bounds genuinely disagree. With
    /// no group between the two thresholds the same proof *does*
    /// verify under both — correctly, because the range boundaries
    /// prove both claims — so the distinguishing group is the point of
    /// the fixture.
    #[test]
    fn a_proof_does_not_verify_under_different_bounds() {
        let (drive, contract) = setup_restaurants();
        insert_docs(
            &drive,
            &contract,
            "visit",
            "guests",
            700,
            &[
                ("alpha", 1),
                ("gamma", 1),
                ("gamma", 2),
                ("beta", 1),
                ("beta", 2),
                ("beta", 3),
            ],
        );

        let over_two = HavingCase::count(HavingOperator::GreaterThan, Value::U64(2), 10);
        let proof = proof_of(run(&drive, &contract, &over_two, true).expect("prove succeeds"));

        // Honest verification succeeds…
        assert!(client_side_query(&contract, &over_two)
            .verify_having_range_proof(&proof, platform_version())
            .is_ok());

        // …but the same bytes under a different threshold must not.
        let over_one = HavingCase::count(HavingOperator::GreaterThan, Value::U64(1), 10);
        let mut tampered_query = client_side_query(&contract, &over_one);
        assert_eq!(
            tampered_query.bounds,
            AxisRangeBounds::Count {
                lo: 2,
                hi: u64::MAX
            }
        );
        assert!(
            tampered_query
                .verify_having_range_proof(&proof, platform_version())
                .is_err(),
            "a proof of `> 2` must not verify as `> 1`"
        );

        // Nor under a different direction or limit.
        tampered_query = client_side_query(&contract, &over_two);
        tampered_query.descending = true;
        assert!(tampered_query
            .verify_having_range_proof(&proof, platform_version())
            .is_err());

        tampered_query = client_side_query(&contract, &over_two);
        tampered_query.limit = 5;
        assert!(tampered_query
            .verify_having_range_proof(&proof, platform_version())
            .is_err());
    }

    /// A `having` on an axis no index declares is refused with the
    /// contract keyword the author needs to add. The `review` doctype's
    /// index is `rankedAverageable` only — a COUNT bound has no
    /// covering secondary.
    #[test]
    fn a_bound_on_an_undeclared_axis_names_the_missing_keyword() {
        let (drive, contract) = setup_restaurants();
        let case = HavingCase {
            document_type_name: "review",
            select: SelectProjection::count_star(),
            having: clause(
                HavingAggregateFunction::Count,
                "",
                HavingOperator::GreaterThan,
                Value::U64(2),
            ),
            order_ascending: None,
            limit: Some(10),
        };
        let error = run(&drive, &contract, &case, false).expect_err("no covering axis");
        assert!(
            format!("{error}").contains("rankedCountable"),
            "the rejection must name the missing keyword, got: {error}"
        );
    }

    /// Equal bounds are a point lookup on the axis: `HAVING SUM(amount)
    /// = 400` returns exactly the group whose running sum is 400.
    #[test]
    fn equality_is_a_point_bound() {
        let (drive, contract) = setup_restaurants();
        insert_docs(
            &drive,
            &contract,
            "tip",
            "amount",
            800,
            &[("alpha", 150), ("beta", 400), ("gamma", 400)],
        );

        let case = HavingCase::sum(HavingOperator::Equal, Value::I64(400), 10);
        let entries = entries_of(run(&drive, &contract, &case, false).expect("read succeeds"));
        assert_eq!(
            keys_of(&entries),
            vec!["beta", "gamma"],
            "equal sums tie-break by group key in walk direction"
        );

        assert_proof_round_trips(&drive, &contract, &case, &entries);
    }

    /// Continuation-by-bound: after a page cut at the limit, the
    /// caller tightens the bound past the last seen value and picks up
    /// at the next *distinct* aggregate value. This is deliberately not
    /// full pagination — a cut inside a tie cannot be continued (the
    /// tied groups past the limit are unreachable without a
    /// composite-key cursor); this fixture's counts are distinct, which
    /// is the case the continuation serves.
    #[test]
    fn tightening_the_bound_continues_past_a_cut_page() {
        let (drive, contract) = setup_restaurants();
        insert_docs(
            &drive,
            &contract,
            "visit",
            "guests",
            900,
            &[
                ("alpha", 1),
                ("alpha", 2),
                ("beta", 1),
                ("beta", 2),
                ("beta", 3),
                ("gamma", 1),
                ("gamma", 2),
                ("gamma", 3),
                ("gamma", 4),
            ],
        );

        // Page 1: counts >= 2, limit 1 → alpha (count 2).
        let page_one = HavingCase::count(HavingOperator::GreaterThanOrEquals, Value::U64(2), 1);
        let first = entries_of(run(&drive, &contract, &page_one, false).expect("read succeeds"));
        assert_eq!(keys_of(&first), vec!["alpha"]);
        let RankedEntryValue::Count(last_seen) = first[0].value else {
            panic!("count axis returns count values");
        };

        // Page 2: counts > last seen → beta, gamma.
        let page_two = HavingCase::count(HavingOperator::GreaterThan, Value::U64(last_seen), 10);
        let rest = entries_of(run(&drive, &contract, &page_two, false).expect("read succeeds"));
        assert_eq!(keys_of(&rest), vec!["beta", "gamma"]);

        assert_proof_round_trips(&drive, &contract, &page_two, &rest);
    }
}

mod identifier_group_keys {
    //! `SELECT AVG(grade) FROM grades GROUP BY identityId HAVING
    //! AVG(grade) > 80` — the same surface as the `execution` suite
    //! above, but with a **32-byte identifier** as the group key
    //! instead of a string. Identifier and string properties encode
    //! differently into the axis secondary's `sort_key‖group_key`
    //! keyspace, so this pins that identifier group keys round-trip
    //! byte-exact through the read, the proof, and the verifier.

    use super::super::drive_dispatcher::{DocumentHavingRequest, DocumentHavingResponse};
    use super::super::mode_detection::detect_having_mode;
    use super::super::DriveDocumentHavingQuery;
    use super::clause;
    use crate::drive::Drive;
    use crate::query::drive_document_having_query::resolve_having_query_for_mode;
    use crate::query::drive_document_ranked_query::{
        RankedEntry, RankedEntryValue, RankedPaginationInputs,
    };
    use crate::query::having::{HavingAggregateFunction, HavingOperator};
    use crate::query::projection::SelectProjection;
    use crate::query::OrderClause;
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
    use std::collections::BTreeMap;

    const GROUP_PROPERTY: &str = "identityId";
    const DOCUMENT_TYPE: &str = "grade";

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    fn setup_grades_ranked() -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = platform_version();
        let contract = json_document_to_contract(
            "tests/supporting_files/contract/grades/grades-ranked-contract.json",
            false,
            pv,
        )
        .expect("expected to parse the ranked grades contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply the ranked grades contract");
        (drive, contract)
    }

    fn insert_grades(
        drive: &Drive,
        contract: &DataContract,
        first_seed: u64,
        rows: &[([u8; 32], i64)],
    ) {
        let pv = platform_version();
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("grade doctype exists");
        for (i, (identity, grade)) in rows.iter().enumerate() {
            let mut doc: Document = document_type
                .random_document(Some(first_seed + i as u64), pv)
                .expect("random document");
            let mut props = BTreeMap::new();
            props.insert(GROUP_PROPERTY.to_string(), Value::Identifier(*identity));
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

    fn run(
        drive: &Drive,
        contract: &DataContract,
        order_by: &[OrderClause],
        prove: bool,
    ) -> DocumentHavingResponse {
        let group_by = vec![GROUP_PROPERTY.to_string()];
        let having = vec![clause(
            HavingAggregateFunction::Avg,
            "grade",
            HavingOperator::GreaterThan,
            Value::U64(80),
        )];
        drive
            .execute_document_having_request(
                DocumentHavingRequest {
                    contract,
                    document_type: contract
                        .document_type_for_name(DOCUMENT_TYPE)
                        .expect("grade doctype exists"),
                    group_by: &group_by,
                    select: SelectProjection::avg("grade"),
                    having: &having,
                    order_by,
                    where_clauses: &[],
                    limit: Some(10),
                    offset: None,
                    has_start_at: false,
                    prove,
                },
                None,
                platform_version(),
            )
            .expect("the having request must execute")
    }

    fn client_side_query<'a>(
        contract: &'a DataContract,
        order_by: &[OrderClause],
    ) -> DriveDocumentHavingQuery<'a> {
        let group_by = vec![GROUP_PROPERTY.to_string()];
        let having = vec![clause(
            HavingAggregateFunction::Avg,
            "grade",
            HavingOperator::GreaterThan,
            Value::U64(80),
        )];
        let mode = detect_having_mode(
            &SelectProjection::avg("grade"),
            &group_by,
            &having,
            order_by,
            &[],
            RankedPaginationInputs {
                limit: Some(10),
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
        resolve_having_query_for_mode(
            contract.id_ref().to_buffer(),
            contract
                .document_type_for_name(DOCUMENT_TYPE)
                .expect("grade doctype exists"),
            DOCUMENT_TYPE.to_string(),
            indexes,
            &mode,
            platform_version(),
        )
        .expect("the fixture declares the avg axis")
    }

    fn assert_proof_round_trips(
        drive: &Drive,
        contract: &DataContract,
        order_by: &[OrderClause],
        expected: &[RankedEntry],
    ) {
        let proof = match run(drive, contract, order_by, true) {
            DocumentHavingResponse::Proof(proof) => proof,
            DocumentHavingResponse::Entries(_) => panic!("expected a proof, got entries"),
        };
        let (root_hash, verified) = client_side_query(contract, order_by)
            .verify_having_range_proof(&proof, platform_version())
            .expect("the proof must verify");
        assert_eq!(
            verified, expected,
            "verified entries must equal what the unproven read returned"
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
    }

    /// Averages exactly *at* the threshold stay out (`>` is strict),
    /// fractional averages just above it come in (80.5 > 80 even
    /// though both grades round-trip as integers), and the entry keys
    /// are the raw 32-byte identifiers.
    #[test]
    fn avg_threshold_over_identifier_groups_reads_and_proves() {
        let (drive, contract) = setup_grades_ranked();
        let at_threshold = [1u8; 32]; // 80, 80 → avg 80: excluded
        let just_above = [2u8; 32]; // 80, 81 → avg 80.5: included
        let well_above = [3u8; 32]; // 85, 95 → avg 90:   included
        let below = [4u8; 32]; // 60, 80 → avg 70:  excluded
        insert_grades(
            &drive,
            &contract,
            1000,
            &[
                (at_threshold, 80),
                (at_threshold, 80),
                (just_above, 80),
                (just_above, 81),
                (well_above, 85),
                (well_above, 95),
                (below, 60),
                (below, 80),
            ],
        );

        let entries = match run(&drive, &contract, &[], false) {
            DocumentHavingResponse::Entries(entries) => entries,
            DocumentHavingResponse::Proof(_) => panic!("expected entries, got a proof"),
        };
        assert_eq!(
            entries,
            vec![
                RankedEntry {
                    key: just_above.to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(161, 2)),
                },
                RankedEntry {
                    key: well_above.to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(180, 2)),
                },
            ],
            "ascending walk: 80.5 then 90, keyed by raw identifier bytes"
        );
        assert_proof_round_trips(&drive, &contract, &[], &entries);

        // `ORDER BY AVG(grade) DESC` walks the same match set from the
        // top; the identifier keys must survive the flipped direction
        // and its proof too.
        let descending = vec![OrderClause {
            field: "grade".to_string(),
            ascending: false,
        }];
        let flipped = match run(&drive, &contract, &descending, false) {
            DocumentHavingResponse::Entries(entries) => entries,
            DocumentHavingResponse::Proof(_) => panic!("expected entries, got a proof"),
        };
        assert_eq!(
            flipped.iter().map(|e| &e.key).collect::<Vec<_>>(),
            vec![&well_above.to_vec(), &just_above.to_vec()],
            "descending walk: 90 then 80.5"
        );
        assert_proof_round_trips(&drive, &contract, &descending, &flipped);
    }
}

mod pinned_prefix {
    //! `SELECT AVG(grade) FROM grades WHERE identityId = X GROUP BY
    //! class HAVING AVG(grade) > 80` — the compound ranked index
    //! `[identityId, class]` with its leading property pinned by an
    //! equality `where` clause. Per-prefix semantics: each identity's
    //! terminal `class` property-name tree is its own indexed tree, so
    //! the bound reads (and proves) only the pinned identity's class
    //! groups. Documents are inserted through the real write path, so
    //! these tests also pin that the document walkers create and
    //! maintain the per-prefix secondaries for compound ranked indexes.

    use super::super::drive_dispatcher::{DocumentHavingRequest, DocumentHavingResponse};
    use super::super::mode_detection::detect_having_mode;
    use super::super::DriveDocumentHavingQuery;
    use super::clause;
    use crate::drive::Drive;
    use crate::error::query::QuerySyntaxError;
    use crate::error::Error;
    use crate::query::drive_document_having_query::resolve_having_query_for_mode;
    use crate::query::drive_document_ranked_query::{
        RankedEntry, RankedEntryValue, RankedPaginationInputs,
    };
    use crate::query::having::{HavingAggregateFunction, HavingOperator};
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
    use std::collections::BTreeMap;

    const PREFIX_PROPERTY: &str = "identityId";
    const GROUP_PROPERTY: &str = "class";
    const DOCUMENT_TYPE: &str = "grade";

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

    fn insert_grades(
        drive: &Drive,
        contract: &DataContract,
        first_seed: u64,
        rows: &[([u8; 32], &str, i64)],
    ) {
        let pv = platform_version();
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("grade doctype exists");
        for (i, (identity, class, grade)) in rows.iter().enumerate() {
            let mut doc: Document = document_type
                .random_document(Some(first_seed + i as u64), pv)
                .expect("random document");
            let mut props = BTreeMap::new();
            props.insert(PREFIX_PROPERTY.to_string(), Value::Identifier(*identity));
            props.insert(GROUP_PROPERTY.to_string(), Value::Text(class.to_string()));
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
        order_by: &[OrderClause],
        prove: bool,
    ) -> Result<DocumentHavingResponse, Error> {
        let group_by = vec![GROUP_PROPERTY.to_string()];
        let having = vec![clause(
            HavingAggregateFunction::Avg,
            "grade",
            HavingOperator::GreaterThan,
            Value::U64(80),
        )];
        drive.execute_document_having_request(
            DocumentHavingRequest {
                contract,
                document_type: contract
                    .document_type_for_name(DOCUMENT_TYPE)
                    .expect("grade doctype exists"),
                group_by: &group_by,
                select: SelectProjection::avg("grade"),
                having: &having,
                order_by,
                where_clauses,
                limit: Some(10),
                offset: None,
                has_start_at: false,
                prove,
            },
            None,
            platform_version(),
        )
    }

    fn entries_of(response: DocumentHavingResponse) -> Vec<RankedEntry> {
        match response {
            DocumentHavingResponse::Entries(entries) => entries,
            DocumentHavingResponse::Proof(_) => panic!("expected entries, got a proof"),
        }
    }

    fn client_side_query<'a>(
        contract: &'a DataContract,
        where_clauses: &[WhereClause],
        order_by: &[OrderClause],
    ) -> DriveDocumentHavingQuery<'a> {
        let group_by = vec![GROUP_PROPERTY.to_string()];
        let having = vec![clause(
            HavingAggregateFunction::Avg,
            "grade",
            HavingOperator::GreaterThan,
            Value::U64(80),
        )];
        let mode = detect_having_mode(
            &SelectProjection::avg("grade"),
            &group_by,
            &having,
            order_by,
            where_clauses,
            RankedPaginationInputs {
                limit: Some(10),
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
        resolve_having_query_for_mode(
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

    fn assert_proof_round_trips(
        drive: &Drive,
        contract: &DataContract,
        where_clauses: &[WhereClause],
        order_by: &[OrderClause],
        expected: &[RankedEntry],
    ) {
        let proof = match run(drive, contract, where_clauses, order_by, true)
            .expect("prove must succeed")
        {
            DocumentHavingResponse::Proof(proof) => proof,
            DocumentHavingResponse::Entries(_) => panic!("expected a proof, got entries"),
        };
        let (root_hash, verified) = client_side_query(contract, where_clauses, order_by)
            .verify_having_range_proof(&proof, platform_version())
            .expect("the proof must verify");
        assert_eq!(
            verified, expected,
            "verified entries must equal what the unproven read returned"
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
    }

    const IDENTITY_X: [u8; 32] = [1u8; 32];
    const IDENTITY_Y: [u8; 32] = [2u8; 32];

    /// The shared dataset: two identities whose class groups overlap by
    /// name, so cross-prefix leakage is visible by construction —
    /// `math` fails X's bound (avg 80, strict `>`) but passes Y's (avg
    /// 92.5), and `science` exists only under Y.
    fn insert_two_identities(drive: &Drive, contract: &DataContract) {
        insert_grades(
            drive,
            contract,
            1000,
            &[
                // Identity X: math avg 80 (at threshold, excluded under >),
                // english avg 80.5 (fractional, just above), art avg 90,
                // history avg 70.
                (IDENTITY_X, "math", 80),
                (IDENTITY_X, "math", 80),
                (IDENTITY_X, "english", 80),
                (IDENTITY_X, "english", 81),
                (IDENTITY_X, "art", 85),
                (IDENTITY_X, "art", 95),
                (IDENTITY_X, "history", 60),
                (IDENTITY_X, "history", 80),
                // Identity Y: math avg 92.5, science avg 95 — both would
                // qualify for X's bound too if prefixes leaked.
                (IDENTITY_Y, "math", 90),
                (IDENTITY_Y, "math", 95),
                (IDENTITY_Y, "science", 95),
                (IDENTITY_Y, "science", 95),
            ],
        );
    }

    /// Exact-threshold exclusion, fractional inclusion, byte-exact
    /// string group keys, both walk directions, proof round-trips —
    /// and isolation: identity Y's qualifying classes never appear in
    /// X's result, even where the class *name* collides.
    #[test]
    fn avg_threshold_over_pinned_prefix_reads_and_proves() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_two_identities(&drive, &contract);

        // Pinned to X, ascending: english (80.5) then art (90). math sits
        // exactly at the threshold and stays out under `>`; Y's math
        // (92.5) and science (95) must not leak in.
        let x_pin = pin(IDENTITY_X);
        let entries =
            entries_of(run(&drive, &contract, &x_pin, &[], false).expect("read succeeds"));
        assert_eq!(
            entries,
            vec![
                RankedEntry {
                    key: b"english".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(161, 2)),
                },
                RankedEntry {
                    key: b"art".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(180, 2)),
                },
            ],
            "ascending walk over X's classes: 80.5 then 90, keyed by raw utf-8 class bytes"
        );
        assert_proof_round_trips(&drive, &contract, &x_pin, &[], &entries);

        // Same pin, descending: art then english, with its own proof.
        let descending = vec![OrderClause {
            field: "grade".to_string(),
            ascending: false,
        }];
        let flipped =
            entries_of(run(&drive, &contract, &x_pin, &descending, false).expect("read succeeds"));
        assert_eq!(
            flipped.iter().map(|e| &e.key).collect::<Vec<_>>(),
            vec![&b"art".to_vec(), &b"english".to_vec()],
            "descending walk: 90 then 80.5"
        );
        assert_proof_round_trips(&drive, &contract, &x_pin, &descending, &flipped);

        // Pinned to Y: math qualifies *here* (92.5) even though the same
        // class name failed X's bound — the two prefixes rank separately.
        let y_pin = pin(IDENTITY_Y);
        let y_entries =
            entries_of(run(&drive, &contract, &y_pin, &[], false).expect("read succeeds"));
        assert_eq!(
            y_entries,
            vec![
                RankedEntry {
                    key: b"math".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(185, 2)),
                },
                RankedEntry {
                    key: b"science".to_vec(),
                    value: RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(190, 2)),
                },
            ],
            "Y's own classes: math 92.5 then science 95"
        );
        assert_proof_round_trips(&drive, &contract, &y_pin, &[], &y_entries);
    }

    /// An unpinned prefix cannot be served: with `group_by = class` and
    /// no `where`, no single-property ranked index on `class` exists,
    /// and the compound index's per-prefix secondaries have no global
    /// ordering to read. The rejection names the missing coverage.
    #[test]
    fn unpinned_prefix_is_rejected() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_two_identities(&drive, &contract);

        let error = run(&drive, &contract, &[], &[], false)
            .expect_err("an unpinned compound prefix must not resolve");
        match error {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(message)) => {
                assert!(
                    message.contains("no ranked index covers")
                        && message.contains("single-property index on `class`"),
                    "the rejection must explain the missing coverage, got: {message}"
                );
            }
            other => panic!("expected a no-covering-index rejection, got {other:?}"),
        }
    }

    /// `IN` on the prefix is rejected at detection with the
    /// not-yet-supported message (v1 pins are equality-only), and a pin
    /// on a property that is not the index's leading property fails
    /// resolution.
    #[test]
    fn in_prefix_and_wrong_pins_are_rejected() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_two_identities(&drive, &contract);

        let in_clause = vec![WhereClause {
            field: PREFIX_PROPERTY.to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Identifier(IDENTITY_X),
                Value::Identifier(IDENTITY_Y),
            ]),
        }];
        let error = run(&drive, &contract, &in_clause, &[], false)
            .expect_err("IN prefixes are not yet supported");
        match error {
            Error::Query(QuerySyntaxError::Unsupported(message)) => {
                assert!(
                    message.contains("IN") && message.contains("not yet supported"),
                    "the IN rejection must say it is a not-yet capability, got: {message}"
                );
            }
            other => panic!("expected Unsupported for an IN prefix, got {other:?}"),
        }

        // A pin on the wrong property: `grade` is not the index's
        // leading property, so nothing covers [grade, class].
        let wrong_pin = vec![WhereClause {
            field: "grade".to_string(),
            operator: WhereOperator::Equal,
            value: Value::I64(80),
        }];
        let error = run(&drive, &contract, &wrong_pin, &[], false)
            .expect_err("a pin on a non-leading property must not resolve");
        assert!(
            matches!(
                error,
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(_))
            ),
            "expected a no-covering-index rejection, got {error:?}"
        );
    }

    /// A pin on an identity that never inserted a document addresses a
    /// prefix value tree that does not exist. The read and the prover
    /// both surface an error rather than fabricating an empty page —
    /// same contract as the empty-secondary limitation on the
    /// single-property surface (the abci layer maps these to a
    /// client-visible rejection).
    #[test]
    fn unknown_prefix_value_errors_rather_than_fabricating_an_empty_page() {
        let (drive, contract) = setup_grades_compound_ranked();
        insert_two_identities(&drive, &contract);

        let unknown = pin([9u8; 32]);
        assert!(
            run(&drive, &contract, &unknown, &[], false).is_err(),
            "reading a never-written prefix value tree must error"
        );
        assert!(
            run(&drive, &contract, &unknown, &[], true).is_err(),
            "proving a never-written prefix value tree must error"
        );
    }
}
