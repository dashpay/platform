use std::collections::HashMap;
use grovedb::{Error, Query};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_distr::{Exp, Distribution};
use rs_drive::contract::{Contract, Document, DocumentType};
use rs_drive::drive::Drive;
use rs_drive::query::DriveQuery;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tempdir::TempDir;

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
    fn random_people(count: u32, seed: u64) -> Vec<Self> {
        let first_names =
            common::text_file_strings("tests/supporting_files/contract/family/first-names.txt");
        let middle_names =
            common::text_file_strings("tests/supporting_files/contract/family/middle-names.txt");
        let last_names =
            common::text_file_strings("tests/supporting_files/contract/family/last-names.txt");
        let mut vec: Vec<Person> = vec![];

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for i in 0..count {
            let person = Person {
                id: Vec::from(rng.gen::<[u8; 32]>()),
                owner_id: Vec::from(rng.gen::<[u8; 32]>()),
                first_name: first_names.choose(&mut rng).unwrap().clone(),
                middle_name: middle_names
                    .choose(&mut rng)
                    .unwrap()
                    .clone(),
                last_name: last_names.choose(&mut rng).unwrap().clone(),
                age: rng.gen_range(0..85),
            };
            vec.push(person);
        }
        vec
    }
}

pub fn setup(count: u32, seed : u64) -> (Drive, Contract) {
    // setup code
    let (mut drive, contract) = common::setup_contract(
        "family",
        "tests/supporting_files/contract/family/family-contract.json",
    );

    let storage = drive.grove.storage();
    let db_transaction = storage.transaction();
    drive.grove.start_transaction();

    let people = Person::random_people(count, seed);
    for person in people {
        let value = serde_json::to_value(&person).expect("serialized person");
        let document_cbor =
            common::value_to_cbor(value, Some(rs_drive::drive::defaults::PROTOCOL_VERSION));
        let document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("document should be properly deserialized");
        drive
            .add_document_for_contract(
                &document,
                &document_cbor,
                &contract,
                "person",
                None,
                true,
                Some(&db_transaction),
            )
            .expect("document should be inserted");
    }
    drive.grove.commit_transaction(db_transaction);
    (drive, contract)
}

#[test]
fn test_query_many() {
    let (mut drive, contract) = setup(10, 73509);
    let all_names = vec!["Adey".to_string(), "Briney".to_string(), "Cammi".to_string(), "Celinda".to_string(), "Dalia".to_string(), "Gilligan".to_string(), "Kevina".to_string(), "Meta".to_string(), "Noellyn".to_string(), "Prissie".to_string()];

    // A query getting all elements by firstName

    let query_value = json!({
        "where": [
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
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None).expect("we should be able to deserialize the cbor");
            let first_name_value = document.properties.get("firstName").expect("we should be able to get the first name");
            let first_name = first_name_value.as_text().expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    assert_eq!(names, all_names);

    // A query getting all people who's first name is before Chris

    let query_value = json!({
        "where": [
            ["firstName", "<", "Chris"]
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
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None).expect("we should be able to deserialize the cbor");
            let first_name_value = document.properties.get("firstName").expect("we should be able to get the first name");
            let first_name = first_name_value.as_text().expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_names_before_chris = vec!["Adey".to_string(), "Briney".to_string(), "Cammi".to_string(), "Celinda".to_string()];
    assert_eq!(names, expected_names_before_chris);

    // A query getting all people who's first name is before Chris

    let query_value = json!({
        "where": [
            ["firstName", "StartsWith", "C"]
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
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None).expect("we should be able to deserialize the cbor");
            let first_name_value = document.properties.get("firstName").expect("we should be able to get the first name");
            let first_name = first_name_value.as_text().expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_names_before_chris = vec!["Cammi".to_string(), "Celinda".to_string()];
    assert_eq!(names, expected_names_before_chris);

    // A query getting all people who's first name is between Chris and Noellyn included

    let query_value = json!({
        "where": [
            ["firstName", ">", "Chris"],
            ["firstName", "<=", "Noellyn"]
        ],
        "startAt": 0,
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &person_document_type)
        .expect("query should be built");
    let (results, skipped) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    assert_eq!(results.len(), 5);

    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None).expect("we should be able to deserialize the cbor");
            let first_name_value = document.properties.get("firstName").expect("we should be able to get the first name");
            let first_name = first_name_value.as_text().expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_between_names = vec!["Dalia".to_string(), "Gilligan".to_string(), "Kevina".to_string(), "Meta".to_string(), "Noellyn".to_string()];

    assert_eq!(names, expected_between_names);

    // A query getting back elements having specific names

    let query_value = json!({
        "where": [
            ["firstName", "in", names]
        ],
        "startAt": 0,
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &person_document_type)
        .expect("query should be built");
    let (results, skipped) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None).expect("we should be able to deserialize the cbor");
            let first_name_value = document.properties.get("firstName").expect("we should be able to get the first name");
            let first_name = first_name_value.as_text().expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    assert_eq!(names, expected_between_names);

    // A query getting back elements having specific names and over a certain age

    let query_value = json!({
        "where": [
            ["firstName", "in", names],
            ["age", ">=", 45]
        ],
        "startAt": 0,
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"],
            ["age", "desc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &person_document_type)
        .expect("query should be built");
    let (results, skipped) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None).expect("we should be able to deserialize the cbor");
            let first_name_value = document.properties.get("firstName").expect("we should be able to get the first name");
            let first_name = first_name_value.as_text().expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_names_45_over = vec!["Dalia".to_string(), "Gilligan".to_string(), "Kevina".to_string(), "Meta".to_string()];

    assert_eq!(names, expected_names_45_over);

    // A query getting back elements having specific names and over a certain age

    let query_value = json!({
        "where": [
            ["firstName", "in", names],
            ["age", ">", 48]
        ],
        "startAt": 0,
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"],
            ["age", "desc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &person_document_type)
        .expect("query should be built");
    let (results, skipped) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None).expect("we should be able to deserialize the cbor");
            let first_name_value = document.properties.get("firstName").expect("we should be able to get the first name");
            let first_name = first_name_value.as_text().expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    // Kevina is 48 so she should be now excluded, Dalia is 68, Gilligan is 49 and Meta is 59

    let expected_names_over_48 = vec!["Dalia".to_string(), "Gilligan".to_string(), "Meta".to_string()];

    assert_eq!(names, expected_names_over_48);

    let ages: HashMap<String,u8> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None).expect("we should be able to deserialize the cbor");
            let name_value = document.properties.get("firstName").expect("we should be able to get the first name");
            let name = name_value.as_text().expect("the first name should be a string").to_string();
            let age_value = document.properties.get("age").expect("we should be able to get the age");
            let age_integer = age_value.as_integer().expect("age should be an integer");
            let age: u8 = age_integer.try_into().expect("expected u8 value");
            (name, age)
        })
        .collect();

    let meta_age = ages.get("Meta").expect("we should be able to get Kevina as she is 48");

    assert_eq!(*meta_age, 59)
}
