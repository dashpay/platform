//! [`FromProof::check_request`] for every view fetched with a [`DocumentQuery`].
//!
//! All the views share one request type and one wire encoding, so a query
//! built for one view can be fetched as another. Some or all nodes serve
//! such a query as what it asks for, and the view's verifier then refuses
//! the answer for the request alone, which the SDK reports as a failed proof
//! and holds against the node. `check_request` must refuse those queries
//! before they are sent (refusing one that every node refuses only moves the
//! error), and must accept every query the view can be verified from.

use crate::documents::composite_document_query::CompositeSubQuery;
use crate::documents::document_query::{DocumentQuery, RankingDirection};
use dapi_grpc::platform::v0::get_documents_response::{
    self, get_documents_response_v0, GetDocumentsResponseV0,
};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::dashcore::Network;
use dpp::data_contract::{DataContract, DataContractFactory, TokenConfiguration};
use dpp::document::Document;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::CoreBlockHeight;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
    SelectFunction, SelectProjection, TimeRangeSelector, WhereClause, WhereOperator,
};
use drive_proof_verifier::types::Documents;
use drive_proof_verifier::{
    CompositeDocuments, DocumentAverage, DocumentCount, DocumentHavingEntries,
    DocumentRankedEntries, DocumentSplitAverages, DocumentSplitCounts, DocumentSplitSums,
    DocumentSum, Error, FromProof,
};
use std::sync::Arc;

type Check = fn(&DocumentQuery) -> Result<(), Error>;

fn check<T: FromProof<DocumentQuery, Request = DocumentQuery>>(
    query: &DocumentQuery,
) -> Result<(), Error> {
    T::check_request(query, PlatformVersion::latest())
}

/// The count, sum and average views check shapes only, so the fixture's
/// document type serves them; whether an index covers the query is the server's
/// call, and the server refuses a query no index covers. The ranked and
/// having-range views also check that an index serves the query, so their
/// queries use [`ranked_contract`].
fn contract() -> Arc<DataContract> {
    Arc::new(
        get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
            .data_contract_owned(),
    )
}

fn query() -> DocumentQuery {
    DocumentQuery::new(contract(), "niceDocument").expect("the fixture has this document type")
}

/// A `review` whose indexes rank restaurants by their average grade and hashtags by how many
/// reviews carry them, so ranked and having-range queries over it are served.
fn ranked_contract() -> Arc<DataContract> {
    let schemas = platform_value!({"review": {
        "type": "object",
        "properties": {
            "restaurantId": {"type": "string", "minLength": 1, "maxLength": 59, "position": 0},
            "grade": {"type": "integer", "minimum": 0, "maximum": 100, "position": 1},
            "hashtag": {"type": "string", "minLength": 1, "maxLength": 59, "position": 2}
        },
        "required": ["restaurantId", "grade", "hashtag"],
        "indices": [
            {"name": "byRestaurant",
             "properties": [{"restaurantId": "asc"}],
             "averageable": "grade",
             "rangeAverageable": true,
             "rankedAverageable": true},
            {"name": "byHashtag",
             "properties": [{"hashtag": "asc"}],
             "countable": "countable",
             "rangeCountable": true,
             "rankedCountable": true}
        ],
        "additionalProperties": false
    }});
    Arc::new(
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(Identifier::new([9u8; 32]), 0, schemas, None, None)
            .expect("the review contract is well-formed")
            .data_contract_owned(),
    )
}

fn reviews() -> DocumentQuery {
    DocumentQuery::new(ranked_contract(), "review").expect("the contract has this document type")
}

/// `SELECT AVG(grade) GROUP BY restaurantId ORDER BY grade DESC LIMIT 5`.
fn ranked() -> DocumentQuery {
    reviews()
        .with_select(SelectProjection::avg("grade"))
        .with_group_by("restaurantId")
        .order_by_selected_aggregate(RankingDirection::Descending)
        .with_limit(5)
}

/// `SELECT COUNT(*) GROUP BY hashtag HAVING $count > 100 LIMIT 100`.
fn having() -> DocumentQuery {
    reviews()
        .with_select(SelectProjection::count_star())
        .with_group_by("hashtag")
        .with_having(vec![HavingClause {
            aggregate: HavingAggregate {
                function: HavingAggregateFunction::Count,
                field: String::new(),
            },
            operator: HavingOperator::GreaterThan,
            right: HavingRightOperand::Value(Value::U64(100)),
        }])
        .with_limit(100)
}

/// A grouped aggregate is answered per group only when an `In` or a range
/// constrains the group.
fn restaurants_in() -> WhereClause {
    WhereClause {
        field: "restaurantId".to_string(),
        operator: WhereOperator::In,
        value: platform_value!(["a", "b"]),
    }
}

fn sub_query() -> CompositeSubQuery {
    CompositeSubQuery::documents(contract(), "niceDocument")
        .expect("the fixture has this document type")
        .with_limit(10)
}

fn composite() -> DocumentQuery {
    query().with_limit(10).with_sub_query(sub_query())
}

