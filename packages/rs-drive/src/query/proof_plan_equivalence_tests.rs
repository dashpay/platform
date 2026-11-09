//! A proved count, sum or average is planned from its request alone
//! ([`CountProofPlan`], [`SumProofPlan`]) and the dispatcher proves the path
//! query the plan lowers to. These tests pin the dispatcher's answer — the
//! proof, or the refusal — for every request of a matrix over a populated
//! document type, to the answer of the per-mode `execute_*_with_proof` method
//! of the query object the plan names, and check the matrix reaches every
//! plan. The fixture has no `summableOffCountIndex` index; the count range
//! proofs that read such an index through its sums are proved through the
//! dispatcher and verified in `summable_off_count_index_e2e_tests`.

use crate::config::DriveConfig;
use crate::drive::Drive;
use crate::query::drive_document_average_query::{
    AverageMode, DocumentAverageRequest, DocumentAverageResponse,
};
use crate::query::drive_document_sum_query::mode_detection::detect_sum_mode_from_inputs;
use crate::query::drive_document_sum_query::{
    DocumentSumRequest, DocumentSumResponse, DriveDocumentSumQuery, SumMode, SumProofPlan,
};
use crate::query::{
    validate_and_canonicalize_where_clauses, CountMode, CountProofPlan, DocumentCountRequest,
    DocumentCountResponse, DriveDocumentCountQuery, OrderClause, WhereClause, WhereOperator,
};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::document_type::Index;
use dpp::data_contract::DataContractFactory;
use dpp::document::{Document, DocumentV0};
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

/// A `widget` type that counts and sums its documents by `amount`, with
/// brand and brand-and-colour indexes that count and sum over ranges, and a
/// colour index that only counts.
fn widget_contract() -> DataContract {
    let schemas = platform_value!({"widget": {
        "type": "object",
        "documentsCountable": true,
        "documentsSummable": "amount",
        "properties": {
            "brand": {"type": "string", "position": 0, "maxLength": 32},
            "color": {"type": "string", "position": 1, "maxLength": 32},
            "amount": {"type": "integer", "position": 2, "minimum": 0, "maximum": 1000}
        },
        "required": ["brand", "color", "amount"],
        "indices": [
            {"name": "byBrand", "properties": [{"brand": "asc"}],
             "summable": "amount", "rangeSummable": true,
             "countable": "countable", "rangeCountable": true},
            {"name": "byBrandColor", "properties": [{"brand": "asc"}, {"color": "asc"}],
             "summable": "amount", "rangeSummable": true,
             "countable": "countable", "rangeCountable": true},
            {"name": "byColor", "properties": [{"color": "asc"}],
             "countable": "countable"}
        ],
        "additionalProperties": false
    }});
    DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::from([41u8; 32]), 0, schemas, None, None)
        .expect("the widget contract is well-formed")
        .data_contract_owned()
}

