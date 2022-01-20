use grovedb::{Error, Query};
use rand::Rng;
use tempdir::TempDir;
use rs_drive::contract::{Contract, Document, DocumentType};
use rs_drive::drive::Drive;
use rs_drive::query::DriveQuery;
use serde::{Deserialize, Serialize};
use rand::seq::SliceRandom;
use serde_json::json;

mod common;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Person {
    #[serde(rename = "$id")]
    id: Vec<u8>,
    #[serde(rename = "$ownerId")]
    owner_id: Vec<u8>,
    first_name: String,
    middle_name: String,
    last_name: String,
    age: u8,
}

impl Person {
    fn random_people(count: u32) -> Vec<Self> {
        let first_names = common::text_file_strings("tests/supporting_files/contract/family/first-names.txt");
        let middle_names = common::text_file_strings("tests/supporting_files/contract/family/middle-names.txt");
        let last_names = common::text_file_strings("tests/supporting_files/contract/family/last-names.txt");
        let mut vec : Vec<Person> = vec![];
        for i in 0..count {
            let person = Person {
                id: Vec::from(rand::thread_rng().gen::<[u8; 32]>()),
                owner_id: Vec::from(rand::thread_rng().gen::<[u8; 32]>()),
                first_name: first_names.choose(&mut rand::thread_rng()).unwrap().clone(),
                middle_name: middle_names.choose(&mut rand::thread_rng()).unwrap().clone(),
                last_name: last_names.choose(&mut rand::thread_rng()).unwrap().clone(),
                age: rand::thread_rng().gen_range(0..85),
            };
            vec.push(person);
        }
        vec
    }
}

pub fn setup() -> (Drive, Contract) {
    // setup code
    let (mut drive, contract) = common::setup_contract("family", "tests/supporting_files/contract/family/family-contract.json");

    let storage = drive.grove.storage();
    let db_transaction = storage.transaction();
    drive.grove.start_transaction();

    let people = Person::random_people(10);
    for person in people {
        let value = serde_json::to_value(&person).expect("serialized person");
        let document_cbor = common::value_to_cbor(value, Some(rs_drive::drive::defaults::PROTOCOL_VERSION));
        let document = Document::from_cbor(document_cbor.as_slice(), None, None).expect("document should be properly deserialized");
        drive.add_document_for_contract(&document, &document_cbor, &contract, "person", None, true, Some(&db_transaction)).expect("document should be inserted");
    }
    drive.grove.commit_transaction(db_transaction);
    (drive, contract)
}

#[test]
fn test_query_many() {
    let (mut drive, contract) = setup();
    // let query_value = json!({
    //     "where": [
    //         ["firstName", ">", "Abe"]
    //     ],
    //     "startAt": 0,
    //     "limit": 100,
    //     "orderBy": ["firstName", "asc"]
    // });
    // let where_cbor = common::value_to_cbor(query_value);
    // let person_document_type = contract.document_types.get("person").expect("contract should have a person document type");
    // let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &person_document_type).expect("query should be built");
    // let (results, skipped) = query.execute_no_proof(&mut drive.grove, None).expect("proof should be executed");
    // assert!(results.len() > 50);

    let query_value = json!({
        "where": [
            ["firstName", ">", "Kiara"],
            ["firstName", "<", "Sam"]
        ],
        "startAt": 0,
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract.document_types.get("person").expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &person_document_type).expect("query should be built");
    let (results, skipped) = query.execute_no_proof(&mut drive.grove, None).expect("proof should be executed");
     assert_ne!(results.len(), 100);

    let names : Result<Vec<Document>, Error> = results.into_iter().map(|result| {
        Document::from_cbor(result.as_slice(), None, None)
    }).collect();

    let query_value = json!({
        "where": [
            ["firstName", "in", "Sam"],
            ["age", ">", 30]
        ],
        "startAt": 0,
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"],
            ["age", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract.document_types.get("person").expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &person_document_type).expect("query should be built");
    let (results, skipped) = query.execute_no_proof(&mut drive.grove, None).expect("proof should be executed");
    assert_eq!(results.len(), 100);
}