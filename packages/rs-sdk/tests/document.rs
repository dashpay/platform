//! Test document CRUDL operations

use std::fmt::Debug;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::{DataContract, Identifier};
use drive::query::DriveQuery;
use drive_proof_verifier::proof::from_proof::Length;
use drive_proof_verifier::FromProof;
use rs_dapi_client::transport::TransportRequest;
use rs_sdk::platform::DocumentQuery;
use rs_sdk::platform::{Fetch, List, Query};
use rs_sdk::Sdk;

include!("common.rs");

const DOCUMENT_TYPE_NAME: &str = "indexedDocument";
const DOCUMENT_ID: &str = "uHfJHpk77MGiqsvvJc8mqgT3O6RK8Ue/u5zIjowu7Uk=";

async fn test_read<O: Fetch, Q: Query<<O as Fetch>::Request>>(
    api: &mut Sdk,
    id: Q,
    expected: Result<usize, rs_sdk::error::Error>,
) -> Result<Option<O>, rs_sdk::error::Error>
where
    O: Debug + Clone + Send,
    Option<O>: Length,
    <O as FromProof<<O as Fetch>::Request>>::Response:
        From<<<O as Fetch>::Request as TransportRequest>::Response>,
{
    let result = O::fetch(api, id).await;

    match expected {
        Ok(count) => {
            if let Ok(ref o) = result {
                assert_eq!(count, o.count_some(), "result: {:?}", o);
            } else {
                panic!("Expected Ok, got error: {:?}", result)
            }
        }
        Err(e) => {
            if let Err(ref e2) = result {
                assert_eq!(e.to_string(), e2.to_string());
            } else {
                panic!("Expected error, got Ok: {:?}", result)
            }
        }
    }

    result
}

#[ignore = "needs working platform"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_read() {
    setup_logs();

    let mut api = setup_api();

    let data_contract_id = base64_identifier(DATA_CONTRACT_ID);
    let document_id = base64_identifier(DOCUMENT_ID);

    let query = DocumentQuery::new_with_document_id(
        &mut api,
        data_contract_id,
        DOCUMENT_TYPE_NAME,
        document_id,
    )
    .await
    .expect("create SdkDocumentQuery");

    let _res: Result<Option<Document>, rs_sdk::error::Error> =
        test_read(&mut api, query, Ok(1)).await;
}

#[ignore = "needs working platform"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_read_no_contract() {
    setup_logs();

    let mut api = setup_api();

    let data_contract_id = Identifier::from_bytes(&[0; 32]).expect("create Identifier");
    let document_id = base64_identifier(DOCUMENT_ID);

    let query = DocumentQuery::new_with_document_id(
        &mut api,
        data_contract_id,
        DOCUMENT_TYPE_NAME,
        document_id,
    )
    .await;

    assert!(matches!(query, Err(rs_sdk::error::Error::NotFound(_))));
}

#[ignore = "needs working platform"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_read_no_document() {
    setup_logs();

    let mut api = setup_api();

    let data_contract_id = base64_identifier(DATA_CONTRACT_ID);
    let document_id = Identifier::from_bytes(&[0; 32]).expect("create Identifier");

    let query = DocumentQuery::new_with_document_id(
        &mut api,
        data_contract_id,
        DOCUMENT_TYPE_NAME,
        document_id,
    )
    .await
    .expect("create SdkDocumentQuery");

    let _res: Result<Option<Document>, rs_sdk::error::Error> =
        test_read(&mut api, query, Ok(0)).await;
}

#[ignore = "needs working platform"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_list() {
    setup_logs();

    let mut api = setup_api();

    let data_contract_id = base64_identifier(DATA_CONTRACT_ID);

    let data_contract = DataContract::fetch(&mut api, data_contract_id)
        .await
        .expect("fetch data contract")
        .expect("data contract not found");

    let doctype = data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("document type not found");

    let query = DriveQuery::any_item_query(&data_contract, doctype);

    let docs = <Document>::list(&mut api, query)
        .await
        .expect("list documents")
        .expect("no documents found");

    assert!(docs.len() > 0);
    let doc_ids: Vec<String> = docs
        .iter()
        .map(|d| d.id().to_string(Encoding::Base64))
        .collect();

    tracing::info!(documents=?doc_ids, "fetched documents");
}
