//! Tests of mocked Fetch trait implementations.

use super::common::{bootstrap_mock_sdk_to_latest, mock_data_contract, mock_document_type};
use dash_sdk::{
    platform::{DocumentQuery, DocumentQuerySdk, Fetch},
    Sdk, SdkBuilder,
};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters,
        document_type::{
            accessors::DocumentTypeV0Getters, random_document::CreateRandomDocument, DocumentType,
        },
    },
    document::Document,
    identity::{accessors::IdentityGettersV0, IdentityV0},
    prelude::{DataContract, Identifier, Identity},
    version::PlatformVersion,
};

#[tokio::test]
/// Given some identity, when I fetch it using mock API, then I get the same identity
async fn test_mock_fetch_identity() {
    let mut sdk = Sdk::new_mock();

    let expected: Identity = Identity::from(IdentityV0::default());
    let query = expected.id();

    sdk.mock()
        .expect_fetch(query, Some(expected.clone()))
        .await
        .unwrap();

    let retrieved = Identity::fetch(&sdk, query)
        .await
        .unwrap()
        .expect("object should exist");

    assert_eq!(retrieved, expected);
}

#[tokio::test]
/// When I define mock expectation twice for the same request, second call ends with error
async fn test_mock_fetch_duplicate_expectation() {
    let mut sdk = Sdk::new_mock();

    let expected: Identity = Identity::from(IdentityV0::default());
    let expected2 =
        Identity::random_identity(3, Some(2), PlatformVersion::latest()).expect("random identity");

    let query = expected.id();

    sdk.mock()
        .expect_fetch(query, Some(expected.clone()))
        .await
        .expect("first expectation should be added correctly");

    sdk.mock()
        .expect_fetch(query, Some(expected2))
        .await
        .expect_err("conflicting expectation should fail");

    let retrieved = Identity::fetch(&sdk, query)
        .await
        .unwrap()
        .expect("object should exist");

    assert_eq!(retrieved, expected);
}

#[tokio::test]
/// Given some random identity ID, when I fetch it using mock API, then I get None
async fn test_mock_fetch_identity_not_found() {
    let mut sdk = Sdk::new_mock();

    let id = Identifier::random();

    sdk.mock()
        .expect_fetch(id, None as Option<Identity>)
        .await
        .unwrap();

    let retrieved = Identity::fetch(&sdk, id)
        .await
        .expect("fetch should succeed");

    assert!(retrieved.is_none());
}

/// Given some data contract, when I fetch it by ID, I get it.
#[tokio::test]
async fn test_mock_fetch_data_contract() {
    // `mock_data_contract` builds V2 document types that only decode at the latest
    // protocol version; the unpinned default floor would downgrade the type and
    // mismatch. A proven `fetch_current` bootstrap ratchets the auto-detect SDK up
    // to the network's latest version before the data contract round-trips, exactly
    // as production converges.
    let mut sdk = SdkBuilder::new_mock()
        .build()
        .expect("mock Sdk should be created");
    bootstrap_mock_sdk_to_latest(&mut sdk).await;

    let document_type: DocumentType = mock_document_type();
    let expected = mock_data_contract(Some(&document_type));
    let id = expected.id();

    sdk.mock()
        .expect_fetch(id, Some(expected.clone()))
        .await
        .unwrap();

    let retrieved = DataContract::fetch(&sdk, id)
        .await
        .unwrap()
        .expect("object should exist");
    assert_eq!(retrieved, expected);
}

/// Given some data contract, document type name and document, when I fetch expected document using mock Sdk, I get it.
#[tokio::test]
async fn test_mock_fetch_document() {
    use dpp::document::DocumentV0Getters;

    let mut sdk = Sdk::new_mock();
    let document_type: DocumentType = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));

    let expected = document_type
        .random_document(None, sdk.version())
        .expect("document should be created");
    let document_id = expected.id();
    let document_type_name = document_type.name();

    // [DocumentQuery::new_with_data_contract_id] will fetch the data contract first, so we need to define an expectation for it.
    sdk.mock()
        .expect_fetch(data_contract.id(), Some(data_contract.clone()))
        .await
        .unwrap();

    let query =
        DocumentQuery::new_with_data_contract_id(&sdk, data_contract.id(), document_type_name)
            .await
            .expect("create document query")
            .with_document_id(&document_id);

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .unwrap();

    let retrieved = Document::fetch(&sdk, query)
        .await
        .unwrap()
        .expect("identity should exist");

    assert_eq!(retrieved, expected);
}

#[tokio::test]
/// Given the fee pots of a contract, when I fetch them by the contract id using mock API, then
/// I get the same pots, a pot paid out in epoch 0 apart from one that never was
async fn should_fetch_mocked_contract_fee_pots_by_contract_id() {
    use dash_sdk::platform::contract_fee_pots::{
        ContractFeePotLastClaim, ContractFeePotState, ContractFeePots, ContractFeePotsQuery,
    };

    let mut sdk = Sdk::new_mock();

    let contract_id = Identifier::from([7u8; 32]);
    let expected = ContractFeePots {
        owner: ContractFeePotState {
            credits: 10_000_000,
            last_claim: None,
        },
        moderators: ContractFeePotState {
            credits: u64::MAX,
            last_claim: Some(ContractFeePotLastClaim {
                epoch_index: 0,
                time_ms: 1_700_000_000_000,
                claimant_id: Identifier::from([9u8; 32]),
            }),
        },
    };

    sdk.mock()
        .expect_fetch(contract_id, Some(expected))
        .await
        .unwrap();

    // The contract id and the named query build the same request, so either finds the pots.
    let by_id = ContractFeePots::fetch(&sdk, contract_id)
        .await
        .unwrap()
        .expect("pots should exist");
    let by_query = ContractFeePots::fetch(&sdk, ContractFeePotsQuery { contract_id })
        .await
        .unwrap()
        .expect("pots should exist");

    assert_eq!(by_id, expected);
    assert_eq!(by_query, expected);
}
