//! A sum of another property than the one a document type sums its documents by. Every node
//! refuses it, at every protocol version that serves sums, with an internal error. A client
//! retries that on every other node and bans each one for it, so the SDK refuses the request
//! before sending it.

use super::*;
use dash_platform_queries::documents::document_query::DocumentQuery;
use dpp::platform_value::Value;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::{TryFromPlatformVersioned, PLATFORM_VERSIONS};
use drive::query::{SelectProjection, WhereClause, WhereOperator};
use drive_proof_verifier::{DocumentSplitSums, DocumentSum, FromProof};

/// Its `tip` documents are summed by `amount`; `sentAt` is another integer property.
const TIP_JAR_CONTRACT_PATH: &str =
    "../rs-drive/tests/supporting_files/contract/tip-jar/tip-jar-contract.json";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_refuse_before_sending_a_sum_every_node_answers_with_an_internal_error() {
    // Sums ride the V1 documents wire.
    for version in PLATFORM_VERSIONS.iter().filter(|version| {
        version
            .drive_abci
            .query
            .document_query
            .default_current_version
            >= 1
    }) {
        let protocol_version = version.protocol_version;
        let fixture = QueryFixture::new_with_version(version);
        let contract = Arc::new(
            json_document_to_contract(TIP_JAR_CONTRACT_PATH, false, version)
                .expect("the tip jar contract"),
        );
        fixture
            .platform
            .drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
            .expect("store the tip jar contract");
        let (mut client, server) = fixture.client();
        let sum_of = |field: &str| {
            DocumentQuery::new(Arc::clone(&contract), "tip")
                .expect("the contract has this document type")
                .with_select(SelectProjection::sum(field))
        };

        let other_property = sum_of("sentAt");
        let request =
            wire::GetDocumentsRequest::try_from_platform_versioned(other_property.clone(), version)
                .expect("the query encodes");
        // What nodes on the network answer, which the SDK has to keep away from. Once nodes
        // answer it with a status clients do not retry, this expectation changes with them.
        assert_eq!(
            code(client.get_documents(request).await.map(|_| ())),
            Code::Internal,
            "protocol {protocol_version}: a status the client retries elsewhere"
        );
        let split = |field: &str| {
            sum_of(field)
                .with_where(WhereClause {
                    field: "recipient".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::Identifier([1; 32]),
                        Value::Identifier([2; 32]),
                    ]),
                })
                .with_group_by("recipient")
        };
        for (view, check, other_property, own_property) in [
            (
                "DocumentSum",
                DocumentSum::check_request as Check,
                other_property.clone(),
                sum_of("amount"),
            ),
            (
                "DocumentSplitSums",
                DocumentSplitSums::check_request,
                split("sentAt"),
                split("amount"),
            ),
        ] {
            match check(&other_property, version) {
                Err(drive_proof_verifier::Error::RequestError { error })
                    if error.contains("summed by amount") => {}
                other => panic!(
                    "protocol {protocol_version}: {view} must refuse it before it is sent, \
                     got {other:?}"
                ),
            }
            check(&own_property, version)
                .unwrap_or_else(|e| panic!("protocol {protocol_version}: {view} sends: {e}"));
            let request =
                wire::GetDocumentsRequest::try_from_platform_versioned(own_property, version)
                    .expect("the query encodes");
            client
                .get_documents(request)
                .await
                .unwrap_or_else(|status| {
                    panic!("protocol {protocol_version}: {view} is served: {status:?}")
                });
        }

        // An average reads the count and sum trees of a covering index, which a type summed
        // by another property does not have, so nodes refuse it as a query error.
        let average = DocumentQuery::new(Arc::clone(&contract), "tip")
            .expect("the contract has this document type")
            .with_select(SelectProjection::avg("sentAt"));
        let request = wire::GetDocumentsRequest::try_from_platform_versioned(average, version)
            .expect("the query encodes");
        assert_eq!(
            code(client.get_documents(request).await.map(|_| ())),
            Code::InvalidArgument,
            "protocol {protocol_version}: an average of another property"
        );
        server.abort();
    }
}

type Check = fn(&DocumentQuery, &PlatformVersion) -> Result<(), drive_proof_verifier::Error>;