/// Every view but [`CompositeDocuments`], each with a query it is verified from.
fn plain_views() -> Vec<(&'static str, Check, DocumentQuery)> {
    vec![
        ("Document", check::<Document>, query()),
        ("Documents", check::<Documents>, query()),
        (
            "DocumentCount",
            check::<DocumentCount>,
            query().with_select(SelectProjection::count_star()),
        ),
        (
            "DocumentSplitCounts",
            check::<DocumentSplitCounts>,
            query()
                .with_select(SelectProjection::count_star())
                .with_where(restaurants_in())
                .with_group_by("restaurantId"),
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            query().with_select(SelectProjection::sum("grade")),
        ),
        (
            "DocumentSplitSums",
            check::<DocumentSplitSums>,
            query()
                .with_select(SelectProjection::sum("grade"))
                .with_where(restaurants_in())
                .with_group_by("restaurantId"),
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            query().with_select(SelectProjection::avg("grade")),
        ),
        (
            "DocumentSplitAverages",
            check::<DocumentSplitAverages>,
            query()
                .with_select(SelectProjection::avg("grade"))
                .with_where(restaurants_in())
                .with_group_by("restaurantId"),
        ),
        (
            "DocumentRankedEntries",
            check::<DocumentRankedEntries>,
            ranked(),
        ),
        (
            "DocumentHavingEntries",
            check::<DocumentHavingEntries>,
            having(),
        ),
    ]
}

fn refusal(view: &str, check: Check, query: &DocumentQuery) -> String {
    match check(query) {
        Err(Error::RequestError { error }) => error,
        other => panic!("{view} must refuse {query:?} before sending it, got {other:?}"),
    }
}

/// Guards every refusal test below: a view must keep accepting the shapes it
/// is verified from. The fixture cannot hold these queries' data, so this pins
/// the shapes only; the drive-abci proof round trips check that every query
/// verified from a real signed answer passes the check too.
#[test]
fn should_accept_the_shape_of_every_query_its_view_is_verified_from() {
    let mut views = plain_views();
    views.push((
        "CompositeDocuments",
        check::<CompositeDocuments>,
        composite(),
    ));
    for (view, check, query) in views {
        assert!(check(&query).is_ok(), "{view}: {:?}", check(&query));
    }
}

#[test]
fn should_refuse_sub_queries_for_every_view_but_composite_documents() {
    for (view, check, query) in plain_views() {
        let error = refusal(view, check, &query.with_sub_query(sub_query()));
        assert!(
            error.contains("cannot return sub-queries"),
            "{view}: {error}"
        );
    }
}

/// Each select is served as it asks; a view of another aggregate, or of
/// documents, can never verify the answer.
#[test]
fn should_refuse_a_view_of_a_query_that_selects_something_else() {
    let documents = SelectProjection::documents();
    let count = SelectProjection::count_star();
    let sum = SelectProjection::sum("grade");
    let avg = SelectProjection::avg("grade");
    let views: [(&str, Check, &SelectProjection); 8] = [
        ("Document", check::<Document>, &documents),
        ("Documents", check::<Documents>, &documents),
        ("DocumentCount", check::<DocumentCount>, &count),
        ("DocumentSplitCounts", check::<DocumentSplitCounts>, &count),
        ("DocumentSum", check::<DocumentSum>, &sum),
        ("DocumentSplitSums", check::<DocumentSplitSums>, &sum),
        ("DocumentAverage", check::<DocumentAverage>, &avg),
        (
            "DocumentSplitAverages",
            check::<DocumentSplitAverages>,
            &avg,
        ),
    ];
    for (view, check, own) in views {
        for select in [&documents, &count, &sum, &avg] {
            if select != own {
                refusal(view, check, &query().with_select(select.clone()));
            }
        }
    }
}

/// A documents select returns the documents themselves: a grouping or a `having`
/// clause has no meaning for it, and every node refuses it.
#[test]
fn should_refuse_a_documents_select_grouped_or_filtered_by_an_aggregate() {
    let views: [(&str, Check); 2] = [
        ("Document", check::<Document>),
        ("Documents", check::<Documents>),
    ];
    for (view, check) in views {
        for query in [
            query().with_group_by("restaurantId"),
            query().with_having(having().having),
        ] {
            let error = refusal(view, check, &query);
            assert!(error.contains("cannot be grouped"), "{view}: {error}");
        }
    }
}

