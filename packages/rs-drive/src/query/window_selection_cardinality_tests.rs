//! The refusal of a request carrying two window selections, as each server path that refuses
//! it words it. A transform's source is its index's first property, so an index buckets one
//! field and no index serves two windows; the documents query, the ranked dispatcher and the
//! having dispatcher refuse such a request before routing it, and a client refuses it before
//! sending in the same words. These texts reach callers, so they are pinned byte for byte.

use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::having::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
};
use crate::query::projection::SelectProjection;
use crate::query::{
    refuse_several_window_selections, resolve_integer_range_bucket_clause,
    resolve_time_range_bucket_clause, DocumentHavingRequest, DocumentRankedRequest,
    DriveDocumentQuery, OrderClause, ResolvedTimeRange, TimeRangeSelector,
};
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContractFactory;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;

/// A `listing` whose `price` is bucketed in windows of 100 and whose `$createdAt` in daily
/// windows, each under its own index, so a window on each field resolves.
fn windowed_contract() -> DataContract {
    let schemas = platform_value!({"listing": {
        "type": "object",
        "properties": {
            "price": {"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 0},
            "category": {"type": "string", "minLength": 1, "maxLength": 63, "position": 1}
        },
        "required": ["price", "category", "$createdAt"],
        "indices": [
            {"name": "byPriceBand",
             "properties": [{"price": "asc"}, {"category": "asc"}],
             "integerRange": {"on": "price", "range": 100u64, "step": 100u64},
             "countable": "countable"},
            {"name": "byDay",
             "properties": [{"$createdAt": "asc"}, {"category": "asc"}],
             "timeRange": {"on": "$createdAt", "range": 86400, "step": 86400},
             "countable": "countable"}
        ],
        "additionalProperties": false
    }});
    DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::from([203u8; 32]), 0, schemas, None, None)
        .expect("the listing contract is well-formed")
        .data_contract_owned()
}

/// One resolved window on `price` and one on `$createdAt`.
fn two_windows(contract: &DataContract) -> Vec<ResolvedTimeRange> {
    let document_type = contract
        .document_type_for_name("listing")
        .expect("the contract has this document type");
    let (_, price) =
        resolve_integer_range_bucket_clause("price", &Value::U64(200), None, document_type)
            .expect("a price window resolves");
    let (_, created_at) = resolve_time_range_bucket_clause(
        "$createdAt",
        TimeRangeSelector::Newest,
        None,
        document_type,
        1_700_000_000_000,
    )
    .expect("a day window resolves");
    vec![price, created_at]
}

/// The text a server path refuses two windows with, `query_kind` naming the query.
fn expected_refusal(query_kind: &str, resolved: &[ResolvedTimeRange]) -> String {
    format!(
        "at most one window selection (IN_TIME_RANGE or IN_INTEGER_RANGE) is supported per \
         {query_kind}; this one resolves {resolved:?}, and no single index can bucket more \
         than one field"
    )
}

fn assert_refused_in(result: Result<(), Error>, query_kind: &str, resolved: &[ResolvedTimeRange]) {
    match result {
        Err(Error::Query(QuerySyntaxError::Unsupported(text))) => {
            assert_eq!(text, expected_refusal(query_kind, resolved))
        }
        other => panic!("expected the two-window refusal per {query_kind}, got {other:?}"),
    }
}

#[test]
fn should_refuse_two_windows_on_a_documents_query_before_picking_an_index() {
    let platform_version = PlatformVersion::latest();
    let contract = windowed_contract();
    let resolved = two_windows(&contract);
    let mut query = DriveDocumentQuery::all_items_query(
        &contract,
        contract
            .document_type_for_name("listing")
            .expect("the contract has this document type"),
        None,
    );
    query.resolved_time_ranges = resolved.clone();
    assert_refused_in(
        query.find_best_index(platform_version).map(drop),
        "query",
        &resolved,
    );
}

#[test]
fn should_refuse_two_windows_on_a_ranked_or_having_request_before_routing_it() {
    let platform_version = PlatformVersion::latest();
    let drive: Drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = windowed_contract();
    let resolved = two_windows(&contract);
    let document_type = contract
        .document_type_for_name("listing")
        .expect("the contract has this document type");
    let group_by = vec!["category".to_string()];
    let order_by = vec![OrderClause {
        field: "$count".to_string(),
        ascending: false,
    }];

    let ranked = drive.execute_document_ranked_request(
        DocumentRankedRequest {
            contract: &contract,
            document_type,
            group_by: &group_by,
            select: SelectProjection::count_star(),
            having: &[],
            order_by: &order_by,
            where_clauses: &[],
            limit: Some(5),
            offset: None,
            has_start_at: false,
            prove: true,
            resolved_time_ranges: &resolved,
        },
        None,
        platform_version,
    );
    assert_refused_in(ranked.map(drop), "ranked query", &resolved);

    let having = [HavingClause {
        aggregate: HavingAggregate {
            function: HavingAggregateFunction::Count,
            field: String::new(),
        },
        operator: HavingOperator::GreaterThan,
        right: HavingRightOperand::Value(Value::U64(1)),
    }];
    let having = drive.execute_document_having_request(
        DocumentHavingRequest {
            contract: &contract,
            document_type,
            group_by: &group_by,
            select: SelectProjection::count_star(),
            having: &having,
            order_by: &[],
            where_clauses: &[],
            resolved_time_ranges: &resolved,
            limit: Some(5),
            offset: None,
            has_start_at: false,
            prove: true,
        },
        None,
        platform_version,
    );
    assert_refused_in(having.map(drop), "having-range query", &resolved);
}

/// Each server path refuses through the one helper; its words for each query kind are the ones
/// the paths above refuse with.
#[test]
fn should_word_the_refusal_of_two_windows_by_the_query_kind() {
    let contract = windowed_contract();
    let resolved = two_windows(&contract);
    for query_kind in ["query", "ranked query", "having-range query"] {
        assert_refused_in(
            refuse_several_window_selections(&resolved, query_kind),
            query_kind,
            &resolved,
        );
    }
    refuse_several_window_selections(&resolved[..1], "query")
        .expect("one window is served by the index that buckets it");
    refuse_several_window_selections(&[], "query").expect("a request without windows");
}
