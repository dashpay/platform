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
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dapi_grpc::platform::v0::get_documents_response::{
    self, get_documents_response_v0, GetDocumentsResponseV0,
};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::{DataContract, DataContractFactory, TokenConfiguration};
use dpp::document::Document;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::CoreBlockHeight;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::{
    resolve_integer_range_bucket_clause, resolve_time_range_bucket_clause, HavingAggregate,
    HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand, OrderClause,
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

/// A contract whose `niceDocument` has no index: a documents select over it reads the
/// primary-key tree, and a count, sum or average over it is served by nothing, so the views of
/// those use [`ranked_contract`].
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
/// reviews carry them, so ranked and having-range queries over it are served. Its documents are
/// counted and summed by grade, and its restaurant index sums and counts grades, so counts, sums
/// and averages over it, in total or per restaurant, are served too.
fn ranked_contract() -> Arc<DataContract> {
    let schemas = platform_value!({"review": {
        "type": "object",
        "documentsCountable": true,
        "documentsSummable": "grade",
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
            reviews().with_select(SelectProjection::count_star()),
        ),
        (
            "DocumentSplitCounts",
            check::<DocumentSplitCounts>,
            reviews()
                .with_select(SelectProjection::count_star())
                .with_where(restaurants_in())
                .with_group_by("restaurantId"),
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            reviews().with_select(SelectProjection::sum("grade")),
        ),
        (
            "DocumentSplitSums",
            check::<DocumentSplitSums>,
            reviews()
                .with_select(SelectProjection::sum("grade"))
                .with_where(restaurants_in())
                .with_group_by("restaurantId"),
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            reviews().with_select(SelectProjection::avg("grade")),
        ),
        (
            "DocumentSplitAverages",
            check::<DocumentSplitAverages>,
            reviews()
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
    let documents_words = "require the documents projection";
    let count_words = "require `SelectProjection::count_star()`";
    let sum_words = "require `SelectProjection::sum(";
    let avg_words = "require `SelectProjection::avg(";
    let views: [(&str, Check, &SelectProjection, &str); 8] = [
        ("Document", check::<Document>, &documents, documents_words),
        ("Documents", check::<Documents>, &documents, documents_words),
        ("DocumentCount", check::<DocumentCount>, &count, count_words),
        (
            "DocumentSplitCounts",
            check::<DocumentSplitCounts>,
            &count,
            count_words,
        ),
        ("DocumentSum", check::<DocumentSum>, &sum, sum_words),
        (
            "DocumentSplitSums",
            check::<DocumentSplitSums>,
            &sum,
            sum_words,
        ),
        ("DocumentAverage", check::<DocumentAverage>, &avg, avg_words),
        (
            "DocumentSplitAverages",
            check::<DocumentSplitAverages>,
            &avg,
            avg_words,
        ),
    ];
    // Over a type that serves every one of these selects, so the refusal is the view's own.
    for (view, check, own, words) in views {
        for select in [&documents, &count, &sum, &avg] {
            if select != own {
                let error = refusal(view, check, &reviews().with_select(select.clone()));
                assert!(error.contains(words), "{view} {select:?}: {error}");
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
/// answer. Neither is a grouped total a count, sum or average view verifies, so
/// the view routes the request as every node routes it and refuses it before
/// sending, naming the view that verifies it.
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
        for (query, verified_by) in [
            (ranked, "DocumentRankedEntries"),
            (having().with_select(select), "DocumentHavingEntries"),
        ] {
            let error = refusal(view, check, &query);
            assert!(error.contains(verified_by), "{view}: {error}");
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

/// A `post` whose `hashtags` is a typed array of strings, with an index that counts posts and
/// sums their `grade` per hashtag.
fn post_contract() -> Arc<DataContract> {
    let schemas = platform_value!({"post": {
        "type": "object",
        "properties": {
            "hashtags": {
                "type": "array",
                "maxItems": 10,
                "items": {"type": "string", "minLength": 1, "maxLength": 61},
                "position": 0
            },
            "grade": {"type": "integer", "minimum": 0, "maximum": 100, "position": 1}
        },
        "required": ["grade", "$createdAt"],
        "indices": [
            {"name": "byHashtagTime",
             "properties": [{"hashtags": "asc"}, {"$createdAt": "asc"}],
             "countable": "countable"},
            {"name": "byHashtag",
             "properties": [{"hashtags": "asc"}],
             "averageable": "grade"}
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
/// in daily windows, each under an index that counts listings and sums their `quantity`, so
/// window selections on it resolve. Nothing sums `price`.
fn windowed_contract() -> Arc<DataContract> {
    let schemas = platform_value!({"listing": {
        "type": "object",
        "properties": {
            "price": {"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 0},
            "category": {"type": "string", "minLength": 1, "maxLength": 63, "position": 1},
            "quantity": {"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 2}
        },
        "required": ["price", "category", "quantity", "$createdAt"],
        "indices": [
            {"name": "byPriceBand",
             "properties": [{"price": "asc"}, {"category": "asc"}],
             "integerRange": {"on": "price", "range": 100u64, "step": 100u64},
             "averageable": "quantity"},
            {"name": "byDay",
             "properties": [{"$createdAt": "asc"}, {"category": "asc"}],
             "timeRange": {"on": "$createdAt", "range": 86400, "step": 86400},
             "averageable": "quantity"}
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
/// It is refused before sending, in Drive's own words.
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
    // The windows as a check resolves them: time-range selections first, at the block time it
    // checks at.
    let document_type = windowed_contract();
    let document_type = document_type
        .document_type_for_name("listing")
        .expect("the contract has this document type");
    let (_, created_at) = resolve_time_range_bucket_clause(
        "$createdAt",
        TimeRangeSelector::Newest,
        None,
        document_type,
        1_700_000_000_000,
    )
    .expect("a day window resolves");
    let (_, price) =
        resolve_integer_range_bucket_clause("price", &Value::U64(200), None, document_type)
            .expect("a price window resolves");
    // Drive's words for a query, naming both windows, as the node refuses a documents query.
    let drives_words = format!(
        "at most one window selection (IN_TIME_RANGE or IN_INTEGER_RANGE) is supported per \
         query; this one resolves {:?}, and no single index can bucket more than one field",
        [created_at, price]
    );
    for (view, check, select) in views {
        let error = refusal(view, check, &two_windows.clone().with_select(select));
        assert!(error.contains(&drives_words), "{view}: {error}");
    }
    for (view, check, query) in [
        (
            "DocumentRankedEntries",
            check::<DocumentRankedEntries> as Check,
            windowed_ranked(),
        ),
        (
            "DocumentHavingEntries",
            check::<DocumentHavingEntries>,
            windowed_having(),
        ),
    ] {
        let error = refusal(
            view,
            check,
            &query
                .with_integer_range("price", Value::U64(200))
                .with_time_range("$createdAt", TimeRangeSelector::Newest),
        );
        assert!(error.contains(&drives_words), "{view}: {error}");
    }
}

/// `SELECT COUNT(*) FROM listing GROUP BY category ORDER BY COUNT(*) DESC LIMIT 5`.
fn windowed_ranked() -> DocumentQuery {
    listings()
        .with_select(SelectProjection::count_star())
        .with_group_by("category")
        .order_by_selected_aggregate(RankingDirection::Descending)
        .with_limit(5)
}

/// `SELECT COUNT(*) FROM listing GROUP BY category HAVING COUNT(*) > 1 LIMIT 5`.
fn windowed_having() -> DocumentQuery {
    listings()
        .with_select(SelectProjection::count_star())
        .with_group_by("category")
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

/// The equality on `$createdAt` a caller writes, as opposed to one a window resolves to.
fn created_at_equal() -> WhereClause {
    WhereClause {
        field: "$createdAt".to_string(),
        operator: WhereOperator::Equal,
        value: Value::U64(1_700_000_000_000),
    }
}

/// A documents select is rebuilt by its verifier into Drive's path query, from the request
/// alone, before the proof is read; that path query reads one index. A query no index covers,
/// here a window on `price` beside an equality on `$createdAt`, is refused by every node and by
/// every verifier for every answer, so it is refused before sending.
#[test]
fn should_refuse_a_documents_select_no_index_covers_before_sending() {
    let uncovered = listings()
        .with_integer_range("price", Value::U64(200))
        .with_where(created_at_equal());
    let views: [(&str, Check); 2] = [
        ("Document", check::<Document>),
        ("Documents", check::<Documents>),
    ];
    for (view, check) in views {
        let error = refusal(view, check, &uncovered);
        assert!(
            error.contains("no answer to this documents query can be verified"),
            "{view}: {error}"
        );
    }
}

/// A window's resolved equality reads as an ordinary equality, and no index may bind a raw
/// value to the bucket keys of the index that buckets the field. The check hands the
/// resolution's provenance to the index pick, as the verifier does, so a windowed query every
/// node serves is sent; a check that dropped it would refuse these.
#[test]
fn should_send_a_documents_select_pinned_to_a_window_its_index_buckets() {
    let category_equal = WhereClause {
        field: "category".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text("tools".to_string()),
    };
    let views: [(&str, Check); 2] = [
        ("Document", check::<Document>),
        ("Documents", check::<Documents>),
    ];
    for (view, check) in views {
        for window in [
            listings().with_integer_range("price", Value::U64(200)),
            listings()
                .with_integer_range("price", Value::U64(200))
                .with_where(category_equal.clone()),
            listings().with_time_range("$createdAt", TimeRangeSelector::Newest),
        ] {
            assert!(
                check(&window).is_ok(),
                "{view} {window:?}: {:?}",
                check(&window)
            );
        }
        // The same equality written by the caller binds to no index, which is what the
        // provenance changes.
        let raw_price = listings().with_where(WhereClause {
            field: "price".to_string(),
            operator: WhereOperator::Equal,
            value: Value::U64(200),
        });
        let error = refusal(view, check, &raw_price);
        assert!(
            error.contains("no answer to this documents query can be verified"),
            "{view}: {error}"
        );
    }
}

/// The verifier rebuilds a cursor page's path query without the cursor document, too, to read
/// its walk direction, so the check's lowering without a cursor document refuses nothing a
/// cursor page is served for, and refuses a cursor where the verifier does.
#[test]
fn should_send_a_documents_select_with_a_cursor() {
    let with_cursor = |query: DocumentQuery| DocumentQuery {
        start: Some(Start::StartAfter([1u8; 32].to_vec())),
        ..query
    };
    let by_restaurant = || {
        reviews()
            .with_where(restaurants_in())
            .with_order_by(OrderClause {
                field: "restaurantId".to_string(),
                ascending: true,
            })
            .with_limit(10)
    };
    type VersionedCheck = fn(&DocumentQuery, &PlatformVersion) -> Result<(), Error>;
    let views: [(&str, VersionedCheck); 2] = [
        (
            "Document",
            <Document as FromProof<DocumentQuery>>::check_request,
        ),
        (
            "Documents",
            <Documents as FromProof<DocumentQuery>>::check_request,
        ),
    ];
    let v13 = PlatformVersion::get(13).expect("protocol version 13");
    for (view, check) in views {
        // The primary-key and single-index routes, at the oldest version still on a network and
        // at the latest.
        for version in [v13, PlatformVersion::latest()] {
            for query in [query().with_limit(10), by_restaurant()] {
                let query = with_cursor(query);
                assert!(
                    check(&query, version).is_ok(),
                    "{view} at {}: {:?}",
                    version.protocol_version,
                    check(&query, version)
                );
            }
        }
        // A bucketed index, which the latest version serves with a cursor.
        let windowed = with_cursor(
            listings()
                .with_integer_range("price", Value::U64(200))
                .with_limit(10),
        );
        assert!(
            check(&windowed, PlatformVersion::latest()).is_ok(),
            "{view}: {:?}",
            check(&windowed, PlatformVersion::latest())
        );
        // Two `In`s on a compound index are served without a cursor; the multiple-`In` route has
        // no cursor position, so the lowering, which reads the cursor, refuses one.
        assert!(
            check(&two_ins(), PlatformVersion::latest()).is_ok(),
            "{view}: {:?}",
            check(&two_ins(), PlatformVersion::latest())
        );
        let error = match check(&with_cursor(two_ins()), PlatformVersion::latest()) {
            Err(Error::RequestError { error }) => error,
            other => panic!("{view} must refuse a cursor over two `In`s, got {other:?}"),
        };
        assert!(
            error.contains("not supported with multiple in clauses"),
            "{view}: {error}"
        );
        // An indexOnly position has no document id a cursor can address, so the lowering, which
        // reads the cursor, refuses it as the verifier does.
        let error = match check(
            &with_cursor(index_only_entries().with_limit(10)),
            PlatformVersion::latest(),
        ) {
            Err(Error::RequestError { error }) => error,
            other => panic!("{view} must refuse an indexOnly cursor, got {other:?}"),
        };
        assert!(
            error.contains("cannot address an indexOnly"),
            "{view}: {error}"
        );
    }
}

/// A `shirt` with a compound index on `color` and `size`, queried with an `In` on each, in
/// index order.
fn two_ins() -> DocumentQuery {
    let schemas = platform_value!({"shirt": {
        "type": "object",
        "properties": {
            "color": {"type": "string", "minLength": 1, "maxLength": 31, "position": 0},
            "size": {"type": "string", "minLength": 1, "maxLength": 7, "position": 1}
        },
        "required": ["color", "size"],
        "indices": [
            {"name": "byColorSize", "properties": [{"color": "asc"}, {"size": "asc"}]}
        ],
        "additionalProperties": false
    }});
    let contract = DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::new([13u8; 32]), 0, schemas, None, None)
        .expect("the shirt contract is well-formed")
        .data_contract_owned();
    DocumentQuery::new(Arc::new(contract), "shirt")
        .expect("the contract has this document type")
        .with_where(WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["red", "blue"]),
        })
        .with_where(WhereClause {
            field: "size".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["s", "m"]),
        })
        .with_order_by(OrderClause {
            field: "color".to_string(),
            ascending: true,
        })
        .with_order_by(OrderClause {
            field: "size".to_string(),
            ascending: true,
        })
        .with_limit(10)
}

/// An indexOnly `entry`: its documents exist only as positions of its index.
fn index_only_entries() -> DocumentQuery {
    let schemas = platform_value!({"entry": {
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "indices": [
            {"name": "byNormalizedName",
             "properties": [{"normalizedName": "asc"}, {"name": "asc"}],
             "terminal": "$ownerId"}
        ],
        "properties": {
            "name": {"type": "string", "maxLength": 32, "position": 0},
            "normalizedName": {"type": "string", "maxLength": 32, "position": 1}
        },
        "required": ["name", "normalizedName"],
        "additionalProperties": false
    }});
    let contract = DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::new([14u8; 32]), 0, schemas, None, None)
        .expect("the entry contract is well-formed")
        .data_contract_owned();
    DocumentQuery::new(Arc::new(contract), "entry")
        .expect("the contract has this document type")
        .with_where(WhereClause {
            field: "normalizedName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("alice".to_string()),
        })
}

/// A count, sum or average proof reads a tree that an index or the document type keeps, picked
/// from the request alone by the node and again by the verifier. A sum or an average of a
/// property no index sums, or a count over a type that counts nothing, is refused by every node
/// and every verifier, so it is refused before sending.
#[test]
fn should_refuse_an_aggregate_no_index_serves_before_sending() {
    let categories_in = WhereClause {
        field: "category".to_string(),
        operator: WhereOperator::In,
        value: platform_value!(["tools", "toys"]),
    };
    let names_in = WhereClause {
        field: "name".to_string(),
        operator: WhereOperator::In,
        value: platform_value!(["a", "b"]),
    };
    let split = |query: DocumentQuery, clause: &WhereClause, field: &str| {
        query.with_where(clause.clone()).with_group_by(field)
    };
    let cases: Vec<(&str, Check, DocumentQuery, &str)> = vec![
        (
            "DocumentSum",
            check::<DocumentSum>,
            listings().with_select(SelectProjection::sum("price")),
            "no answer to this sum can be verified",
        ),
        (
            "DocumentSplitSums",
            check::<DocumentSplitSums>,
            split(
                listings().with_select(SelectProjection::sum("price")),
                &categories_in,
                "category",
            ),
            "no answer to this sum can be verified",
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            listings().with_select(SelectProjection::avg("price")),
            "no answer to this average can be verified",
        ),
        (
            "DocumentSplitAverages",
            check::<DocumentSplitAverages>,
            split(
                listings().with_select(SelectProjection::avg("price")),
                &categories_in,
                "category",
            ),
            "no answer to this average can be verified",
        ),
        (
            "DocumentCount",
            check::<DocumentCount>,
            query().with_select(SelectProjection::count_star()),
            "no answer to this count can be verified",
        ),
        (
            "DocumentSplitCounts",
            check::<DocumentSplitCounts>,
            split(
                query().with_select(SelectProjection::count_star()),
                &names_in,
                "name",
            ),
            "no answer to this count can be verified",
        ),
    ];
    for (view, check, query, words) in cases {
        let error = refusal(view, check, &query);
        assert!(error.contains(words), "{view} {query:?}: {error}");
        assert!(
            error.contains("index"),
            "{view}: refused for the index it lacks: {error}"
        );
    }
}

/// A `sale` whose index on `category` then `price` counts and sums `amount` over price ranges.
fn sales() -> DocumentQuery {
    let schemas = platform_value!({"sale": {
        "type": "object",
        "properties": {
            "category": {"type": "string", "minLength": 1, "maxLength": 59, "position": 0},
            "price": {"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 1},
            "amount": {"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 2}
        },
        "required": ["category", "price", "amount"],
        "indices": [
            {"name": "byCategoryPrice",
             "properties": [{"category": "asc"}, {"price": "asc"}],
             "averageable": "amount",
             "rangeAverageable": true}
        ],
        "additionalProperties": false
    }});
    let contract = DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::new([15u8; 32]), 0, schemas, None, None)
        .expect("the sale contract is well-formed")
        .data_contract_owned();
    DocumentQuery::new(Arc::new(contract), "sale").expect("the contract has this document type")
}

/// A count, sum or average proof answers the path query its plan lowers to, which the node
/// builds from the request alone before it proves anything. A request whose path query cannot
/// be built — an ungrouped average over a range under several `In` prefixes, which the range
/// total reads through one prefix only — is refused by every node and every verifier, so it is
/// refused before sending; the same average under one prefix is sent.
#[test]
fn should_refuse_an_aggregate_whose_path_query_cannot_be_built_before_sending() {
    let price_above_zero = WhereClause {
        field: "price".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::U64(0),
    };
    let average = || sales().with_select(SelectProjection::avg("amount"));
    let under_several = average()
        .with_where(WhereClause {
            field: "category".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["tools", "toys"]),
        })
        .with_where(price_above_zero.clone());
    let error = refusal("DocumentAverage", check::<DocumentAverage>, &under_several);
    assert!(
        error.contains("no answer to this average can be verified")
            && error.contains("prefix properties must use `==`"),
        "{error}"
    );
    let under_one = average()
        .with_where(WhereClause {
            field: "category".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("tools".to_string()),
        })
        .with_where(price_above_zero);
    assert!(
        check::<DocumentAverage>(&under_one).is_ok(),
        "{:?}",
        check::<DocumentAverage>(&under_one)
    );
}

/// A node routes every count, sum and average before executing it, and refuses a `limit` the
/// routed mode cannot honor: a total is one row, and a grouping by an `In` field is bounded by
/// the `In` array. The view routes the request the same way and refuses it before sending, in
/// the node's words; a limit on a distinct walk, which a node honors, is sent.
#[test]
fn should_refuse_a_limit_the_routed_mode_cannot_honor_before_sending() {
    let total = |select: SelectProjection| reviews().with_select(select).with_limit(1);
    let per_restaurant = |select: SelectProjection| {
        reviews()
            .with_select(select)
            .with_where(restaurants_in())
            .with_group_by("restaurantId")
            .with_limit(1)
    };
    let total_words = "is not valid for SELECT";
    let cases: Vec<(&str, Check, DocumentQuery)> = vec![
        (
            "DocumentCount",
            check::<DocumentCount>,
            total(SelectProjection::count_star()),
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            total(SelectProjection::sum("grade")),
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            total(SelectProjection::avg("grade")),
        ),
        (
            "DocumentSplitCounts",
            check::<DocumentSplitCounts>,
            per_restaurant(SelectProjection::count_star()),
        ),
        (
            "DocumentSplitSums",
            check::<DocumentSplitSums>,
            per_restaurant(SelectProjection::sum("grade")),
        ),
        (
            "DocumentSplitAverages",
            check::<DocumentSplitAverages>,
            per_restaurant(SelectProjection::avg("grade")),
        ),
    ];
    for (view, check, query) in cases {
        let error = refusal(view, check, &query);
        assert!(
            error.contains("`limit`") && error.contains(total_words),
            "{view}: {error}"
        );
        let mut without_limit = query;
        without_limit.limit = 0;
        assert!(
            check(&without_limit).is_ok(),
            "{view}: {:?}",
            check(&without_limit)
        );
    }
    let distinct_with_limit = reviews()
        .with_select(SelectProjection::count_star())
        .with_where(WhereClause {
            field: "hashtag".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("a".to_string()),
        })
        .with_group_by("hashtag")
        .with_limit(5);
    assert!(
        check::<DocumentSplitCounts>(&distinct_with_limit).is_ok(),
        "{:?}",
        check::<DocumentSplitCounts>(&distinct_with_limit)
    );
}

/// The verifier rebuilds a distinct walk's limit byte for byte and leaves the cap on it to the
/// node's configuration, which a client cannot know: a limit any node may serve is sent, and
/// one no path query can carry is refused.
#[test]
fn should_send_a_distinct_walk_limit_only_a_path_query_can_carry() {
    let distinct = |limit: u32| {
        reviews()
            .with_select(SelectProjection::count_star())
            .with_where(WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Text("a".to_string()),
            })
            .with_group_by("hashtag")
            .with_limit(limit)
    };
    for limit in [1, 500, u16::MAX as u32] {
        let query = distinct(limit);
        assert!(
            check::<DocumentSplitCounts>(&query).is_ok(),
            "limit {limit}: {:?}",
            check::<DocumentSplitCounts>(&query)
        );
    }
    let error = refusal(
        "DocumentSplitCounts",
        check::<DocumentSplitCounts>,
        &distinct(u16::MAX as u32 + 1),
    );
    assert!(error.contains("exceeds u16::MAX"), "{error}");
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

/// A count, sum or average view verifies grouped totals, not a `having` clause. A grouped query
/// with one is answered as a having-range page, which only the having view verifies, and one
/// without a grouping is refused by every node. So these views, and their split views, refuse
/// any `having` clause before sending, even one whose group an `In` constrains.
#[test]
fn should_refuse_a_having_clause_in_an_aggregate_view_before_sending() {
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
        let ungrouped = reviews().with_select(select.clone());
        let grouped_by_in = reviews()
            .with_select(select)
            .with_where(restaurants_in())
            .with_group_by("restaurantId");
        for aggregate in [ungrouped, grouped_by_in] {
            assert!(
                check(&aggregate).is_ok(),
                "{view}: the aggregate without a having clause is sent: {:?}",
                check(&aggregate)
            );
            let error = refusal(view, check, &aggregate.with_having(having.clone()));
            assert!(error.contains("DocumentHavingEntries"), "{view}: {error}");
        }
        // The `having` clause is named even when no index serves the aggregate: it is checked
        // before the proof plan, whose refusal would only say that.
        let unserved = query()
            .with_select(select_of(view))
            .with_having(having.clone());
        let error = refusal(view, check, &unserved);
        assert!(error.contains("DocumentHavingEntries"), "{view}: {error}");
    }
}

/// The select each aggregate view is verified from.
fn select_of(view: &str) -> SelectProjection {
    if view.contains("Count") {
        SelectProjection::count_star()
    } else if view.contains("Sum") {
        SelectProjection::sum("grade")
    } else {
        SelectProjection::avg("grade")
    }
}

/// A documents select is rebuilt by its verifier into Drive's query before the proof is read,
/// so a where clause Drive's grammar refuses, such as a second `In` on one field, is refused
/// for every answer. It is refused before sending, in the verifier's words.
#[test]
fn should_refuse_a_documents_select_drive_cannot_build_before_sending() {
    let in_on = |field: &str, values: Value| WhereClause {
        field: field.to_string(),
        operator: WhereOperator::In,
        value: values,
    };
    let refused = [
        query()
            .with_where(in_on("restaurantId", platform_value!(["a", "b"])))
            .with_where(in_on("restaurantId", platform_value!(["c", "d"]))),
        query()
            .with_where(in_on("restaurantId", platform_value!(["a", "b"])))
            .with_where(in_on("restaurantId", platform_value!(["c", "d"])))
            .with_where(in_on("grade", platform_value!([1u64, 2u64]))),
    ];
    let views: [(&str, Check); 2] = [
        ("Document", check::<Document>),
        ("Documents", check::<Documents>),
    ];
    for (view, check) in views {
        for query in &refused {
            let error = refusal(view, check, query);
            assert!(
                error.contains("Failed to convert DocumentQuery to DriveQuery"),
                "{view}: {error}"
            );
        }
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
            SelectProjection::sum("quantity"),
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            SelectProjection::avg("quantity"),
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
            // The window with the rest of its index pinned is served by the index.
            let served = window
                .clone()
                .with_where(WhereClause {
                    field: "category".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("tools".to_string()),
                })
                .with_select(select.clone());
            assert!(
                check(&served).is_ok(),
                "{view} {field}: the window is sent: {:?}",
                check(&served)
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
    trending_contract(platform_value!({"on": "$createdAt", "range": 86400, "step": 86400}))
}

/// Seconds after a day's window starts that [`expiring_ranked_contract`]'s index drops it.
const TRENDING_TTL_SECONDS: u64 = 172_800;

/// [`windowed_ranked_contract`] with an index that drops each day's window
/// [`TRENDING_TTL_SECONDS`] after it starts.
fn expiring_ranked_contract() -> Arc<DataContract> {
    trending_contract(platform_value!({
        "on": "$createdAt",
        "range": 86400,
        "step": 86400,
        "ttl": TRENDING_TTL_SECONDS
    }))
}

fn trending_contract(time_range: Value) -> Arc<DataContract> {
    let schemas = platform_value!({"post": {
        "type": "object",
        "properties": {
            "hashtag": {"type": "string", "minLength": 1, "maxLength": 59, "position": 0}
        },
        "required": ["hashtag", "$createdAt"],
        "indices": [
            {"name": "trending",
             "properties": [{"$createdAt": "asc"}, {"hashtag": "asc"}],
             "timeRange": time_range,
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
            "Documents",
            check::<Documents>,
            verifier_refuses::<Documents>,
            query()
                .with_where(WhereClause {
                    field: "restaurantId".to_string(),
                    operator: WhereOperator::In,
                    value: platform_value!(["a", "b"]),
                })
                .with_where(WhereClause {
                    field: "restaurantId".to_string(),
                    operator: WhereOperator::In,
                    value: platform_value!(["c", "d"]),
                }),
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
    // An aggregate no index serves is refused before sending, as the verifier's proof plan
    // refuses it; its siblings an index serves are sent. Each is pinned both ways here, so a
    // check that skipped the plan cannot pass by sending the refused ones.
    let aggregates: [(&str, Check, Verifies, DocumentQuery, bool); 5] = [
        (
            "DocumentSum",
            check::<DocumentSum>,
            verifier_refuses::<DocumentSum>,
            listings().with_select(SelectProjection::sum("price")),
            true,
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            verifier_refuses::<DocumentAverage>,
            listings().with_select(SelectProjection::avg("price")),
            true,
        ),
        (
            "DocumentCount",
            check::<DocumentCount>,
            verifier_refuses::<DocumentCount>,
            query().with_select(SelectProjection::count_star()),
            true,
        ),
        (
            "DocumentSum",
            check::<DocumentSum>,
            verifier_refuses::<DocumentSum>,
            listings()
                .with_select(SelectProjection::sum("quantity"))
                .with_integer_range("price", Value::U64(200))
                .with_where(WhereClause {
                    field: "category".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("tools".to_string()),
                }),
            false,
        ),
        (
            "DocumentAverage",
            check::<DocumentAverage>,
            verifier_refuses::<DocumentAverage>,
            reviews().with_select(SelectProjection::avg("grade")),
            false,
        ),
    ];
    for (view, check, _, query, refused_before_sending) in &aggregates {
        assert_eq!(
            check(query).is_err(),
            *refused_before_sending,
            "{view} {query:?}: {:?}",
            check(query)
        );
    }
    let shapes = shapes
        .into_iter()
        .chain(
            aggregates
                .into_iter()
                .map(|(view, check, verifies, query, _)| (view, check, verifies, query)),
        )
        .collect::<Vec<_>>();
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
        refused >= 6 && accepted_by_both >= 4,
        "{refused} refused, {accepted_by_both} accepted by both"
    );
}

/// Whether a window by start has passed its index's TTL depends on the answer's block time, so
/// the check before sending leaves it to the verifier. A window the TTL dropped long ago is
/// sent; the verifier accepts it from an answer signed at its horizon and refuses it from one
/// signed a millisecond later.
#[test]
fn should_leave_a_window_past_its_ttl_to_the_answers_block_time() {
    // A day's window in September 2020: past its TTL at any block time a check could stand in
    // for the answer's.
    let start_ms = 1_600_041_600_000;
    let horizon_ms = start_ms + TRENDING_TTL_SECONDS * 1_000;
    let selector = TimeRangeSelector::ByStart { start_ms };
    let expiring = |query: DocumentQuery| DocumentQuery {
        data_contract: expiring_ranked_contract(),
        ..query
    };
    let shapes: [(&str, Check, Verifies, DocumentQuery); 2] = [
        (
            "DocumentRankedEntries",
            check::<DocumentRankedEntries>,
            verifier_refuses::<DocumentRankedEntries>,
            expiring(trending_ranked(selector)),
        ),
        (
            "DocumentHavingEntries",
            check::<DocumentHavingEntries>,
            verifier_refuses::<DocumentHavingEntries>,
            expiring(trending_having(selector)),
        ),
    ];
    for (view, check, verifier_refuses, query) in &shapes {
        assert!(check(query).is_ok(), "{view}: {:?}", check(query));
        assert!(
            !verifier_refuses(query, horizon_ms),
            "{view}: a window is not expired at its horizon"
        );
        assert!(
            verifier_refuses(query, horizon_ms + 1),
            "{view}: a window is expired a millisecond past its horizon"
        );
    }
}