/// A `tip` whose documents are summed by `amount`.
fn tip_contract() -> Arc<DataContract> {
    let schemas = platform_value!({"tip": {
        "type": "object",
        "documentsSummable": "amount",
        "properties": {
            "recipient": {"type": "string", "minLength": 1, "maxLength": 63, "position": 0},
            "amount": {"type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 1},
            "sentAt": {"type": "integer", "minimum": 0, "position": 2}
        },
        "required": ["recipient", "amount", "sentAt"],
        "indices": [
            {"name": "byRecipient",
             "properties": [{"recipient": "asc"}],
             "summable": "amount"}
        ],
        "additionalProperties": false
    }});
    Arc::new(
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(Identifier::new([8u8; 32]), 0, schemas, None, None)
            .expect("the tip contract is well-formed")
            .data_contract_owned(),
    )
}

/// A document type summed by one property keeps no totals of another, so every node
/// refuses a sum of another property, whatever the where clauses and indexes, with an
/// internal error: the SDK would ask each node in turn and ban it for the answer.
#[test]
fn should_refuse_a_sum_of_another_property_than_the_type_sums_its_documents_by() {
    let sum_of = |field: &str| {
        DocumentQuery::new(tip_contract(), "tip")
            .expect("the contract has this document type")
            .with_select(SelectProjection::sum(field))
    };
    let recipients_in = WhereClause {
        field: "recipient".to_string(),
        operator: WhereOperator::In,
        value: platform_value!(["a", "b"]),
    };
    let split = |field: &str| {
        sum_of(field)
            .with_where(recipients_in.clone())
            .with_group_by("recipient")
    };
    let views: [(&str, Check, DocumentQuery, DocumentQuery); 2] = [
        (
            "DocumentSum",
            check::<DocumentSum>,
            sum_of("sentAt"),
            sum_of("amount"),
        ),
        (
            "DocumentSplitSums",
            check::<DocumentSplitSums>,
            split("sentAt"),
            split("amount"),
        ),
    ];
    for (view, check, other_property, own_property) in views {
        let error = refusal(view, check, &other_property);
        assert!(
            error.contains("summed by amount, so no node sums sentAt"),
            "{view}: {error}"
        );
        assert!(
            check(&own_property).is_ok(),
            "{view}: {:?}",
            check(&own_property)
        );
    }
}

/// A node routing with an older query table reads only the order direction
/// of a ranked request, so it serves one outside the ranked grammar as a
/// plain grouped aggregate.
#[test]
fn should_refuse_a_ranked_query_outside_the_ranked_grammar() {
    let range_on_the_group = ranked().with_where(WhereClause {
        field: "restaurantId".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("a".to_string()),
    });
    let without_limit = ranked().with_limit(0);
    for query in [range_on_the_group, without_limit] {
        let error = refusal(
            "DocumentRankedEntries",
            check::<DocumentRankedEntries>,
            &query,
        );
        assert!(error.contains("not a well-formed ranked query"), "{error}");
    }
}

/// Without a `having` clause the request is an ordinary grouped aggregate,
/// which nodes serve.
#[test]
fn should_refuse_a_having_view_without_a_having_clause() {
    let error = refusal(
        "DocumentHavingEntries",
        check::<DocumentHavingEntries>,
        &having().with_having(vec![]),
    );
    assert!(
        error.contains("not a well-formed having-range query"),
        "{error}"
    );
}

/// From protocol version 14 a grouped aggregate ordered by its aggregate is
/// served as a ranked answer, and one with a `having` clause as a having-range
/// answer. Neither is a grouped total a count, sum or average view verifies,
/// and its verifier refuses the request outright: a node serves a ranked or
/// having-range request only when no `In` or range constrains its group, since
/// the where clauses pin the properties before the grouped one.
#[test]
fn should_refuse_an_aggregate_view_of_a_ranked_or_having_query() {
    let views: [(&str, Check, SelectProjection); 6] = [
        (
            "DocumentCount",
            check::<DocumentCount>,
            SelectProjection::count_star(),
        ),
        (
            "DocumentSplitCounts",
            check::<DocumentSplitCounts>,
            SelectProjection::count_star(),
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            SelectProjection::sum("grade"),
        ),
        (
            "DocumentSplitSums",
            check::<DocumentSplitSums>,
            SelectProjection::sum("grade"),
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            SelectProjection::avg("grade"),
        ),
        (
            "DocumentSplitAverages",
            check::<DocumentSplitAverages>,
            SelectProjection::avg("grade"),
        ),
    ];
    for (view, check, select) in views {
        let ranked = query()
            .with_select(select.clone())
            .with_group_by("restaurantId")
            .order_by_selected_aggregate(RankingDirection::Descending)
            .with_limit(5);
        for query in [ranked, having().with_select(select)] {
            let error = refusal(view, check, &query);
            assert!(
                error.contains("not constrained by an `In` or range"),
                "{view}: {error}"
            );
        }
    }
}

/// A [`ContextProvider`] for a verifier that refuses the request before it
/// reads the proof.
struct NoProvider;

impl ContextProvider for NoProvider {
    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        unreachable!("the request is refused first")
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        unreachable!("the request is refused first")
    }

    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        unreachable!("the request is refused first")
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        unreachable!("the request is refused first")
    }
}

/// A caller reading the error gets the same text whether the query was
/// refused before it was sent or its answer was refused after.
#[test]
fn should_refuse_a_composite_shape_in_the_same_words_before_and_after_an_answer() {
    let query = query().with_limit(10);
    let before = refusal("CompositeDocuments", check::<CompositeDocuments>, &query);
    let after = <CompositeDocuments as FromProof<DocumentQuery>>::maybe_from_proof_with_metadata(
        query,
        GetDocumentsResponse::default(),
        Network::Testnet,
        PlatformVersion::latest(),
        &NoProvider,
    );
    match after {
        Err(Error::RequestError { error }) => assert_eq!(error, before),
        other => panic!("the verifier must refuse the request, got {other:?}"),
    }
}

