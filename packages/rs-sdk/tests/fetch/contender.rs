use std::sync::Arc;

use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::Contender;
use dpp::data_contract::DataContract;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
/*
///! Tests for SDK requests that return one or more [Contender] objects.

/// Given some data contract ID, document type and document ID, when I fetch it, then I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_vote_states_ok() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("contested_resource_vote_states_ok").await;

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

    // Now query for individual document
    let query = ContestedDocumentVotePollDriveQuery {
        limit: None,
        offset: None,
        order_ascending: true,
        start_at: None,
        vote_poll,
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    };

    let first_contender = Contender::fetch_many(&sdk, query)
        .await
        .expect("fetch many contenders")
        .pop_first()
        .expect("first item must exist")
        .1
        .expect("contender must exist");

    let doc = Document::fetch(&sdk, query)
        .await
        .expect("fetch document")
        .expect("document must be found");

    assert_eq!(first_doc, doc);
}
*/
