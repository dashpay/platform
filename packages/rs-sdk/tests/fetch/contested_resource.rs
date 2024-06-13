//! Tests of ContestedResource object

use crate::fetch::{common::setup_logs, config::Config};
use dash_sdk::platform::FetchMany;
use dpp::platform_value::Value;
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive_proof_verifier::types::ContestedResource;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "network-testing",
    ignore = "requires a DPNS name to be registered"
)]
async fn test_contested_resources_ok() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("test_contested_resources_ok").await;

    let index_name = "parentNameAndLabel";

    let query = VotePollsByDocumentTypeQuery {
        contract_id: cfg.existing_data_contract_id,
        document_type_name: cfg.existing_document_type_name.clone(),
        index_name: index_name.to_string(),
        start_at_value: None,
        start_index_values: vec![Value::Text("dash".into())],
        end_index_values: vec![],
        limit: None,
        order_ascending: false,
    };

    let rss = ContestedResource::fetch_many(&sdk, query)
        .await
        .expect("fetch contested resources");

    assert!(!rss.0.is_empty());
}