/// Without sub-queries the encoder sends a plain page, which nodes serve.
#[test]
fn should_refuse_composite_documents_without_sub_queries() {
    let error = refusal(
        "CompositeDocuments",
        check::<CompositeDocuments>,
        &query().with_limit(10),
    );
    assert!(error.contains("sub-queries"), "{error}");
    assert!(!error.contains("SDK misconfigured"), "{error}");
}

/// The composite shape check reports more than configuration errors; each
/// reaches the caller as the same request error.
#[test]
fn should_refuse_a_composite_sub_query_outside_the_composite_shape() {
    let error = refusal(
        "CompositeDocuments",
        check::<CompositeDocuments>,
        &query()
            .with_limit(10)
            .with_sub_query(sub_query().with_limit(0)),
    );
    assert!(error.contains("sub-query 0: limit"), "{error}");
}

/// A window that names no window of a grid the contract declares (a start off the grid or
/// not an integer, or a field no index buckets) is refused by the verifier for every answer,
/// so it is refused before sending, in the verifier's words. An integer-range selection, and a
/// time-range selection by start, resolve from the query alone. A newest or oldest window
/// resolves at the answer's block time, but at any block time past the grid's phase, which
/// contract validation keeps under a year, it fails only for the grid.
#[test]
fn should_refuse_a_window_that_does_not_resolve_before_sending() {
    let integer = "failed to resolve integer range clause";
    let time = "failed to resolve time range clause";
    let off_grid_day = TimeRangeSelector::ByStart {
        start_ms: 1_700_006_400_001,
    };
    let cases: Vec<(&str, Check, DocumentQuery, &str)> = vec![
        (
            "DocumentCount",
            check::<DocumentCount>,
            listings()
                .with_select(SelectProjection::count_star())
                .with_integer_range("price", Value::U64(150)),
            integer,
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            listings()
                .with_select(SelectProjection::sum("price"))
                .with_integer_range("price", Value::Text("200".to_string())),
            integer,
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            listings()
                .with_select(SelectProjection::avg("price"))
                .with_time_range("$createdAt", off_grid_day),
            time,
        ),
        (
            "DocumentCount",
            check::<DocumentCount>,
            query()
                .with_select(SelectProjection::count_star())
                .with_time_range(
                    "$createdAt",
                    TimeRangeSelector::ByStart {
                        start_ms: 1_700_006_400_000,
                    },
                ),
            time,
        ),
        (
            "DocumentRankedEntries",
            check::<DocumentRankedEntries>,
            ranked().with_integer_range("price", Value::U64(600)),
            integer,
        ),
        (
            "DocumentRankedEntries",
            check::<DocumentRankedEntries>,
            trending_ranked(off_grid_day),
            time,
        ),
        (
            "DocumentHavingEntries",
            check::<DocumentHavingEntries>,
            having().with_integer_range("grade", Value::U64(50)),
            integer,
        ),
        (
            "DocumentHavingEntries",
            check::<DocumentHavingEntries>,
            trending_having(off_grid_day),
            time,
        ),
        (
            "DocumentRankedEntries",
            check::<DocumentRankedEntries>,
            ranked().with_time_range("$createdAt", TimeRangeSelector::Newest),
            time,
        ),
        (
            "DocumentHavingEntries",
            check::<DocumentHavingEntries>,
            having().with_time_range("$createdAt", TimeRangeSelector::Oldest),
            time,
        ),
        (
            "DocumentCount",
            check::<DocumentCount>,
            listings()
                .with_select(SelectProjection::count_star())
                .with_time_range("$updatedAt", TimeRangeSelector::Newest)
                .with_integer_range("price", Value::U64(150)),
            time,
        ),
    ];
    for (view, check, query, words) in cases {
        let error = refusal(view, check, &query);
        assert!(error.contains(words), "{view} {query:?}: {error}");
    }
}

/// A window selection is resolved against the answer's block time before the
/// proof and its signature are checked, so a window that does not resolve may
/// come from a forged block time. It stays a request error, which moves the
/// request to another node and bans this one, and never a verified result no
/// node can return.
#[test]
fn should_keep_a_window_that_does_not_resolve_at_the_answers_block_time_a_request_error() {
    let query = query()
        .with_select(SelectProjection::count_star())
        .with_time_range("$createdAt", TimeRangeSelector::Newest);
    let response = GetDocumentsResponse {
        version: Some(get_documents_response::Version::V0(
            GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata {
                    time_ms: 1_700_000_000_000,
                    protocol_version: PlatformVersion::latest().protocol_version,
                    ..Default::default()
                }),
            },
        )),
    };
    let answer = <DocumentCount as FromProof<DocumentQuery>>::maybe_from_proof_with_metadata(
        query,
        response,
        Network::Testnet,
        PlatformVersion::latest(),
        &NoProvider,
    );
    assert!(
        matches!(answer, Err(Error::RequestError { .. })),
        "{answer:?}"
    );
}