fn populated_drive(contract: &DataContract, platform_version: &PlatformVersion) -> Drive {
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    drive
        .apply_contract(
            contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply the widget contract");
    let document_type = contract.document_type_for_name("widget").expect("widget");
    let rows = [
        ("a", "red", 3u64),
        ("a", "zinc", 5),
        ("b", "navy", 7),
        ("b", "red", 11),
        ("c", "olive", 17),
        ("d", "plum", 23),
    ];
    for (i, (brand, color, amount)) in rows.into_iter().enumerate() {
        let document: Document = DocumentV0 {
            id: Identifier::from([i as u8 + 1; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: BTreeMap::from([
                ("brand".to_string(), Value::Text(brand.to_string())),
                ("color".to_string(), Value::Text(color.to_string())),
                ("amount".to_string(), Value::U64(amount)),
            ]),
            ..Default::default()
        }
        .into();
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("insert a widget");
    }
    drive
}

fn clause(field: &str, operator: WhereOperator, value: Value) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator,
        value,
    }
}

/// Where shapes across every proof mode: none, equalities, an `In`, ranges
/// with and without an `In` or a second range.
fn shapes() -> Vec<Vec<WhereClause>> {
    let text = |s: &str| Value::Text(s.to_string());
    vec![
        vec![],
        vec![clause("brand", WhereOperator::Equal, text("a"))],
        vec![clause(
            "brand",
            WhereOperator::In,
            platform_value!(["a", "b"]),
        )],
        vec![clause("brand", WhereOperator::GreaterThan, text("a"))],
        vec![
            clause("brand", WhereOperator::In, platform_value!(["a", "b"])),
            clause("color", WhereOperator::GreaterThan, text("m")),
        ],
        vec![
            clause("brand", WhereOperator::GreaterThan, text("a")),
            clause("color", WhereOperator::GreaterThan, text("m")),
        ],
        vec![
            clause("brand", WhereOperator::Equal, text("a")),
            clause("color", WhereOperator::GreaterThan, text("m")),
        ],
        vec![clause("color", WhereOperator::Equal, text("red"))],
    ]
}

const MODES: [CountMode; 4] = [
    CountMode::Aggregate,
    CountMode::GroupByIn,
    CountMode::GroupByRange,
    CountMode::GroupByCompound,
];

fn orders() -> [Vec<OrderClause>; 2] {
    [
        vec![],
        vec![OrderClause {
            field: "brand".to_string(),
            ascending: false,
        }],
    ]
}

fn count_query<'a>(
    contract: &DataContract,
    document_type: DocumentTypeRef<'a>,
    where_clauses: &[WhereClause],
    index: &'a Index,
) -> DriveDocumentCountQuery<'a> {
    DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses: where_clauses.to_vec(),
    }
}

