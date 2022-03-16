use grovedb::Error;
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

pub fn setup_family_tests(count: u32, seed: u64) -> (Drive, Contract, TempDir) {
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Records {
    dash_unique_identity_id: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Domain {
    #[serde(rename = "$id")]
    id: Vec<u8>,
    #[serde(rename = "$ownerId")]
    owner_id: Vec<u8>,
    label: String,
    normalized_label: String,
    normalized_parent_domain_name: String,
    records: Records,
    preorder_salt: Vec<u8>,
    subdomain_rules: bool,
}

impl Domain {
    fn random_domains_in_parent(
        count: u32,
        seed: u64,
        normalized_parent_domain_name: &str,
    ) -> Vec<Self> {
        let first_names =
            common::text_file_strings("tests/supporting_files/contract/family/first-names.txt");
        let mut vec: Vec<Domain> = Vec::with_capacity(count as usize);

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for i in 0..count {
            let label = first_names.choose(&mut rng).unwrap();
            let domain = Domain {
                id: Vec::from(rng.gen::<[u8; 32]>()),
                owner_id: Vec::from(rng.gen::<[u8; 32]>()),
                label: label.clone(),
                normalized_label: label.to_lowercase(),
                normalized_parent_domain_name: normalized_parent_domain_name.to_string(),
                records: Records {
                    dash_unique_identity_id: Vec::from(rng.gen::<[u8; 32]>()),
                },
                preorder_salt: Vec::from(rng.gen::<[u8; 32]>()),
                subdomain_rules: false,
            };
            vec.push(domain);
        }
        vec
    }
}

pub fn setup_dpns_tests(count: u32, seed: u64) -> (Drive, Contract, TempDir) {
    let tmp_dir = TempDir::new("dpns").unwrap();
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
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        Some(&db_transaction),
    );

    let domains = Domain::random_domains_in_parent(count, seed, "dash");
    for domain in domains {
        let value = serde_json::to_value(&domain).expect("serialized domain");
        let document_cbor =
            common::value_to_cbor(value, Some(rs_drive::drive::defaults::PROTOCOL_VERSION));
        let document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("document should be properly deserialized");
        drive
            .add_document_for_contract(
                &document,
                &document_cbor,
                &contract,
                "domain",
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
#[ignore]
fn test_query_many() {
    let (mut drive, contract, tmp_dir) = setup_family_tests(1600, 73509);
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
fn test_family_query() {
    let (mut drive, contract, tmp_dir) = setup_family_tests(10, 73509);

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

    // A query getting all people who's first name starts with C

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

    let expected_names_starting_with_c = ["Cammi".to_string(), "Celinda".to_string()];
    assert_eq!(names, expected_names_starting_with_c);

    // A query getting all people who's first name starts with C, but limit to 1 and be descending

    let query_value = json!({
        "where": [
            ["firstName", "StartsWith", "C"]
        ],
        "limit": 1,
        "orderBy": [
            ["firstName", "desc"]
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

    let expected_names_starting_with_c_desc_1 = ["Celinda".to_string()];
    assert_eq!(names, expected_names_starting_with_c_desc_1);

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

    let query_value = json!({
        "where": [
            ["firstName", "in", names]
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "desc"]
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

    let expected_reversed_between_names = [
        "Noellyn".to_string(),
        "Meta".to_string(),
        "Kevina".to_string(),
        "Gilligan".to_string(),
        "Dalia".to_string(),
    ];

    assert_eq!(names, expected_reversed_between_names);

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

    assert_eq!(results.len(), 2);

    let last_person = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
        .expect("we should be able to deserialize the cbor");

    assert_eq!(
        last_person.id,
        vec![
            76, 161, 17, 201, 152, 232, 129, 48, 168, 13, 49, 10, 218, 53, 118, 136, 165, 198, 189,
            116, 116, 22, 133, 92, 104, 165, 186, 249, 94, 81, 45, 20
        ]
        .as_slice()
    );

    // fetching by $id with order by desc

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "orderBy": [["$id", "desc"]],
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

    let last_person = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
        .expect("we should be able to deserialize the cbor");

    assert_eq!(
        last_person.id,
        vec![
            140, 161, 17, 201, 152, 232, 129, 48, 168, 13, 49, 10, 218, 53, 118, 136, 165, 198,
            189, 116, 116, 22, 133, 92, 104, 165, 186, 249, 94, 81, 45, 20
        ]
        .as_slice()
    );

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
    // // fetching with empty where and orderBy $id desc
    //
    let query_value = json!({
        "orderBy": [["$id", "desc"]]
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

    assert_eq!(results.len(), 12);

    let last_person = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
        .expect("we should be able to deserialize the cbor");

    assert_eq!(
        last_person.id,
        vec![
            249, 170, 70, 122, 181, 31, 35, 176, 175, 131, 70, 150, 250, 223, 194, 203, 175, 200,
            107, 252, 199, 227, 154, 105, 89, 57, 38, 85, 236, 192, 254, 88
        ]
        .as_slice()
    );

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

    // query empty contract with nested path queries

    let contract_cbor = hex::decode("01000000a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

    drive
        .apply_contract(contract_cbor.clone(), Some(&db_transaction))
        .expect("expected to apply contract successfully");

    let query_value = json!({
        "where": [
            ["$ownerId", "==", "BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj"],
            ["toUserId", "==", "BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj"],
        ],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let (results, _) = drive
        .query_documents_from_contract_cbor(
            contract_cbor.as_slice(),
            String::from("contact"),
            query_cbor.as_slice(),
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 0);

    // using non existing document in startAt

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("5A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF178")]],
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

    assert_eq!(results.len(), 1);

    // using non existing document in startAt

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "startAt": String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF178"),
        "orderBy": [["$id", "asc"]],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let result = drive.query_documents_from_contract(
        &contract,
        person_document_type,
        query_cbor.as_slice(),
        Some(&db_transaction),
    );

    assert!(
        matches!(result, Err(Error::InvalidQuery(message)) if message == "startAt document not found")
    );

    // using non existing document in startAfter

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "startAfter": String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF178"),
        "orderBy": [["$id", "asc"]],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let result = drive.query_documents_from_contract(
        &contract,
        person_document_type,
        query_cbor.as_slice(),
        Some(&db_transaction),
    );

    assert!(
        matches!(result, Err(Error::InvalidQuery(message)) if message == "startAfter document not found")
    );

    // validate eventual root hash

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .expect("there is always a root hash");
    assert_eq!(
        root_hash.as_slice(),
        vec![
            10, 188, 94, 203, 19, 207, 167, 237, 34, 159, 113, 240, 54, 221, 246, 234, 48, 238, 82,
            222, 228, 119, 192, 92, 57, 3, 28, 78, 115, 35, 133, 50
        ]
    );
}

#[test]
fn test_sql_query() {
    // These tests confirm that sql statements produce the same drive query
    // as their json counterparts, tests above confirm that the json queries
    // produce the correct result set
    let (_, contract, tmp_dir) = setup_family_tests(10, 73509);
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

    // In clause
    let names = vec![String::from("a"), String::from("b")];
    let query_cbor = common::value_to_cbor(
        json!({
            "where": [
                ["firstName", "in", names]
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"]
            ],
        }),
        None,
    );
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, &person_document_type)
        .expect("should build query");

    let sql_string =
        "select * from person where firstName in ('a', 'b') order by firstName limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);
}

#[test]
fn test_dpns_query() {
    let (mut drive, contract, tmp_dir) = setup_dpns_tests(10, 11456);

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
            57, 11, 30, 250, 150, 197, 183, 79, 207, 232, 221, 217, 161, 151, 74, 66, 129, 31, 126,
            111, 232, 47, 94, 84, 246, 13, 195, 76, 72, 6, 229, 33
        ]
    );

    let all_names = [
        "amalle".to_string(),
        "anna-diane".to_string(),
        "atalanta".to_string(),
        "eden".to_string(),
        "laureen".to_string(),
        "leone".to_string(),
        "marilyn".to_string(),
        "minna".to_string(),
        "mora".to_string(),
        "phillie".to_string(),
    ];

    // A query getting all elements by firstName

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"]
        ],
        "limit": 100,
        "orderBy": [
            ["normalizedLabel", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &domain_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let normalized_label_value = document
                .properties
                .get("normalizedLabel")
                .expect("we should be able to get the first name");
            let normalized_label = normalized_label_value
                .as_text()
                .expect("the normalized label should be a string");
            String::from(normalized_label)
        })
        .collect();

    assert_eq!(names, all_names);

    // A query getting all elements starting with a in dash parent domain

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"],
            ["normalizedLabel", "startsWith", "a"]
        ],
        "limit": 5,
        "orderBy": [
            ["normalizedLabel", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &domain_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .clone()
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let normalized_label_value = document
                .properties
                .get("normalizedLabel")
                .expect("we should be able to get the first name");
            let normalized_label = normalized_label_value
                .as_text()
                .expect("the normalized label should be a string");
            String::from(normalized_label)
        })
        .collect();

    let a_names = [
        "amalle".to_string(),
        "anna-diane".to_string(),
        "atalanta".to_string(),
    ];

    assert_eq!(names, a_names);

    let ids: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            hex::encode(document.id)
        })
        .collect();

    let a_ids = [
        "61978359176813a3e9b79c07df8addda2aea3841cfff2afe5b23cf1b5b926c1b".to_string(),
        "0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080".to_string(),
        "26a9344b6d0fcf8f525dfc160c160a7a52ef3301a7e55fccf41d73857f50a55a".to_string(),
    ];

    assert_eq!(ids, a_ids);

    // A query getting one element starting with a in dash parent domain asc

    let anna_id = hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
        .expect("expected to decode id");
    let encoded_start_at = bs58::encode(anna_id).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"],
            ["normalizedLabel", "startsWith", "a"]
        ],
        "startAt":  encoded_start_at,
        "limit": 1,
        "orderBy": [
            ["normalizedLabel", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &domain_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let normalized_label_value = document
                .properties
                .get("normalizedLabel")
                .expect("we should be able to get the first name");
            let normalized_label = normalized_label_value
                .as_text()
                .expect("the normalized label should be a string");
            String::from(normalized_label)
        })
        .collect();

    let a_names = ["anna-diane".to_string()];

    assert_eq!(names, a_names);

    // A query getting one element starting with a in dash parent domain desc

    let anna_id = hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
        .expect("expected to decode id");
    let encoded_start_at = bs58::encode(anna_id.clone()).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"],
            ["normalizedLabel", "startsWith", "a"]
        ],
        "startAt":  encoded_start_at,
        "limit": 1,
        "orderBy": [
            ["normalizedLabel", "desc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &domain_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .clone()
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let normalized_label_value = document
                .properties
                .get("normalizedLabel")
                .expect("we should be able to get the first name");
            let normalized_label = normalized_label_value
                .as_text()
                .expect("the normalized label should be a string");
            String::from(normalized_label)
        })
        .collect();

    let a_names = ["anna-diane".to_string()];

    assert_eq!(names, a_names);

    let record_id_base64: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");

            let records_value = document
                .properties
                .get("records")
                .expect("we should be able to get the records");
            let map_records_value = records_value.as_map().expect("this should be a map");
            let record_dash_unique_identity_id = rs_drive::contract::cbor_inner_bytes_value(
                map_records_value,
                "dashUniqueIdentityId",
            )
            .expect("there should be a dashUniqueIdentityId");
            base64::encode(record_dash_unique_identity_id)
        })
        .collect();

    let a_record_id_base64 = ["RdBiF9ph2C6C4dRhT9C/xVSoOxb+uvduuLlT/0EEDZA=".to_string()];

    assert_eq!(record_id_base64, a_record_id_base64);

    // A query getting elements by the dashUniqueIdentityId desc

    let query_value = json!({
        "where": [
            ["records.dashUniqueIdentityId", "<=", "RdBiF9ph2C6C4dRhT9C/xVSoOxb+uvduuLlT/0EEDZA="],
        ],
        "limit": 10,
        "orderBy": [
            ["records.dashUniqueIdentityId", "desc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &domain_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let normalized_label_value = document
                .properties
                .get("normalizedLabel")
                .expect("we should be able to get the first name");
            let normalized_label = normalized_label_value
                .as_text()
                .expect("the normalized label should be a string");
            String::from(normalized_label)
        })
        .collect();

    let a_names = [
        "anna-diane".to_string(),
        "marilyn".to_string(),
        "minna".to_string(),
    ];

    assert_eq!(names, a_names);

    // A query getting 2 elements asc by the dashUniqueIdentityId

    let query_value = json!({
        "where": [
            ["records.dashUniqueIdentityId", "<=", "RdBiF9ph2C6C4dRhT9C/xVSoOxb+uvduuLlT/0EEDZA="],
        ],
        "limit": 2,
        "orderBy": [
            ["records.dashUniqueIdentityId", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &domain_document_type)
        .expect("query should be built");
    let (results, _) = query
        .execute_no_proof(&mut drive.grove, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let normalized_label_value = document
                .properties
                .get("normalizedLabel")
                .expect("we should be able to get the first name");
            let normalized_label = normalized_label_value
                .as_text()
                .expect("the normalized label should be a string");
            String::from(normalized_label)
        })
        .collect();

    let a_names = ["minna".to_string(), "marilyn".to_string()];

    assert_eq!(names, a_names);
}
