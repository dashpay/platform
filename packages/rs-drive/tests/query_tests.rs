use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rs_drive::common;
use rs_drive::contract::{Contract, Document};
use rs_drive::drive::Drive;
use rs_drive::query::DriveQuery;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tempdir::TempDir;

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
        let mut vec: Vec<Person> = Vec::with_capacity(count as usize);

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for _ in 0..count {
            let person = Person {
                id: Vec::from(rng.gen::<[u8; 32]>()),
                owner_id: Vec::from(rng.gen::<[u8; 32]>()),
                first_name: first_names.choose(&mut rng).unwrap().clone(),
                middle_name: middle_names.choose(&mut rng).unwrap().clone(),
                last_name: last_names.choose(&mut rng).unwrap().clone(),
                age: rng.gen_range(0..85),
            };
            vec.push(person);
        }
        vec
    }
}

pub fn setup(count: u32, seed: u64) -> (Drive, Contract, TempDir) {
    let tmp_dir = TempDir::new("family").unwrap();
    let mut drive: Drive = Drive::open(&tmp_dir).expect("expected to open Drive successfully");

    let storage = drive.grove.storage();
    let db_transaction = storage.transaction();
    drive
        .grove
        .start_transaction()
        .expect("expected to start transaction successfully");

    drive
        .create_root_tree(Some(&db_transaction))
        .expect("expected to create root tree successfully");

    // setup code
    let contract = common::setup_contract(
        &mut drive,
        "tests/supporting_files/contract/family/family-contract.json",
        Some(&db_transaction),
    );

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
    drive
        .grove
        .commit_transaction(db_transaction)
        .expect("transaction should be committed");
    (drive, contract, tmp_dir)
}

#[test]
fn test_query_many() {
    let (mut drive, contract, tmp_dir) = setup(1600, 73509);
    let storage = drive.grove.storage();
    let db_transaction = storage.transaction();
    let people = Person::random_people(10, 73409);
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
    drive
        .grove
        .commit_transaction(db_transaction)
        .expect("transaction should be committed");
}

