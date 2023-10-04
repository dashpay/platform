//!
//!
use std::collections::BTreeMap;

use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters,
        document_type::{
            accessors::DocumentTypeV0Getters, random_document::CreateRandomDocument, DocumentType,
        },
        DataContractFacade,
    },
    document::Document,
    identity::{accessors::IdentityGettersV0, IdentityV0},
    platform_value::platform_value,
    platform_value::Value,
    prelude::{DataContract, Identifier, Identity},
    version::{PlatformVersion, PlatformVersionCurrentVersion},
};
use rs_sdk::{
    platform::{DocumentQuery, Fetch},
    Sdk,
};

include!("common.rs");

#[tokio::test]
/// Given some identity ID, when I fetch it using mock API, then I get the same identity
async fn test_mock_identity() {
    let mut api = Sdk::new_mock();

    let expected = Identity::from(IdentityV0::default());
    let id = expected.id();

    api.mock().expect_fetch(id, expected.clone());

    let retrieved = dpp::prelude::Identity::fetch(&mut api, id)
        .await
        .unwrap()
        .expect("object should exist");

    assert_eq!(retrieved, expected);
}

fn mock_document_type() -> DocumentType {
    let platform_version = PlatformVersion::get_current().unwrap();

    let schema = platform_value!({
        "type": "object",
        "properties": {
            "a": {
                "type": "string",
                "maxLength": 10,
            }
        },
        "additionalProperties": false,
    });

    DocumentType::try_from_schema(
        Identifier::random(),
        "document_type_name",
        schema,
        None,
        false,
        false,
        true,
        platform_version,
    )
    .expect("expected to create a document type")
}

fn mock_data_contract(document_type: Option<&DocumentType>) -> DataContract {
    let platform_version = PlatformVersion::latest();
    let protocol_version = platform_version.protocol_version;

    let owner_id = Identifier::from_bytes(&IDENTITY_ID_BYTES).unwrap();

    // let factory = DataContractFactory::new(protocol_version, None)
    //     .expect("expected to create a factory for get_dpns_data_contract_fixture");
    // let data_contract =factory
    //     .create_with_value_config(owner_id, document_schemas.into(), None, Some(defs))
    //     .expect("data in fixture should be correct");
    let mut document_types: BTreeMap<String, Value> = BTreeMap::new();

    if let Some(doc) = document_type {
        let schema = doc.schema();
        document_types.insert(doc.name().to_string(), schema.clone());
    }

    let data_contract = DataContractFacade::new(protocol_version, None)
        .unwrap()
        .create(owner_id, platform_value!(document_types), None, None)
        .expect("create data contract")
        .data_contract_owned();

    data_contract
}

#[tokio::test]
async fn test_mock_data_contract() {
    let mut api = Sdk::new_mock();

    let expected = mock_data_contract(None);
    let id = expected.id();

    api.mock().expect_fetch(id, expected.clone());

    let retrieved = DataContract::fetch(&mut api, id)
        .await
        .unwrap()
        .expect("object should exist");
    assert_eq!(retrieved, expected);
}

#[tokio::test]
async fn test_mock_document() {
    use dpp::document::DocumentV0Getters;

    let mut api = Sdk::new_mock();
    let document_type: DocumentType = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));

    let expected = document_type
        .random_filled_document(None, Sdk::version())
        .expect("document should be created");
    let document_id = expected.id();
    let document_type_name = document_type.name();

    // [DocumentQuery::new_with_document_id] will fetch the data contract first, so we need to define an expectation for it.
    api.mock()
        .expect_fetch(data_contract.id(), data_contract.clone());

    let query = DocumentQuery::new_with_document_id(
        &mut api,
        data_contract.id(),
        document_type_name,
        document_id,
    )
    .await
    .expect("create document query");

    api.mock().expect_fetch(query.clone(), expected.clone());

    let retrieved = Document::fetch(&mut api, query)
        .await
        .unwrap()
        .expect("identity should exist");

    assert_eq!(retrieved, expected);
}
