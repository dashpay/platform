use super::common::{mock_data_contract, mock_document_type};
use dapi_grpc::platform::v0::{
    get_contested_resource_identity_votes_request as identity_request,
    get_vote_polls_by_end_date_request as polls_request, GetContestedResourceIdentityVotesRequest,
    GetVotePollsByEndDateRequest,
};
use dash_sdk::{
    platform::{DocumentQuery, FetchMany, Query},
    Sdk,
};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters,
        document_type::{
            accessors::DocumentTypeV0Getters, random_document::CreateRandomDocument, DocumentType,
        },
    },
    document::{Document, DocumentV0Getters},
};
use dpp::{
    platform_value::Value,
    prelude::Identifier,
    voting::{
        vote_choices::resource_vote_choice::ResourceVoteChoice,
        vote_polls::{
            contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll, VotePoll,
        },
        votes::resource_vote::{v0::ResourceVoteV0, ResourceVote},
    },
};
use drive::query::{
    contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery,
    VotePollsByEndDateDriveQuery,
};
use drive_proof_verifier::types::{
    Documents, ResourceVotesByIdentity, VotePollsGroupedByTimestamp,
};

/// Given some data contract, document type and 1 document of this type, when I request multiple documents, I get that
/// document.
#[tokio::test]
async fn test_mock_document_fetch_many() {
    let mut sdk = Sdk::new_mock();
    let document_type: DocumentType = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));

    let expected_doc = document_type
        .random_document(None, sdk.version())
        .expect("document should be created");
    let expected = Documents::from([(expected_doc.id(), Some(expected_doc.clone()))]);

    // document that should not be returned, as it will be defined as a duplicate
    let not_expected_doc = document_type
        .random_document(None, sdk.version())
        .expect("document 2 should be created");
    let not_expected = Documents::from([(not_expected_doc.id(), Some(not_expected_doc))]);

    let document_type_name = document_type.name();

    // [DocumentQuery::new_with_document_id] will fetch the data contract first, so we need to define an expectation for it.
    sdk.mock()
        .expect_fetch(data_contract.id(), Some(data_contract.clone()))
        .await
        .unwrap();

    let query =
        DocumentQuery::new(data_contract, document_type_name).expect("create document query");
    sdk.mock()
        .expect_fetch_many(query.clone(), Some(expected.clone()))
        .await
        .unwrap();

    sdk.mock()
        .expect_fetch_many(query.clone(), Some(not_expected))
        .await
        .expect_err("duplicate expectations are not allowed");

    let retrieved = Document::fetch_many(&sdk, query).await.unwrap();

    assert!(!retrieved.is_empty());
    assert_eq!(retrieved, expected);
}

fn mock_vote_poll() -> VotePoll {
    ContestedDocumentResourceVotePoll {
        contract_id: Identifier::from([5; 32]),
        document_type_name: "domain".into(),
        index_name: "parentNameAndLabel".into(),
        index_values: vec![Value::Text("dash".into()), Value::Text("alice".into())],
    }
    .into()
}

#[tokio::test]
async fn should_match_public_mock_poll_expectations_with_effective_query_limits() {
    let expected = VotePollsGroupedByTimestamp(vec![(42, vec![mock_vote_poll()])]);
    for (prove, limit) in [
        (true, None),
        (true, Some(0)),
        (true, Some(7)),
        (false, None),
    ] {
        let mut sdk = Sdk::new_mock();
        let query = VotePollsByEndDateDriveQuery {
            start_time: Some((40, true)),
            end_time: Some((50, false)),
            limit,
            offset: None,
            order_ascending: false,
        };
        let mut raw: GetVotePollsByEndDateRequest = query.query(&sdk.query_settings()).unwrap();
        let Some(polls_request::Version::V0(v0)) = raw.version.as_mut() else {
            panic!("V0 poll request")
        };
        v0.prove = prove;
        if prove {
            sdk.mock()
                .expect_fetch_many::<_, VotePoll, _, VotePollsGroupedByTimestamp>(
                    query.clone(),
                    Some(expected.clone()),
                )
                .await
                .unwrap();
            assert_eq!(
                VotePoll::fetch_many(&sdk, query).await.unwrap().0,
                expected.0
            );
        } else {
            sdk.mock()
                .expect_fetch_many::<_, VotePoll, _, VotePollsGroupedByTimestamp>(
                    raw,
                    Some(expected.clone()),
                )
                .await
                .unwrap();
        }
        assert_eq!(VotePoll::fetch_many(&sdk, raw).await.unwrap().0, expected.0);
    }
}

#[tokio::test]
async fn should_match_public_mock_identity_vote_expectations_with_effective_query_limits() {
    let poll = mock_vote_poll();
    let expected: ResourceVotesByIdentity = [(
        poll.unique_id().unwrap(),
        Some(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: poll,
            resource_vote_choice: ResourceVoteChoice::Abstain,
        })),
    )]
    .into_iter()
    .collect();
    for (prove, limit) in [
        (true, None),
        (true, Some(0)),
        (true, Some(7)),
        (false, None),
    ] {
        let mut sdk = Sdk::new_mock();
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id: Identifier::from([4; 32]),
            limit,
            offset: None,
            start_at: None,
            order_ascending: false,
        };
        let mut raw: GetContestedResourceIdentityVotesRequest =
            query.query(&sdk.query_settings()).unwrap();
        let Some(identity_request::Version::V0(v0)) = raw.version.as_mut() else {
            panic!("V0 identity-votes request")
        };
        v0.prove = prove;
        if prove {
            sdk.mock()
                .expect_fetch_many::<_, ResourceVote, _, ResourceVotesByIdentity>(
                    query.clone(),
                    Some(expected.clone()),
                )
                .await
                .unwrap();
            assert_eq!(
                ResourceVote::fetch_many(&sdk, query).await.unwrap(),
                expected
            );
        } else {
            sdk.mock()
                .expect_fetch_many::<_, ResourceVote, _, ResourceVotesByIdentity>(
                    raw.clone(),
                    Some(expected.clone()),
                )
                .await
                .unwrap();
        }
        assert_eq!(
            ResourceVote::fetch_many(&sdk, raw.clone()).await.unwrap(),
            expected
        );
    }
}