/// The answer, proof or refusal, a proved count gets from the per-mode
/// method its plan names, and the plan's variant.
#[allow(clippy::too_many_arguments)]
fn count_by_mode(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    where_clauses: &[WhereClause],
    mode: CountMode,
    order_clauses: &[OrderClause],
    limit: Option<u32>,
    config: &DriveConfig,
    platform_version: &PlatformVersion,
) -> Result<(Vec<u8>, &'static str), String> {
    let canonical =
        validate_and_canonicalize_where_clauses(where_clauses.to_vec(), platform_version)
            .map_err(|e| format!("{e:?}"))?;
    let proof_mode =
        DriveDocumentCountQuery::detect_mode_versioned(&canonical, mode, true, platform_version)
            .map_err(|e| format!("{:?}", crate::error::Error::Query(e)))?;
    let plan = DriveDocumentCountQuery::proof_plan(
        &document_type,
        &canonical,
        &[],
        proof_mode,
        order_clauses,
        limit,
        Some(config.max_query_limit),
    )
    .map_err(|e| format!("{e:?}"))?;
    let query = |index| count_query(contract, document_type, &canonical, index);
    let (proof, variant) = match plan {
        CountProofPlan::PrimaryKeyCountTree => (
            drive
                .grove
                .get_proved_path_query(
                    &DriveDocumentCountQuery::primary_key_count_tree_path_query(
                        contract.id().to_buffer(),
                        "widget",
                    ),
                    None,
                    None,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .map_err(|e| crate::error::Error::GroveDB(Box::new(e))),
            "primary key",
        ),
        CountProofPlan::PointLookup(index) => (
            query(index).execute_point_lookup_count_with_proof(drive, None, platform_version),
            "point lookup",
        ),
        CountProofPlan::Range(index) => (
            query(index).execute_aggregate_count_with_proof(drive, None, platform_version),
            "range",
        ),
        CountProofPlan::RangeDistinct {
            index,
            limit,
            left_to_right,
        } => (
            query(index).execute_distinct_count_with_proof(
                drive,
                limit,
                left_to_right,
                None,
                platform_version,
            ),
            "distinct",
        ),
        CountProofPlan::RangeAggregateCarrier {
            index,
            limit,
            left_to_right,
        } => (
            query(index).execute_carrier_aggregate_count_with_proof(
                drive,
                limit,
                left_to_right,
                None,
                platform_version,
            ),
            "carrier",
        ),
    };
    proof
        .map(|proof| (proof, variant))
        .map_err(|e| format!("{e:?}"))
}

const EVERY_PLAN: [&str; 5] = [
    "primary key",
    "point lookup",
    "range",
    "distinct",
    "carrier",
];

#[test]
fn should_prove_a_count_plan_as_its_mode_proves_its_index() {
    let platform_version = PlatformVersion::latest();
    let contract = widget_contract();
    let drive = populated_drive(&contract, platform_version);
    let document_type = contract.document_type_for_name("widget").expect("widget");
    let config = DriveConfig::default();
    let mut reached = BTreeSet::new();
    for where_clauses in shapes() {
        for mode in MODES {
            for limit in [None, Some(3u32)] {
                for order_clauses in orders() {
                    let dispatched = match drive.execute_document_count_request(
                        DocumentCountRequest {
                            contract: &contract,
                            document_type,
                            where_clauses: where_clauses.clone(),
                            resolved_time_ranges: vec![],
                            order_clauses: order_clauses.clone(),
                            mode,
                            limit,
                            prove: true,
                            drive_config: &config,
                        },
                        None,
                        platform_version,
                    ) {
                        Ok(DocumentCountResponse::Proof(proof)) => Ok(proof),
                        Ok(other) => panic!("a proved count answered {other:?}"),
                        Err(e) => Err(format!("{e:?}")),
                    };
                    let by_mode = count_by_mode(
                        &drive,
                        &contract,
                        document_type,
                        &where_clauses,
                        mode,
                        &order_clauses,
                        limit,
                        &config,
                        platform_version,
                    );
                    if let Ok((_, variant)) = &by_mode {
                        reached.insert(*variant);
                    }
                    assert_eq!(
                        dispatched,
                        by_mode.map(|(proof, _)| proof),
                        "{where_clauses:?} {mode:?} {limit:?} {order_clauses:?}"
                    );
                }
            }
        }
    }
    assert_eq!(reached, BTreeSet::from(EVERY_PLAN));
}

/// The answer, proof or refusal, a proved sum or average gets from the
/// per-mode method its plan names, and the plan's variant; `with_counts` for
/// an average.
fn sum_plan_by_mode(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    where_clauses: &[WhereClause],
    plan: SumProofPlan,
    with_counts: bool,
    platform_version: &PlatformVersion,
) -> Result<(Vec<u8>, &'static str), crate::error::Error> {
    let query = |index| DriveDocumentSumQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses: where_clauses.to_vec(),
        sum_property: "amount".to_string(),
    };
    let (proof, variant) = match plan {
        SumProofPlan::PrimaryKeySumTree => (
            drive
                .grove
                .get_proved_path_query(
                    &DriveDocumentSumQuery::primary_key_sum_path_query(
                        contract.id().to_buffer(),
                        "widget",
                    ),
                    None,
                    None,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .map_err(|e| crate::error::Error::GroveDB(Box::new(e))),
            "primary key",
        ),
        SumProofPlan::PointLookup(index) => (
            query(index).execute_point_lookup_sum_with_proof(drive, None, platform_version),
            "point lookup",
        ),
        SumProofPlan::Range(index) if with_counts => (
            query(index).execute_aggregate_count_and_sum_with_proof(drive, None, platform_version),
            "range",
        ),
        SumProofPlan::Range(index) => (
            query(index).execute_aggregate_sum_with_proof(drive, None, platform_version),
            "range",
        ),
        SumProofPlan::RangeDistinct {
            index,
            limit,
            left_to_right,
        } => (
            query(index).execute_distinct_sum_with_proof(
                drive,
                limit,
                left_to_right,
                None,
                platform_version,
            ),
            "distinct",
        ),
        SumProofPlan::RangeAggregateCarrier {
            index,
            limit,
            left_to_right,
        } if with_counts => (
            query(index).execute_carrier_aggregate_count_and_sum_with_proof(
                drive,
                limit,
                left_to_right,
                None,
                platform_version,
            ),
            "carrier",
        ),
        SumProofPlan::RangeAggregateCarrier {
            index,
            limit,
            left_to_right,
        } => (
            query(index).execute_carrier_aggregate_sum_with_proof(
                drive,
                limit,
                left_to_right,
                None,
                platform_version,
            ),
            "carrier",
        ),
    };
    proof.map(|proof| (proof, variant))
}

#[test]
fn should_prove_a_sum_or_average_plan_as_its_mode_proves_its_index() {
    let platform_version = PlatformVersion::latest();
    let contract = widget_contract();
    let drive = populated_drive(&contract, platform_version);
    let document_type = contract.document_type_for_name("widget").expect("widget");
    let config = DriveConfig::default();
    let refusal = |e: crate::error::Error| format!("{e:?}");
    let (mut sums_reached, mut averages_reached) = (BTreeSet::new(), BTreeSet::new());
    for where_clauses in shapes() {
        for mode in MODES {
            for limit in [None, Some(3u32)] {
                for order_clauses in orders() {
                    let canonical = || {
                        validate_and_canonicalize_where_clauses(
                            where_clauses.clone(),
                            platform_version,
                        )
                        .map_err(refusal)
                    };

                    let dispatched = match drive.execute_document_sum_request(
                        DocumentSumRequest {
                            contract: &contract,
                            document_type,
                            sum_property: "amount".to_string(),
                            where_clauses: where_clauses.clone(),
                            resolved_time_ranges: vec![],
                            order_clauses: order_clauses.clone(),
                            mode: SumMode::from(mode),
                            limit,
                            prove: true,
                            drive_config: &config,
                        },
                        None,
                        platform_version,
                    ) {
                        Ok(DocumentSumResponse::Proof(proof)) => Ok(proof),
                        Ok(other) => panic!("a proved sum answered {other:?}"),
                        Err(e) => Err(refusal(e)),
                    };
                    let by_mode = canonical().and_then(|canonical| {
                        let proof_mode = detect_sum_mode_from_inputs(
                            &canonical,
                            SumMode::from(mode),
                            true,
                            platform_version,
                        )
                        .map_err(refusal)?;
                        let plan = DriveDocumentSumQuery::proof_plan(
                            &document_type,
                            &canonical,
                            &[],
                            "amount",
                            proof_mode,
                            &order_clauses,
                            limit,
                            Some(config.max_query_limit),
                        )
                        .map_err(refusal)?;
                        sum_plan_by_mode(
                            &drive,
                            &contract,
                            document_type,
                            &canonical,
                            plan,
                            false,
                            platform_version,
                        )
                        .map_err(refusal)
                    });
                    if let Ok((_, variant)) = &by_mode {
                        sums_reached.insert(*variant);
                    }
                    assert_eq!(
                        dispatched,
                        by_mode.map(|(proof, _)| proof),
                        "sum {where_clauses:?} {mode:?} {limit:?} {order_clauses:?}"
                    );

                    let dispatched = match drive.execute_document_average_request(
                        DocumentAverageRequest {
                            contract: &contract,
                            document_type,
                            sum_property: "amount".to_string(),
                            where_clauses: where_clauses.clone(),
                            resolved_time_ranges: vec![],
                            order_clauses: order_clauses.clone(),
                            mode: AverageMode::from(mode),
                            limit,
                            prove: true,
                            drive_config: &config,
                        },
                        None,
                        platform_version,
                    ) {
                        Ok(DocumentAverageResponse::Proof(proof)) => Ok(proof),
                        Ok(other) => panic!("a proved average answered {other:?}"),
                        Err(e) => Err(refusal(e)),
                    };
                    let by_mode = canonical().and_then(|canonical| {
                        let plan = DriveDocumentSumQuery::average_proof_plan(
                            &document_type,
                            &canonical,
                            &[],
                            "amount",
                            AverageMode::from(mode),
                            &order_clauses,
                            limit,
                            Some(config.max_query_limit),
                        )
                        .map_err(refusal)?;
                        sum_plan_by_mode(
                            &drive,
                            &contract,
                            document_type,
                            &canonical,
                            plan,
                            true,
                            platform_version,
                        )
                        .map_err(refusal)
                    });
                    if let Ok((_, variant)) = &by_mode {
                        averages_reached.insert(*variant);
                    }
                    assert_eq!(
                        dispatched,
                        by_mode.map(|(proof, _)| proof),
                        "avg {where_clauses:?} {mode:?} {limit:?} {order_clauses:?}"
                    );
                }
            }
        }
    }
    assert_eq!(sums_reached, BTreeSet::from(EVERY_PLAN));
    assert_eq!(averages_reached, BTreeSet::from(EVERY_PLAN));
}
