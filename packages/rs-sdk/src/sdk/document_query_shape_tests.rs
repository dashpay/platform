//! A document query that the view it is fetched as cannot be verified from is
//! refused before it is sent. Sent, it is served by every node, and the
//! view's verifier then refuses the answer for the request alone: an
//! `Error::Proof`, which is retried on other nodes and bans each node that
//! answered.
use super::{Sdk, SdkInstance};
use crate::platform::documents::composite_document_query::CompositeSubQuery;
use crate::platform::documents::document_query::RankingDirection;
use crate::platform::{DocumentQuery, Fetch, FetchMany, Query};
use crate::{Error, SdkBuilder};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0, GetDocumentsResponseV0, Version,
};
use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse, Proof, ResponseMetadata};
use dpp::document::Document;
use dpp::platform_value::Value;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::{SelectProjection, WhereClause, WhereOperator};
use drive_proof_verifier::{
    CompositeDocuments, DocumentCount, DocumentHavingEntries, DocumentRankedEntries,
};
use rs_dapi_client::ExecutionResponse;
use std::fmt::Debug;

fn sdk() -> Sdk {
    SdkBuilder::new_mock()
        .with_version(PlatformVersion::latest())
        .build()
        .expect("mock sdk")
}

fn query() -> DocumentQuery {
    let contract = get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
        .data_contract_owned();
    DocumentQuery::new(contract, "niceDocument").expect("the fixture has this document type")
}

/// `SELECT AVG(grade) GROUP BY restaurantId ORDER BY grade DESC LIMIT 5`.
fn ranked() -> DocumentQuery {
    query()
        .with_select(SelectProjection::avg("grade"))
        .with_group_by("restaurantId")
        .order_by_selected_aggregate(RankingDirection::Descending)
        .with_limit(5)
}

/// Have the network answer `query` with a proof, as every node serves it, and
/// return the wire request the fetch would send.
async fn serve(sdk: &Sdk, query: &DocumentQuery) -> GetDocumentsRequest {
    let request: GetDocumentsRequest =
        Query::<GetDocumentsRequest>::query(query, &sdk.query_settings()).expect("encodes");
    let response = GetDocumentsResponse {
        version: Some(Version::V0(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(Proof::default())),
            metadata: Some(ResponseMetadata {
                protocol_version: PlatformVersion::latest().protocol_version,
                ..Default::default()
            }),
        })),
    };
    let SdkInstance::Mock { dapi, .. } = &sdk.inner else {
        panic!("mock")
    };
    dapi.lock()
        .await
        .expect(
            &request,
            &Ok(ExecutionResponse {
                inner: response,
                address: "http://127.0.0.1:9000".parse().unwrap(),
                retries: 0,
            }),
        )
        .unwrap();
    request
}

async fn times_sent(sdk: &Sdk, request: &GetDocumentsRequest) -> usize {
    let SdkInstance::Mock { dapi, .. } = &sdk.inner else {
        panic!("mock")
    };
    dapi.lock().await.request_count(request)
}

async fn assert_refused_before_sending<T: Debug>(
    sdk: &Sdk,
    request: &GetDocumentsRequest,
    result: Result<T, Error>,
    case: &str,
) {
    let sent = times_sent(sdk, request).await;
    match result {
        Err(Error::Config(_)) if sent == 0 => {}
        other => panic!("{case}: must be refused before it is sent; got {other:?} after sending it {sent} times"),
    }
}

/// A node routing with an older query table serves a ranked request outside
/// the ranked grammar (here a range on the grouped property) as a plain
/// grouped aggregate; every protocol-13 node does.
#[tokio::test]
async fn should_refuse_a_malformed_ranked_query_without_sending_it() {
    let sdk = sdk();
    let query = ranked().with_where(WhereClause {
        field: "restaurantId".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("a".to_string()),
    });
    let request = serve(&sdk, &query).await;
    let result = DocumentRankedEntries::fetch(&sdk, query).await;
    assert_refused_before_sending(&sdk, &request, result, "range on the ranked group").await;
}

#[tokio::test]
async fn should_refuse_document_sub_queries_without_sending_them() {
    let sdk = sdk();
    let sub_query = CompositeSubQuery::documents(query().data_contract, "niceDocument")
        .expect("the fixture has this document type")
        .with_limit(10);
    let query = query().with_limit(10).with_sub_query(sub_query);
    let request = serve(&sdk, &query).await;
    let result = Document::fetch_many(&sdk, query).await;
    assert_refused_before_sending(&sdk, &request, result, "documents with sub-queries").await;
}

/// Without `count_star()` the query is a plain documents query, which every
/// node serves.
#[tokio::test]
async fn should_refuse_a_count_of_a_query_that_does_not_select_it_without_sending_it() {
    let sdk = sdk();
    let query = query();
    let request = serve(&sdk, &query).await;
    let result = DocumentCount::fetch(&sdk, query).await;
    assert_refused_before_sending(&sdk, &request, result, "count of a documents query").await;
}

/// Every node serves a count; the documents verifier rebuilds a documents
/// query and cannot verify it.
#[tokio::test]
async fn should_refuse_documents_of_an_aggregate_select_without_sending_it() {
    let sdk = sdk();
    let query = query().with_select(SelectProjection::count_star());
    let request = serve(&sdk, &query).await;
    let result = Document::fetch_many(&sdk, query).await;
    assert_refused_before_sending(&sdk, &request, result, "documents of a count").await;
}

/// Without a `having` clause the query is an ordinary grouped aggregate,
/// which every node serves.
#[tokio::test]
async fn should_refuse_a_having_fetch_without_a_having_clause_without_sending_it() {
    let sdk = sdk();
    let query = query()
        .with_select(SelectProjection::count_star())
        .with_group_by("hashtag")
        .with_limit(100);
    let request = serve(&sdk, &query).await;
    let result = DocumentHavingEntries::fetch(&sdk, query).await;
    assert_refused_before_sending(&sdk, &request, result, "having without a having clause").await;
}

/// Without sub-queries the query is sent as a plain page, which every node
/// serves.
#[tokio::test]
async fn should_refuse_composite_documents_without_sub_queries_without_sending_it() {
    let sdk = sdk();
    let query = query().with_limit(10);
    let request = serve(&sdk, &query).await;
    let result = CompositeDocuments::fetch(&sdk, query).await;
    assert_refused_before_sending(&sdk, &request, result, "composite without sub-queries").await;
}

/// Guards the refusals above: a query its view is verified from still goes
/// out through both fetch paths. The made-up proof then fails verification,
/// which is not what is under test.
#[tokio::test]
async fn should_still_send_queries_their_view_is_verified_from() {
    let sdk = sdk();
    let ranked = ranked();
    let ranked_request = serve(&sdk, &ranked).await;
    let ranked_result = DocumentRankedEntries::fetch(&sdk, ranked).await;
    let documents = query();
    let documents_request = serve(&sdk, &documents).await;
    let documents_result = Document::fetch_many(&sdk, documents).await;
    for (case, request, config_error) in [
        (
            "ranked",
            &ranked_request,
            matches!(ranked_result, Err(Error::Config(_))),
        ),
        (
            "documents",
            &documents_request,
            matches!(documents_result, Err(Error::Config(_))),
        ),
    ] {
        assert!(!config_error, "{case}: refused locally");
        assert!(times_sent(&sdk, request).await > 0, "{case}: never sent");
    }
}
