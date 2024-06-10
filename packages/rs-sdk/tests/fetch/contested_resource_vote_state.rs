//! Tests for SDK requests that return one or more [Contender] objects.
use crate::fetch::{common::setup_logs, config::Config};
use dash_sdk::platform::{DocumentQuery, Fetch, FetchMany};
use dpp::{
    data_contract::DataContract, document::Document, voting::contender_structs::Contender,
    voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll,
};
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use std::sync::Arc;

/// Given some data contract ID, document type and document ID, when I fetch it, then I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_vote_states_not_found() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("contested_resource_vote_states_not_found")
        .await;

    let data_contract_id = cfg.existing_data_contract_id;

    let contract = Arc::new(
        DataContract::fetch(&sdk, data_contract_id)
            .await
            .expect("fetch data contract")
            .expect("data contract not found"),
    );

    // Fetch multiple documents so that we get document ID
    let all_docs_query =
        DocumentQuery::new(Arc::clone(&contract), &cfg.existing_document_type_name)
            .expect("create SdkDocumentQuery");
    let first_doc = Document::fetch_many(&sdk, all_docs_query)
        .await
        .expect("fetch many documents")
        .pop_first()
        .expect("first item must exist")
        .1
        .expect("document must exist");

    tracing::info!("first_doc: {}", first_doc.to_string());
    // Now query for individual document
    let query = ContestedDocumentVotePollDriveQuery {
        limit: None,
        offset: None,
        start_at: None,
        vote_poll: ContestedDocumentResourceVotePoll {
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec!["dash".into()],
            document_type_name: cfg.existing_document_type_name,
            contract_id: data_contract_id,
        },
        allow_include_locked_and_abstaining_vote_tally: true,
        // TODO test other result types
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    };

    let contenders = Contender::fetch_many(&sdk, query)
        .await
        .expect("fetch many contenders");

    assert!(
        contenders.contenders.is_empty(),
        "no contenders expected for this query"
    );
}