/// A `post` whose `hashtags` is a typed array of strings.
fn post_contract() -> Arc<DataContract> {
    let schemas = platform_value!({"post": {
        "type": "object",
        "properties": {
            "hashtags": {
                "type": "array",
                "maxItems": 10,
                "items": {"type": "string", "minLength": 1, "maxLength": 61},
                "position": 0
            }
        },
        "required": ["$createdAt"],
        "indices": [
            {"name": "byHashtagTime",
             "properties": [{"hashtags": "asc"}, {"$createdAt": "asc"}],
             "countable": "countable"}
        ],
        "additionalProperties": false
    }});
    Arc::new(
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(Identifier::new([7u8; 32]), 0, schemas, None, None)
            .expect("the post contract is well-formed")
            .data_contract_owned(),
    )
}

/// Grouped by a typed array, a `containsAny` over several of its elements is
/// served with one total per element. A split view verifies those totals; a
/// view that adds them up into one would count a post once per element it
/// holds, so its verifier refuses the request.
#[test]
fn should_refuse_one_total_over_several_elements_of_a_typed_array() {
    let several_hashtags = |select: SelectProjection| {
        DocumentQuery::new(post_contract(), "post")
            .expect("the contract has this document type")
            .with_select(select)
            .with_where(WhereClause {
                field: "hashtags".to_string(),
                operator: WhereOperator::ContainsAny,
                value: platform_value!(["dash", "news"]),
            })
            .with_group_by("hashtags")
    };
    let folded: [(&str, Check, SelectProjection); 3] = [
        (
            "DocumentCount",
            check::<DocumentCount>,
            SelectProjection::count_star(),
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            SelectProjection::sum("grade"),
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            SelectProjection::avg("grade"),
        ),
    ];
    for (view, check, select) in folded {
        let error = refusal(view, check, &several_hashtags(select));
        assert!(
            error.contains("only answered grouped by"),
            "{view}: {error}"
        );
    }
    let split: [(&str, Check, SelectProjection); 3] = [
        (
            "DocumentSplitCounts",
            check::<DocumentSplitCounts>,
            SelectProjection::count_star(),
        ),
        (
            "DocumentSplitSums",
            check::<DocumentSplitSums>,
            SelectProjection::sum("grade"),
        ),
        (
            "DocumentSplitAverages",
            check::<DocumentSplitAverages>,
            SelectProjection::avg("grade"),
        ),
    ];
    for (view, check, select) in split {
        let query = several_hashtags(select);
        assert!(check(&query).is_ok(), "{view}: {:?}", check(&query));
        // Not grouped by the array alone, or under a range clause, the
        // elements' totals are folded, and every node refuses the request by
        // the rule the split verifier refuses it by.
        let mut ungrouped = query.clone();
        ungrouped.group_by.clear();
        let in_a_range = query.with_where(WhereClause {
            field: "$createdAt".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::U64(0),
        });
        for query in [ungrouped, in_a_range] {
            let error = refusal(view, check, &query);
            assert!(
                error.contains("only answered grouped by"),
                "{view}: {error}"
            );
        }
    }
}

/// The server refuses a projection field on the documents select over the V1
/// wire, and the V0 encoder drops it, so the documents a V0 node serves for
/// it verify; only the projected function decides the view.
#[test]
fn should_leave_a_documents_projection_field_to_the_encoder_and_the_server() {
    let query = query().with_select(SelectProjection {
        function: SelectFunction::Documents,
        field: "name".to_string(),
    });
    for (view, check) in [
        ("Document", check::<Document> as Check),
        ("Documents", check::<Documents>),
    ] {
        assert!(check(&query).is_ok(), "{view}: {:?}", check(&query));
    }
}

/// A `listing` whose `price` is bucketed in windows of 100 and whose `$createdAt` is bucketed
/// in daily windows, each under a countable index, so window selections on it resolve.
fn windowed_contract() -> Arc<DataContract> {
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
    Arc::new(
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(Identifier::new([11u8; 32]), 0, schemas, None, None)
            .expect("the listing contract is well-formed")
            .data_contract_owned(),
    )
}

fn listings() -> DocumentQuery {
    DocumentQuery::new(windowed_contract(), "listing").expect("the contract has this type")
}

/// A documents select resolves its windows before sending as its verifier resolves them from
/// the answer: a window that does not resolve, or an `In` or range beside it on the field it
/// buckets, is refused for every answer, so it is refused before sending. The window alone is
/// sent.
#[test]
fn should_refuse_a_documents_select_whose_window_every_verifier_refuses_before_sending() {
    let price_above_one = WhereClause {
        field: "price".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::U64(1),
    };
    let refused = [
        (
            listings().with_integer_range("price", Value::U64(150)),
            "failed to resolve integer range clause",
        ),
        (
            listings().with_time_range(
                "$createdAt",
                TimeRangeSelector::ByStart {
                    start_ms: 1_700_006_400_001,
                },
            ),
            "failed to resolve time range clause",
        ),
        (
            query().with_time_range("$createdAt", TimeRangeSelector::Newest),
            "failed to resolve time range clause",
        ),
        (
            listings()
                .with_integer_range("price", Value::U64(200))
                .with_where(price_above_one),
            "invalid window selection query shape",
        ),
    ];
    let views: [(&str, Check); 2] = [
        ("Document", check::<Document>),
        ("Documents", check::<Documents>),
    ];
    for (view, check) in views {
        for (query, words) in &refused {
            let error = refusal(view, check, query);
            assert!(error.contains(words), "{view} {query:?}: {error}");
        }
        for window in [
            listings().with_integer_range("price", Value::U64(200)),
            listings().with_time_range("$createdAt", TimeRangeSelector::Newest),
        ] {
            assert!(check(&window).is_ok(), "{view}: {:?}", check(&window));
        }
    }
}

