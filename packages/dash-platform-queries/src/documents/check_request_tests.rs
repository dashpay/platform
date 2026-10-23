//! [`FromProof::check_request`] for every view fetched with a [`DocumentQuery`].
//!
//! All the views share one request type and one wire encoding, so a query
//! built for one view can be fetched as another. Every node serves such a
//! query as what it asks for, and the view's verifier then refuses the
//! answer for the request alone, which the SDK reports as a failed proof and
//! holds against the node. `check_request` must refuse exactly those queries
//! before they are sent, and must accept every query the view can be
//! verified from.

use crate::documents::composite_document_query::CompositeSubQuery;
use crate::documents::document_query::{DocumentQuery, RankingDirection};
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::platform_value::Value;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
    SelectProjection, TimeRangeSelector, WhereClause, WhereOperator,
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

/// Guards every refusal test below: a view must keep accepting the queries it
/// is verified from, or the check would break fetches that work today.
#[test]
fn should_accept_every_query_its_view_is_verified_from() {
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

/// Each aggregate is served as the select asks; a view of another aggregate,
/// or of documents, can never verify the answer.
#[test]
fn should_refuse_a_view_of_a_query_that_selects_something_else() {
    let documents = query();
    let count = query().with_select(SelectProjection::count_star());
    let sum = query().with_select(SelectProjection::sum("grade"));
    let cases: [(&str, Check, &DocumentQuery); 8] = [
        ("Document", check::<Document>, &count),
        ("Documents", check::<Documents>, &sum),
        ("DocumentCount", check::<DocumentCount>, &documents),
        ("DocumentSplitCounts", check::<DocumentSplitCounts>, &sum),
        ("DocumentSum", check::<DocumentSum>, &documents),
        ("DocumentSplitSums", check::<DocumentSplitSums>, &count),
        ("DocumentAverage", check::<DocumentAverage>, &documents),
        (
            "DocumentSplitAverages",
            check::<DocumentSplitAverages>,
            &sum,
        ),
    ];
    for (view, check, query) in cases {
        refusal(view, check, query);
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

/// Without sub-queries the encoder sends a plain page, which nodes serve.
#[test]
fn should_refuse_composite_documents_without_sub_queries() {
    let error = refusal(
        "CompositeDocuments",
        check::<CompositeDocuments>,
        &query().with_limit(10),
    );
    assert!(error.contains("between 1 and"), "{error}");
    assert!(!error.contains("SDK misconfigured"), "{error}");
}

/// A pending time-range selection is resolved only against the answer's
/// signed block time, so the check must leave it alone: the fixture has no
/// time-range index, and resolving the selection here would refuse it.
#[test]
fn should_accept_pending_time_range_selections_before_they_are_resolved() {
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
        let query = query.with_time_range("$createdAt", TimeRangeSelector::Newest);
        assert!(check(&query).is_ok(), "{view}: {:?}", check(&query));
    }
}