#[test]
fn test_query() {
    let (mut drive, contract, tmp_dir) = setup(10, 73509);

    let storage = drive.grove.storage();
    let db_transaction = storage.transaction();
    drive
        .grove
        .start_transaction()
        .expect("expected to start transaction");

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .expect("there is always a root hash");
    assert_eq!(
        root_hash.as_slice(),
        vec![
            164, 56, 26, 188, 12, 251, 247, 43, 109, 153, 109, 110, 78, 131, 37, 79, 19, 178, 159,
            69, 35, 250, 159, 210, 2, 125, 12, 103, 50, 40, 108, 114
        ]
    );

    let all_names = [
        "Adey".to_string(),
        "Briney".to_string(),
        "Cammi".to_string(),
        "Celinda".to_string(),
        "Dalia".to_string(),
        "Gilligan".to_string(),
        "Kevina".to_string(),
        "Meta".to_string(),
        "Noellyn".to_string(),
        "Prissie".to_string(),
    ];

    // A query getting all elements by firstName

    let query_value = json!({
        "where": [
        ],
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
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    assert_eq!(names, all_names);

    // A query getting all people who's first name is Chris (which should exist)

    let query_value = json!({
        "where": [
            ["firstName", "==", "Adey"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(&contract, person_document_type, query_cbor.as_slice(), None)
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // A query getting all people who's first name is Adey and lastName Randolf

    let query_value = json!({
        "where": [
            ["firstName", "==", "Adey"],
            ["lastName", "==", "Randolf"]
        ],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(&contract, person_document_type, query_cbor.as_slice(), None)
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let document = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
        .expect("we should be able to deserialize the cbor");
    let last_name = document
        .properties
        .get("lastName")
        .expect("we should be able to get the last name")
        .as_text()
        .expect("last name must be a string");

    assert_eq!(last_name, "Randolf");

    // A query getting all people who's first name is in a range with a single element Adey,
    // order by lastName (this should exist)

    let query_value = json!({
        "where": [
            ["firstName", "in", ["Adey"]]
        ],
        "orderBy": [
            ["firstName", "asc"],
            ["lastName", "asc"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(&contract, person_document_type, query_cbor.as_slice(), None)
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // A query getting all people who's first name is Adey, order by lastName (which should exist)

    let query_value = json!({
        "where": [
            ["firstName", "==", "Adey"]
        ],
        "orderBy": [
            ["lastName", "asc"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(&contract, person_document_type, query_cbor.as_slice(), None)
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let document = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
        .expect("we should be able to deserialize the cbor");
    let last_name = document
        .properties
        .get("lastName")
        .expect("we should be able to get the last name")
        .as_text()
        .expect("last name must be a string");

    assert_eq!(last_name, "Randolf");

    // A query getting all people who's first name is Chris (which is not exist)

    let query_value = json!({
        "where": [
            ["firstName", "==", "Chris"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(&contract, person_document_type, query_cbor.as_slice(), None)
        .expect("query should be executed");

    assert_eq!(results.len(), 0);

    // A query getting a middle name

    let query_value = json!({
        "where": [
            ["middleName", "==", "Briggs"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(&contract, person_document_type, query_cbor.as_slice(), None)
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // A query getting all people who's first name is before Chris

    let query_value = json!({
        "where": [
            ["firstName", "<", "Chris"]
        ],
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
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_names_before_chris = [
        "Adey".to_string(),
        "Briney".to_string(),
        "Cammi".to_string(),
        "Celinda".to_string(),
    ];
    assert_eq!(names, expected_names_before_chris);

    // A query getting all people who's first name is before Chris

    let query_value = json!({
        "where": [
            ["firstName", "StartsWith", "C"]
        ],
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
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_names_before_chris = ["Cammi".to_string(), "Celinda".to_string()];
    assert_eq!(names, expected_names_before_chris);

    // A query getting all people who's first name is between Chris and Noellyn included

    let query_value = json!({
        "where": [
            ["firstName", ">", "Chris"],
            ["firstName", "<=", "Noellyn"]
        ],
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
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    assert_eq!(results.len(), 5);

    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_between_names = [
        "Dalia".to_string(),
        "Gilligan".to_string(),
        "Kevina".to_string(),
        "Meta".to_string(),
        "Noellyn".to_string(),
    ];

    assert_eq!(names, expected_between_names);

    // A query getting all people who's first name is between Chris and Noellyn included
    // However here there will be a startAt of the ID of Kevina

    // Let's first get the ID of Kevina
    let ids: HashMap<String, Vec<u8>> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let name = name_value
                .as_text()
                .expect("the first name should be a string")
                .to_string();
            (name, Vec::from(document.id))
        })
        .collect();

    let kevina_id = ids
        .get("Kevina")
        .expect("We should be able to get back Kevina's Id");
    let kevina_encoded_id = bs58::encode(kevina_id).into_string();

    let query_value = json!({
        "where": [
            ["firstName", ">", "Chris"],
            ["firstName", "<=", "Noellyn"]
        ],
        "startAt": kevina_encoded_id, //Kevina
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
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    assert_eq!(results.len(), 3);

    let reduced_names_after: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_reduced_names = [
        "Kevina".to_string(),
        "Meta".to_string(),
        "Noellyn".to_string(),
    ];

    assert_eq!(reduced_names_after, expected_reduced_names);

    // Now lets try startsAfter

    let query_value = json!({
        "where": [
            ["firstName", ">", "Chris"],
            ["firstName", "<=", "Noellyn"]
        ],
        "startAfter": kevina_encoded_id, //Kevina
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
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    assert_eq!(results.len(), 2);

    let reduced_names_after: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_reduced_names = ["Meta".to_string(), "Noellyn".to_string()];

    assert_eq!(reduced_names_after, expected_reduced_names);

    // A query getting back elements having specific names

    let query_value = json!({
        "where": [
            ["firstName", "in", names]
        ],
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
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
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
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_names_45_over = [
        "Dalia".to_string(),
        "Gilligan".to_string(),
        "Kevina".to_string(),
        "Meta".to_string(),
    ];

    assert_eq!(names, expected_names_45_over);

    // A query getting back elements having specific names and over a certain age

    let query_value = json!({
        "where": [
            ["firstName", "in", names],
            ["age", ">", 48]
        ],
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
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    // Kevina is 48 so she should be now excluded, Dalia is 68, Gilligan is 49 and Meta is 59

    let expected_names_over_48 = [
        "Dalia".to_string(),
        "Gilligan".to_string(),
        "Meta".to_string(),
    ];

    assert_eq!(names, expected_names_over_48);

    let ages: HashMap<String, u8> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let name = name_value
                .as_text()
                .expect("the first name should be a string")
                .to_string();
            let age_value = document
                .properties
                .get("age")
                .expect("we should be able to get the age");
            let age_integer = age_value.as_integer().expect("age should be an integer");
            let age: u8 = age_integer.try_into().expect("expected u8 value");
            (name, age)
        })
        .collect();

    let meta_age = ages
        .get("Meta")
        .expect("we should be able to get Kevina as she is 48");

    assert_eq!(*meta_age, 59);

    // fetching by $id
    let mut rng = rand::rngs::StdRng::seed_from_u64(84594);
    let id_bytes = bs58::decode("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD")
        .into_vec()
        .expect("this should decode");

    let owner_id_bytes = bs58::decode("BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj")
        .into_vec()
        .expect("this should decode");

    let fixed_person = Person {
        id: id_bytes,
        owner_id: owner_id_bytes,
        first_name: String::from("Wisdom"),
        middle_name: String::from("Madabuchukwu"),
        last_name: String::from("Ogwu"),
        age: rng.gen_range(0..85),
    };
    let serialized_person = serde_json::to_value(&fixed_person).expect("serialized person");
    let person_cbor = common::value_to_cbor(
        serialized_person,
        Some(rs_drive::drive::defaults::PROTOCOL_VERSION),
    );
    let document = Document::from_cbor(person_cbor.as_slice(), None, None)
        .expect("document should be properly deserialized");

    drive
        .add_document_for_contract(
            &document,
            &person_cbor,
            &contract,
            "person",
            None,
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    let id_two_bytes = bs58::decode("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")
        .into_vec()
        .expect("should decode");
    let owner_id_bytes = bs58::decode("Di8dtJXv3L2YnzDNUN4w5rWLPSsSAzv6hLMMQbg3eyVA")
        .into_vec()
        .expect("this should decode");
    let next_person = Person {
        id: id_two_bytes,
        owner_id: owner_id_bytes,
        first_name: String::from("Wdskdfslgjfdlj"),
        middle_name: String::from("Mdsfdsgsdl"),
        last_name: String::from("dkfjghfdk"),
        age: rng.gen_range(0..85),
    };
    let serialized_person = serde_json::to_value(&next_person).expect("serialized person");
    let person_cbor = common::value_to_cbor(
        serialized_person,
        Some(rs_drive::drive::defaults::PROTOCOL_VERSION),
    );
    let document = Document::from_cbor(person_cbor.as_slice(), None, None)
        .expect("document should be properly deserialized");

    drive
        .add_document_for_contract(
            &document,
            &person_cbor,
            &contract,
            "person",
            None,
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    let query_value = json!({
        "where": [
            ["$id", "in", vec![String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            Some(&db_transaction),
        )
        .expect("query should be executed");

    // dbg!(&results);
    assert_eq!(results.len(), 1);

    // fetching by $id with order by

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "orderBy": [["$id", "asc"]],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            Some(&db_transaction),
        )
        .expect("query should be executed");
    // dbg!(&results);

    assert_eq!(results.len(), 2);
    //
    // // fetching with empty where and orderBy
    //
    let query_value = json!({});

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 12);

    //
    // // fetching with ownerId in a set of values
    //
    let query_value = json!({
        "where": [
            ["$ownerId", "in", ["BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj", "Di8dtJXv3L2YnzDNUN4w5rWLPSsSAzv6hLMMQbg3eyVA"]]
        ],
        "orderBy": [["$ownerId", "desc"]]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 2);

    //
    // // fetching with ownerId equal and orderBy
    //
    let query_value = json!({
        "where": [
            ["$ownerId", "==", "BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj"]
        ],
        "orderBy": [["$ownerId", "asc"]]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .expect("there is always a root hash");
    assert_eq!(
        root_hash.as_slice(),
        vec![
            84, 219, 205, 67, 253, 96, 148, 19, 207, 81, 77, 219, 252, 21, 109, 59, 247, 36, 196,
            182, 199, 250, 174, 118, 180, 251, 127, 165, 73, 206, 63, 127
        ]
    );
}

#[test]
fn test_sql_query() {
    // These tests confirm that sql statements produce the same drive query
    // as their json counterparts, tests above confirm that the json queries
    // produce the correct result set
    let (_, contract, tmp_dir) = setup(10, 73509);
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    // Empty where clause
    let query_cbor = common::value_to_cbor(
        json!({
            "where": [],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"]
            ]
        }),
        None,
    );
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, &person_document_type)
        .expect("should build query");

    let sql_string = "select * from person order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);

    // Equality clause
    let query_cbor = common::value_to_cbor(
        json!({
            "where": [
                ["firstName", "==", "Chris"]
            ]
        }),
        None,
    );
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, &person_document_type)
        .expect("should build query");

    let sql_string = "select * from person where firstName = 'Chris'";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);

    // Less than
    let query_cbor = common::value_to_cbor(
        json!({
            "where": [
                ["firstName", "<", "Chris"]
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"]
            ]
        }),
        None,
    );
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, &person_document_type)
        .expect("should build query");

    let sql_string =
        "select * from person where firstName < 'Chris' order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);

    // Starts with
    let query_cbor = common::value_to_cbor(
        json!({
            "where": [
                ["firstName", "StartsWith", "C"]
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"]
            ]
        }),
        None,
    );
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, &person_document_type)
        .expect("should build query");

    let sql_string =
        "select * from person where firstName like 'C%' order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);

    // Range combination
    let query_cbor = common::value_to_cbor(
        json!({
            "where": [
                ["firstName", ">", "Chris"],
                ["firstName", "<=", "Noellyn"]
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"]
            ]
        }),
        None,
    );
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, &person_document_type)
        .expect("should build query");

    let sql_string = "select * from person where firstName > 'Chris' and firstName <= 'Noellyn' order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);
}