/// An index buckets one field, so no index serves windows on two fields, even when each
/// window resolves: every node refuses such a query, and so does every verifier's index pick.
/// It is refused before sending.
#[test]
fn should_refuse_windows_on_two_fields_before_sending() {
    let two_windows = listings()
        .with_integer_range("price", Value::U64(200))
        .with_time_range("$createdAt", TimeRangeSelector::Newest);
    let views: [(&str, Check, SelectProjection); 4] = [
        (
            "Documents",
            check::<Documents>,
            SelectProjection::documents(),
        ),
        (
            "DocumentCount",
            check::<DocumentCount>,
            SelectProjection::count_star(),
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            SelectProjection::sum("price"),
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            SelectProjection::avg("price"),
        ),
    ];
    for (view, check, select) in views {
        let error = refusal(view, check, &two_windows.clone().with_select(select));
        assert!(
            error.contains("at most one window selection"),
            "{view}: {error}"
        );
    }
}

/// A query whose contract lacks its document type is refused by the verifier for every answer,
/// in the same words, when it carries a window to resolve.
#[test]
fn should_refuse_a_window_on_a_document_type_the_contract_lacks_before_sending() {
    let mut query = listings().with_integer_range("price", Value::U64(200));
    query.document_type_name = "missing".to_string();
    let views: [(&str, Check, SelectProjection); 3] = [
        ("Document", check::<Document>, SelectProjection::documents()),
        (
            "Documents",
            check::<Documents>,
            SelectProjection::documents(),
        ),
        (
            "DocumentCount",
            check::<DocumentCount>,
            SelectProjection::count_star(),
        ),
    ];
    for (view, check, select) in views {
        let error = refusal(view, check, &query.clone().with_select(select));
        assert!(
            error.contains("document type not found for window selection query"),
            "{view}: {error}"
        );
    }
}

/// A `having` clause filters the groups of a grouped aggregate. Without a grouping every node
/// refuses it, whatever its protocol version, so the count, sum and average views and their
/// split views refuse it before sending.
#[test]
fn should_refuse_a_having_clause_without_a_grouping_before_sending() {
    let having = vec![HavingClause {
        aggregate: HavingAggregate {
            function: HavingAggregateFunction::Count,
            field: String::new(),
        },
        operator: HavingOperator::GreaterThan,
        right: HavingRightOperand::Value(Value::U64(1)),
    }];
    let views: [(&str, Check, SelectProjection); 6] = [
        (
            "DocumentCount",
            check::<DocumentCount>,
            SelectProjection::count_star(),
        ),
        (
            "DocumentSplitCounts",
            check::<DocumentSplitCounts>,
            SelectProjection::count_star(),
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            SelectProjection::sum("grade"),
        ),
        (
            "DocumentSplitSums",
            check::<DocumentSplitSums>,
            SelectProjection::sum("grade"),
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            SelectProjection::avg("grade"),
        ),
        (
            "DocumentSplitAverages",
            check::<DocumentSplitAverages>,
            SelectProjection::avg("grade"),
        ),
    ];
    for (view, check, select) in views {
        let query = query().with_select(select).with_having(having.clone());
        let error = refusal(view, check, &query);
        assert!(error.contains("needs a group by"), "{view}: {error}");
        assert!(
            check(&query.clone().with_having(Vec::new())).is_ok(),
            "{view}: the aggregate without its having clause is sent"
        );
    }
}

/// A field a window selection buckets may carry only the equality the selection resolves to.
/// An `In` or a range beside it is refused by every verifier, at whatever block time the
/// answer is signed, so it is refused before sending too.
#[test]
fn should_refuse_an_in_or_range_beside_a_window_on_the_same_field_before_sending() {
    let beside = |query: DocumentQuery, field: &str, operator: WhereOperator, value: Value| {
        query.with_where(WhereClause {
            field: field.to_string(),
            operator,
            value,
        })
    };
    let windows = [
        (
            listings().with_integer_range("price", Value::U64(200)),
            "price",
        ),
        (
            listings().with_time_range("$createdAt", TimeRangeSelector::Newest),
            "$createdAt",
        ),
    ];
    let views: [(&str, Check, SelectProjection); 3] = [
        (
            "DocumentCount",
            check::<DocumentCount>,
            SelectProjection::count_star(),
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            SelectProjection::sum("price"),
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            SelectProjection::avg("price"),
        ),
    ];
    for (window, field) in windows {
        for (view, check, select) in &views {
            for (operator, value) in [
                (WhereOperator::GreaterThan, Value::U64(1)),
                (WhereOperator::In, platform_value!([1u64, 2u64])),
            ] {
                let query =
                    beside(window.clone(), field, operator, value).with_select(select.clone());
                let error = refusal(view, *check, &query);
                assert!(
                    error.contains("invalid window selection query shape"),
                    "{view} {field} {operator:?}: {error}"
                );
            }
            assert!(
                check(&window.clone().with_select(select.clone())).is_ok(),
                "{view} {field}: the window alone is sent"
            );
        }
    }
}

