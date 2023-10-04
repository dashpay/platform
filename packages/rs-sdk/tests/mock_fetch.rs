use std::collections::BTreeMap;

use dpp::{
    data_contract::{accessors::v0::DataContractV0Getters, DataContractFacade},
    identity::{accessors::IdentityGettersV0, IdentityV0},
    platform_value::platform_value,
    platform_value::Value,
    prelude::{DataContract, Identifier, Identity},
    version::PlatformVersion,
};
use rs_sdk::platform::Fetch;

include!("common.rs");

#[tokio::test]
async fn test_mock_identity() {
    let mut api: rs_sdk::Sdk = setup_mock_api().await;

    let expected = Identity::from(IdentityV0::default());
    let id = expected.id();

    api.mock().expect_fetch(id, expected.clone());

    let retrieved = dpp::prelude::Identity::fetch(&mut api, id)
        .await
        .unwrap()
        .expect("object should exist");

    assert_eq!(retrieved, expected);
}

// #[tokio::test]
// async fn test_mock_document() {
//     use dpp::document::DocumentV0Getters;

//     let mut api: rs_sdk::Sdk = setup_mock_api().await;

//     let expected = Document::V0(DocumentV0::default());
//     let id = expected.id();
//     document_type_name = expected.document_type_name();
//     DocumentQuery::new_with_document_id(
//         &mut api,
//         data_contract_id,
//         document_type_name,
//         document_id,
//     );

//     api.mock().expect_fetch(id, expected);

//     let identity = dpp::prelude::Identity::fetch(&mut api, id)
//         .await
//         .unwrap()
//         .expect("identity should exist");

//     assert_eq!(identity.id(), &id);
// }

#[tokio::test]
async fn test_mock_data_contract() {
    let mut api: rs_sdk::Sdk = setup_mock_api().await;
    let platform_version = PlatformVersion::latest();
    let protocol_version = platform_version.protocol_version;

    let owner_id = Identifier::from_bytes(&IDENTITY_ID_BYTES).unwrap();

    let document = platform_value!(BTreeMap::<String, Value>::new());
    let expected = DataContractFacade::new(protocol_version, None)
        .unwrap()
        .create(owner_id, document, None, None)
        .expect("create data contract")
        .data_contract_owned();

    let id = expected.id();

    api.mock().expect_fetch(id, expected.clone());

    let retrieved = DataContract::fetch(&mut api, id)
        .await
        .unwrap()
        .expect("object should exist");
    assert_eq!(retrieved, expected);
}
