//! A document query that the view it is fetched as cannot be verified from is
//! refused before it is sent. Sent, it is served by the nodes these tests
//! name, and the view's verifier then refuses the answer for the request
//! alone: an `Error::Proof`, which is retried on other nodes and bans each
//! node that answered.
use super::{Sdk, SdkInstance};
use crate::mock::{MockDashPlatformSdk, MockResponse};
use crate::platform::documents::composite_document_query::CompositeSubQuery;
use crate::platform::documents::document_query::RankingDirection;
use crate::platform::{DocumentQuery, Fetch, FetchMany, Query, QuerySettings};
use crate::{Error, SdkBuilder};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0, GetDocumentsResponseV0, Version,
};
use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::document::Document;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::{SelectProjection, WhereClause, WhereOperator};
use drive_proof_verifier::{
    CompositeDocuments, DocumentCount, DocumentHavingEntries, DocumentRankedEntries,
    DocumentSplitCounts, FromProof,
};
use rs_dapi_client::ExecutionResponse;
use std::fmt::Debug;
use std::sync::atomic::Ordering;

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
/// grouped aggregate.
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

/// A `post` whose `hashtags` is a typed array of strings.
fn post_contract() -> DataContract {
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
            {"name": "byHashtag", "properties": [{"hashtags": "asc"}], "countable": "countable"}
        ],
        "additionalProperties": false
    }});
    DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::new([7u8; 32]), 0, schemas, None, None)
        .expect("the post contract is well-formed")
        .data_contract_owned()
}

/// Grouped by a typed array, a `containsAny` over several elements is served
/// with one count per element; one total would count a post once per element
/// it holds, so `DocumentCount` cannot verify it.
#[tokio::test]
async fn should_refuse_one_count_over_several_elements_of_a_typed_array_without_sending_it() {
    let sdk = sdk();
    let query = DocumentQuery::new(post_contract(), "post")
        .expect("the contract has this document type")
        .with_select(SelectProjection::count_star())
        .with_where(WhereClause {
            field: "hashtags".to_string(),
            operator: WhereOperator::ContainsAny,
            value: platform_value!(["dash", "news"]),
        })
        .with_group_by("hashtags");
    let request = serve(&sdk, &query).await;
    let result = DocumentCount::fetch(&sdk, query).await;
    assert_refused_before_sending(&sdk, &request, result, "one count over several elements").await;
}

/// From protocol version 14 a grouped count ordered by the count is served as
/// a ranked answer, which no count view verifies.
#[tokio::test]
async fn should_refuse_counts_of_a_ranked_query_without_sending_it() {
    let sdk = sdk();
    let query = query()
        .with_select(SelectProjection::count_star())
        .with_group_by("restaurantId")
        .order_by_selected_aggregate(RankingDirection::Descending)
        .with_limit(5);
    let request = serve(&sdk, &query).await;
    let result = DocumentSplitCounts::fetch(&sdk, query).await;
    assert_refused_before_sending(&sdk, &request, result, "counts of a ranked query").await;
}

/// Refuses every request, naming the protocol version it was checked at.
#[derive(Debug, Default)]
pub(super) struct CheckedAt;

impl FromProof<DocumentQuery> for CheckedAt {
    type Request = DocumentQuery;
    type Response = GetDocumentsResponse;

    fn check_request(
        _request: &DocumentQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(), drive_proof_verifier::Error> {
        Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "checked at protocol version {}",
                platform_version.protocol_version
            ),
        })
    }

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        _response: O,
        _network: Network,
        _platform_version: &PlatformVersion,
        _provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: 'a,
    {
        unreachable!("every request is refused before it is sent")
    }
}

impl MockResponse for CheckedAt {
    fn mock_serialize(&self, _mock_sdk: &MockDashPlatformSdk) -> Vec<u8> {
        unreachable!("every request is refused before it is sent")
    }

    fn mock_deserialize(_mock_sdk: &MockDashPlatformSdk, _buf: &[u8]) -> Self {
        unreachable!("every request is refused before it is sent")
    }
}

impl FromIterator<(u8, Option<CheckedAt>)> for CheckedAt {
    fn from_iter<T: IntoIterator<Item = (u8, Option<CheckedAt>)>>(_iter: T) -> Self {
        CheckedAt
    }
}

impl Fetch for CheckedAt {
    type Query = DocumentQuery;
    type Request = GetDocumentsRequest;
}

impl FetchMany<u8, CheckedAt> for CheckedAt {
    type Query = DocumentQuery;
    type Request = GetDocumentsRequest;
}

/// A query whose building raises the SDK's protocol version, as a verified
/// answer to another request can while this one is being built.
#[derive(Clone, Debug)]
struct RaisesTheVersion {
    sdk: Sdk,
    query: DocumentQuery,
}

impl Query<DocumentQuery> for RaisesTheVersion {
    fn query(&self, _settings: &QuerySettings<'_>) -> Result<DocumentQuery, Error> {
        self.sdk.protocol_version.store(
            PlatformVersion::latest().protocol_version,
            Ordering::Relaxed,
        );
        Ok(self.query.clone())
    }
}

/// A request is checked at the protocol version it was encoded with, so the
/// check judges the request that is sent.
#[tokio::test]
async fn should_check_a_request_at_the_version_it_was_encoded_with() {
    let encoded_with = PlatformVersion::latest().protocol_version - 1;
    let sdk = SdkBuilder::new_mock()
        .with_version(PlatformVersion::get(encoded_with).expect("a known version"))
        .build()
        .expect("mock sdk");
    let raises_the_version = || RaisesTheVersion {
        sdk: sdk.clone(),
        query: query(),
    };
    let fetched = CheckedAt::fetch(&sdk, raises_the_version())
        .await
        .map(|_| ());
    sdk.protocol_version.store(encoded_with, Ordering::Relaxed);
    let fetched_many = CheckedAt::fetch_many(&sdk, raises_the_version())
        .await
        .map(|_| ());
    let expected = format!("checked at protocol version {encoded_with}");
    for (case, result) in [("fetch", fetched), ("fetch_many", fetched_many)] {
        match result {
            Err(Error::Config(message)) => assert_eq!(message, expected, "{case}"),
            other => panic!("{case}: must be refused, got {other:?}"),
        }
    }
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