/// A ranked or having-range query no index of the contract serves is refused by every node
/// and every verifier, so it is refused before sending.
#[test]
fn should_refuse_a_ranked_or_having_query_no_index_serves_before_sending() {
    let uncovered_ranked = query()
        .with_select(SelectProjection::avg("grade"))
        .with_group_by("restaurantId")
        .order_by_selected_aggregate(RankingDirection::Descending)
        .with_limit(5);
    let error = refusal(
        "DocumentRankedEntries",
        check::<DocumentRankedEntries>,
        &uncovered_ranked,
    );
    assert!(error.contains("cannot serve this ranked query"), "{error}");
    let mut uncovered_having = having();
    uncovered_having.group_by = vec!["restaurantId".to_string()];
    let error = refusal(
        "DocumentHavingEntries",
        check::<DocumentHavingEntries>,
        &uncovered_having,
    );
    assert!(
        error.contains("cannot serve this having-range query"),
        "{error}"
    );
}

/// A `post` whose index ranks hashtags by how many posts carry them within each day's window
/// of `$createdAt`, so ranked and having-range queries pinned to a window are served.
fn windowed_ranked_contract() -> Arc<DataContract> {
    let schemas = platform_value!({"post": {
        "type": "object",
        "properties": {
            "hashtag": {"type": "string", "minLength": 1, "maxLength": 59, "position": 0}
        },
        "required": ["hashtag", "$createdAt"],
        "indices": [
            {"name": "trending",
             "properties": [{"$createdAt": "asc"}, {"hashtag": "asc"}],
             "timeRange": {"on": "$createdAt", "range": 86400, "step": 86400},
             "countable": "countable",
             "rangeCountable": true,
             "rankedCountable": true}
        ],
        "additionalProperties": false
    }});
    Arc::new(
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(Identifier::new([12u8; 32]), 0, schemas, None, None)
            .expect("the post contract is well-formed")
            .data_contract_owned(),
    )
}

fn trending(selector: TimeRangeSelector) -> DocumentQuery {
    DocumentQuery::new(windowed_ranked_contract(), "post")
        .expect("the contract has this document type")
        .with_select(SelectProjection::count_star())
        .with_group_by("hashtag")
        .with_time_range("$createdAt", selector)
}

fn trending_ranked(selector: TimeRangeSelector) -> DocumentQuery {
    trending(selector)
        .order_by_selected_aggregate(RankingDirection::Descending)
        .with_limit(5)
}

fn trending_having(selector: TimeRangeSelector) -> DocumentQuery {
    trending(selector)
        .with_having(vec![HavingClause {
            aggregate: HavingAggregate {
                function: HavingAggregateFunction::Count,
                field: String::new(),
            },
            operator: HavingOperator::GreaterThan,
            right: HavingRightOperand::Value(Value::U64(1)),
        }])
        .with_limit(5)
}

/// A ranked or having-range query pinned to a window is served by the index that buckets the
/// window, so it is sent: the check resolves the window and hands its provenance to the index
/// pick, as the verifier does.
#[test]
fn should_send_a_ranked_or_having_query_pinned_to_a_window_its_index_buckets() {
    for selector in [
        TimeRangeSelector::Newest,
        TimeRangeSelector::Oldest,
        TimeRangeSelector::ByStart {
            start_ms: 1_700_006_400_000,
        },
    ] {
        let ranked = trending_ranked(selector);
        assert!(
            check::<DocumentRankedEntries>(&ranked).is_ok(),
            "{selector:?}: {:?}",
            check::<DocumentRankedEntries>(&ranked)
        );
        let having = trending_having(selector);
        assert!(
            check::<DocumentHavingEntries>(&having).is_ok(),
            "{selector:?}: {:?}",
            check::<DocumentHavingEntries>(&having)
        );
    }
}

/// A ranked query whose window resolves is still checked against the ranked grammar.
#[test]
fn should_refuse_a_malformed_ranked_query_pinned_to_a_window_before_sending() {
    let error = refusal(
        "DocumentRankedEntries",
        check::<DocumentRankedEntries>,
        &trending_ranked(TimeRangeSelector::Newest).with_limit(0),
    );
    assert!(error.contains("not a well-formed ranked query"), "{error}");
}

/// A verifier answer to `query`, signed at `time_ms`, whose proof is empty: a verifier that
/// accepts the request fails on the proof, so a request error is a refusal of the request.
fn verifier_refuses<
    T: FromProof<DocumentQuery, Request = DocumentQuery, Response = GetDocumentsResponse>,
>(
    query: &DocumentQuery,
    time_ms: u64,
) -> bool {
    let response = GetDocumentsResponse {
        version: Some(get_documents_response::Version::V0(
            GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata {
                    time_ms,
                    protocol_version: PlatformVersion::latest().protocol_version,
                    ..Default::default()
                }),
            },
        )),
    };
    matches!(
        T::maybe_from_proof_with_metadata(
            query.clone(),
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &NoProvider,
        ),
        Err(Error::RequestError { .. })
    )
}

type Verifies = fn(&DocumentQuery, u64) -> bool;

