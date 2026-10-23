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
use dapi_grpc::platform::v0::GetDocumentsResponse;
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

/// The grammars these views check are shape-only, so the fixture's document
/// type serves every view; whether an index covers the query is the server's
/// call, and the server refuses a query no index covers.
fn contract() -> Arc<DataContract> {
    Arc::new(
        get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
            .data_contract_owned(),
    )
}

fn query() -> DocumentQuery {
    DocumentQuery::new(contract(), "niceDocument").expect("the fixture has this document type")
}

/// `SELECT AVG(grade) GROUP BY restaurantId ORDER BY grade DESC LIMIT 5`.
fn ranked() -> DocumentQuery {
    query()
        .with_select(SelectProjection::avg("grade"))
        .with_group_by("restaurantId")
        .order_by_selected_aggregate(RankingDirection::Descending)
        .with_limit(5)
}

/// `SELECT COUNT(*) GROUP BY hashtag HAVING $count > 100 LIMIT 100`.
fn having() -> DocumentQuery {
    query()
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

/// Window selections stay pending until the answer arrives: a relative
/// time-range selection resolves only against the answer's signed block time.
/// Resolving any selection also needs the contract's bucketing index, which
/// the fixture lacks, so a check that resolved them would refuse these.
#[test]
fn should_accept_pending_window_selections_before_they_are_resolved() {
    let cases: [(&str, Check, DocumentQuery); 2] = [
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
    ];
    for (view, check, query) in cases {
        for query in [
            query
                .clone()
                .with_time_range("$createdAt", TimeRangeSelector::Newest),
            query.with_integer_range("price", Value::U64(600)),
        ] {
            assert!(check(&query).is_ok(), "{view}: {:?}", check(&query));
        }
    }
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