/// What a check refuses before sending, the verifier refuses for every answer, whatever block
/// time the answer is signed at. And a query the check accepts is one the verifier accepts at
/// some block time, so the comparison can tell the two apart.
#[test]
fn should_refuse_before_sending_only_what_the_verifier_refuses_at_any_block_time() {
    let block_times = [
        86_400_000,
        1_600_000_000_000,
        1_700_000_000_000,
        1_800_000_123_456,
        4_000_000_000_000,
    ];
    let shapes: Vec<(&str, Check, Verifies, DocumentQuery)> = vec![
        (
            "DocumentCount",
            check::<DocumentCount>,
            verifier_refuses::<DocumentCount>,
            listings()
                .with_select(SelectProjection::count_star())
                .with_integer_range("price", Value::U64(200)),
        ),
        (
            "DocumentCount",
            check::<DocumentCount>,
            verifier_refuses::<DocumentCount>,
            listings()
                .with_select(SelectProjection::count_star())
                .with_integer_range("price", Value::U64(200))
                .with_where(WhereClause {
                    field: "price".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: Value::U64(1),
                }),
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            verifier_refuses::<DocumentSum>,
            listings()
                .with_select(SelectProjection::sum("price"))
                .with_time_range("$createdAt", TimeRangeSelector::Newest)
                .with_where(WhereClause {
                    field: "$createdAt".to_string(),
                    operator: WhereOperator::In,
                    value: platform_value!([1u64, 2u64]),
                }),
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            verifier_refuses::<DocumentAverage>,
            listings()
                .with_select(SelectProjection::avg("price"))
                .with_time_range("$createdAt", TimeRangeSelector::Oldest),
        ),
        (
            "DocumentSplitCounts",
            check::<DocumentSplitCounts>,
            verifier_refuses::<DocumentSplitCounts>,
            listings()
                .with_select(SelectProjection::count_star())
                .with_integer_range("price", Value::U64(200))
                .with_group_by("category"),
        ),
        (
            "DocumentRankedEntries",
            check::<DocumentRankedEntries>,
            verifier_refuses::<DocumentRankedEntries>,
            trending_ranked(TimeRangeSelector::Newest),
        ),
        (
            "DocumentRankedEntries",
            check::<DocumentRankedEntries>,
            verifier_refuses::<DocumentRankedEntries>,
            trending_ranked(TimeRangeSelector::Newest).with_where(WhereClause {
                field: "$createdAt".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::U64(1),
            }),
        ),
        (
            "DocumentHavingEntries",
            check::<DocumentHavingEntries>,
            verifier_refuses::<DocumentHavingEntries>,
            trending_having(TimeRangeSelector::ByStart {
                start_ms: 1_700_006_400_000,
            }),
        ),
        (
            "DocumentRankedEntries",
            check::<DocumentRankedEntries>,
            verifier_refuses::<DocumentRankedEntries>,
            query()
                .with_select(SelectProjection::avg("grade"))
                .with_group_by("restaurantId")
                .order_by_selected_aggregate(RankingDirection::Descending)
                .with_limit(5),
        ),
        (
            "DocumentCount",
            check::<DocumentCount>,
            verifier_refuses::<DocumentCount>,
            listings()
                .with_select(SelectProjection::count_star())
                .with_integer_range("price", Value::U64(150)),
        ),
        (
            "DocumentHavingEntries",
            check::<DocumentHavingEntries>,
            verifier_refuses::<DocumentHavingEntries>,
            trending_having(TimeRangeSelector::ByStart {
                start_ms: 1_700_006_400_001,
            }),
        ),
        (
            "DocumentCount",
            check::<DocumentCount>,
            verifier_refuses::<DocumentCount>,
            query()
                .with_select(SelectProjection::count_star())
                .with_time_range("$createdAt", TimeRangeSelector::Newest),
        ),
        (
            "Documents",
            check::<Documents>,
            verifier_refuses::<Documents>,
            listings().with_integer_range("price", Value::U64(150)),
        ),
        (
            "Documents",
            check::<Documents>,
            verifier_refuses::<Documents>,
            listings().with_integer_range("price", Value::U64(200)),
        ),
        (
            "DocumentCount",
            check::<DocumentCount>,
            verifier_refuses::<DocumentCount>,
            listings()
                .with_select(SelectProjection::count_star())
                .with_integer_range("price", Value::U64(200))
                .with_time_range("$createdAt", TimeRangeSelector::Newest),
        ),
    ];
    let (mut refused, mut accepted_by_both) = (0, 0);
    for (view, check, verifier_refuses, query) in &shapes {
        if check(query).is_ok() {
            if block_times
                .iter()
                .any(|time_ms| !verifier_refuses(query, *time_ms))
            {
                accepted_by_both += 1;
            }
            continue;
        }
        refused += 1;
        for time_ms in block_times {
            assert!(
                verifier_refuses(query, time_ms),
                "{view} refused {query:?} before sending, but the verifier accepts it at block \
                 time {time_ms}"
            );
        }
    }
    assert!(
        refused >= 3 && accepted_by_both >= 2,
        "{refused} refused, {accepted_by_both} accepted by both"
    );
}
