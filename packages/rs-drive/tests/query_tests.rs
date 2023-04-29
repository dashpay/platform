// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Query Tests
//!

use ciborium::cbor;
#[cfg(feature = "full")]
use grovedb::TransactionArg;
#[cfg(feature = "full")]
use std::borrow::Cow;
#[cfg(feature = "full")]
use std::collections::HashMap;
#[cfg(feature = "full")]
use std::fs::File;
#[cfg(feature = "full")]
use std::io::{self, BufRead};
#[cfg(feature = "full")]
use std::option::Option::None;
#[cfg(feature = "full")]
use std::sync::Arc;

#[cfg(feature = "full")]
use dpp::data_contract::DataContractFactory;
#[cfg(feature = "full")]
use rand::seq::SliceRandom;
#[cfg(feature = "full")]
use rand::{Rng, SeedableRng};
#[cfg(feature = "full")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "full")]
use serde_json::json;
#[cfg(feature = "full")]
use tempfile::TempDir;

#[cfg(feature = "full")]
use drive::common;
#[cfg(feature = "full")]
use drive::common::setup_contract;
#[cfg(feature = "full")]
use drive::drive::batch::GroveDbOpBatch;
#[cfg(feature = "full")]
use drive::drive::config::DriveConfig;
#[cfg(feature = "full")]
use drive::drive::contract::add_init_contracts_structure_operations;
#[cfg(feature = "full")]
use drive::drive::flags::StorageFlags;
#[cfg(feature = "full")]
use drive::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
#[cfg(feature = "full")]
use drive::drive::Drive;
#[cfg(feature = "full")]
use drive::error::{query::QueryError, Error};
#[cfg(feature = "full")]
use drive::query::DriveQuery;
#[cfg(feature = "full")]
#[cfg(test)]
use drive::tests::helpers::setup::setup_drive;

#[cfg(feature = "full")]
use dpp::data_contract::validation::data_contract_validator::DataContractValidator;
#[cfg(feature = "full")]
use dpp::document::Document;
#[cfg(feature = "full")]
use dpp::platform_value::Value;
use dpp::platform_value::{platform_value, Bytes32, Identifier};

#[cfg(feature = "full")]
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::extra::common::json_document_to_contract;
use dpp::platform_value;

#[cfg(feature = "full")]
use dpp::prelude::DataContract;
use dpp::prelude::Revision;
#[cfg(feature = "full")]
use dpp::util::cbor_serializer;
#[cfg(feature = "full")]
use dpp::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
#[cfg(feature = "full")]
use drive::contract::Contract;
use drive::drive::defaults;
use drive::drive::object_size_info::DocumentInfo::DocumentRefInfo;
#[cfg(feature = "full")]
use drive::drive::query::QuerySerializedDocumentsOutcome;

use drive::query::{WhereClause, WhereOperator};
use drive::tests::helpers::setup::setup_drive_with_initial_state_structure;

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersonWithOptionalValues {
    #[serde(rename = "$id")]
    id: Vec<u8>,
    #[serde(rename = "$ownerId")]
    owner_id: Vec<u8>,
    first_name: Option<String>,
    middle_name: Option<String>,
    last_name: Option<String>,
    age: u8,
}

#[cfg(feature = "full")]
impl PersonWithOptionalValues {
    fn random_people(count: u32, seed: u64) -> Vec<Self> {
        let first_names =
            common::text_file_strings("tests/supporting_files/contract/family/first-names.txt");
        let middle_names =
            common::text_file_strings("tests/supporting_files/contract/family/middle-names.txt");
        let last_names =
            common::text_file_strings("tests/supporting_files/contract/family/last-names.txt");
        let mut vec: Vec<PersonWithOptionalValues> = Vec::with_capacity(count as usize);

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for _ in 0..count {
            let value = rng.gen::<u8>();
            let person = PersonWithOptionalValues {
                id: Vec::from(rng.gen::<[u8; 32]>()),
                owner_id: Vec::from(rng.gen::<[u8; 32]>()),
                first_name: if value & 1 != 0 {
                    Some(first_names.choose(&mut rng).unwrap().clone())
                } else {
                    None
                },
                middle_name: if value & 2 != 0 {
                    Some(middle_names.choose(&mut rng).unwrap().clone())
                } else {
                    None
                },
                last_name: if value & 4 != 0 {
                    Some(last_names.choose(&mut rng).unwrap().clone())
                } else {
                    None
                },
                age: rng.gen_range(0..85),
            };
            vec.push(person);
        }
        vec
    }
}

#[cfg(feature = "full")]
/// Inserts the test "family" contract and adds `count` documents containing randomly named people to it.
pub fn setup_family_tests(count: u32, seed: u64) -> (Drive, Contract) {
    let drive_config = DriveConfig::default();

    let drive = setup_drive(Some(drive_config));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction))
        .expect("expected to create contracts tree successfully");

    // setup code
    let contract = common::setup_contract(
        &drive,
        "tests/supporting_files/contract/family/family-contract.json",
        None,
        Some(&db_transaction),
    );

    let people = Person::random_people(count, seed);
    for person in people {
        let value = serde_json::to_value(person).expect("serialized person");
        let document_cbor = cbor_serializer::serializable_value_to_cbor(
            &value,
            Some(drive::drive::defaults::PROTOCOL_VERSION),
        )
        .expect("expected to serialize to cbor");
        let document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("document should be properly deserialized");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::genesis(),
                true,
                Some(&db_transaction),
            )
            .expect("document should be inserted");
    }
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "full")]
/// Same as `setup_family_tests` but with null values in the documents.
pub fn setup_family_tests_with_nulls(count: u32, seed: u64) -> (Drive, Contract) {
    let drive_config = DriveConfig::default();

    let drive = setup_drive(Some(drive_config));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction))
        .expect("expected to create contracts tree successfully");

    // setup code
    let contract = common::setup_contract(
        &drive,
        "tests/supporting_files/contract/family/family-contract-fields-optional.json",
        None,
        Some(&db_transaction),
    );

    let people = PersonWithOptionalValues::random_people(count, seed);
    for person in people {
        let value = serde_json::to_value(person).expect("serialized person");
        let document_cbor = cbor_serializer::serializable_value_to_cbor(
            &value,
            Some(drive::drive::defaults::PROTOCOL_VERSION),
        )
        .expect("expected to serialize to cbor");
        let document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("document should be properly deserialized");
        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::genesis(),
                true,
                Some(&db_transaction),
            )
            .expect("document should be inserted");
    }
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "full")]
/// Inserts the test "family" contract and adds `count` documents containing randomly named people to it.
pub fn setup_family_tests_only_first_name_index(count: u32, seed: u64) -> (Drive, Contract) {
    let drive_config = DriveConfig::default();

    let drive = setup_drive(Some(drive_config));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction))
        .expect("expected to create contracts tree successfully");

    // setup code
    let contract = common::setup_contract(
        &drive,
        "tests/supporting_files/contract/family/family-contract-only-first-name-index.json",
        None,
        Some(&db_transaction),
    );

    let people = Person::random_people(count, seed);
    for person in people {
        let value = serde_json::to_value(person).expect("serialized person");
        let document_cbor = cbor_serializer::serializable_value_to_cbor(
            &value,
            Some(drive::drive::defaults::PROTOCOL_VERSION),
        )
        .expect("expected to serialize to cbor");
        let document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("document should be properly deserialized");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::genesis(),
                true,
                Some(&db_transaction),
            )
            .expect("document should be inserted");
    }
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "full")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Records {
    dash_unique_identity_id: Identifier,
}

#[cfg(feature = "full")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubdomainRules {
    allow_subdomains: bool,
}

#[cfg(feature = "full")]
/// DPNS domain info
// In the real dpns, label is required. We make it optional here for a test.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Domain {
    #[serde(rename = "$id")]
    id: Identifier,
    #[serde(rename = "$ownerId")]
    owner_id: Identifier,
    label: Option<String>,
    normalized_label: Option<String>,
    normalized_parent_domain_name: String,
    records: Records,
    preorder_salt: Bytes32,
    subdomain_rules: SubdomainRules,
}

#[cfg(feature = "full")]
#[test]
fn test_serialization_and_deserialization() {
    let domains = Domain::random_domains_in_parent(20, 100, "dash");
    let contract =
        json_document_to_contract("tests/supporting_files/contract/dpns/dpns-contract.json")
            .expect("expected to get cbor contract");

    let document_type = contract
        .document_type_for_name("domain")
        .expect("expected to get document type");
    for domain in domains {
        let value = platform_value::to_value(domain).expect("expected value");
        let mut document = Document::from_platform_value(value).expect("expected value");
        document.revision = Some(1);
        let serialized = document
            .serialize(document_type)
            .expect("expected to serialize domain document");
        let _deserialized = Document::from_bytes(&serialized, document_type)
            .expect("expected to deserialize domain document");
    }
}

#[cfg(feature = "full")]
#[test]
fn test_serialization_and_deserialization_with_null_values_should_fail_if_required() {
    let contract =
        json_document_to_contract("tests/supporting_files/contract/dpns/dpns-contract.json")
            .expect("expected to get cbor contract");

    let document_type = contract
        .document_type_for_name("domain")
        .expect("expected to get document type");

    let mut rng = rand::rngs::StdRng::seed_from_u64(5);

    let domain = Domain {
        id: Identifier::random_with_rng(&mut rng),
        owner_id: Identifier::random_with_rng(&mut rng),
        label: None,
        normalized_label: None,
        normalized_parent_domain_name: "dash".to_string(),
        records: Records {
            dash_unique_identity_id: Identifier::random_with_rng(&mut rng),
        },
        preorder_salt: Bytes32::random_with_rng(&mut rng),
        subdomain_rules: SubdomainRules {
            allow_subdomains: false,
        },
    };

    let value = platform_value::to_value(domain).expect("expected value");
    let mut document = Document::from_platform_value(value).expect("expected value");
    document.revision = Some(1);
    document
        .serialize(document_type)
        .expect_err("expected to not be able to serialize domain document");
}

#[cfg(feature = "full")]
#[test]
fn test_serialization_and_deserialization_with_null_values() {
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/dpns/dpns-contract-label-not-required.json",
    )
    .expect("expected to get cbor contract");

    let document_type = contract
        .document_type_for_name("domain")
        .expect("expected to get document type");

    let mut rng = rand::rngs::StdRng::seed_from_u64(5);

    let domain = Domain {
        id: Identifier::random_with_rng(&mut rng),
        owner_id: Identifier::random_with_rng(&mut rng),
        label: None,
        normalized_label: None,
        normalized_parent_domain_name: "dash".to_string(),
        records: Records {
            dash_unique_identity_id: Identifier::random_with_rng(&mut rng),
        },
        preorder_salt: Bytes32::random_with_rng(&mut rng),
        subdomain_rules: SubdomainRules {
            allow_subdomains: false,
        },
    };

    let mut value = platform_value::to_value(domain).expect("expected value");
    value
        .remove_optional_value("label")
        .expect("expected to remove null");
    value
        .remove_optional_value("normalizedLabel")
        .expect("expected to remove null");
    let mut document = Document::from_platform_value(value).expect("expected value");
    document.revision = Some(1);
    let serialized = document
        .serialize(document_type)
        .expect("expected to be able to serialize domain document");
    Document::from_bytes(&serialized, document_type)
        .expect("expected to deserialize domain document");
}

#[cfg(feature = "full")]
impl Domain {
    /// Creates `count` random names as domain names for the given parent domain
    fn random_domains_in_parent(
        count: u32,
        seed: u64,
        normalized_parent_domain_name: &str,
    ) -> Vec<Self> {
        let first_names =
            common::text_file_strings("tests/supporting_files/contract/family/first-names.txt");
        let mut vec: Vec<Domain> = Vec::with_capacity(count as usize);

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for _i in 0..count {
            let label = first_names.choose(&mut rng).unwrap();
            let domain = Domain {
                id: Identifier::random_with_rng(&mut rng),
                owner_id: Identifier::random_with_rng(&mut rng),
                label: Some(label.clone()),
                normalized_label: Some(label.to_lowercase()),
                normalized_parent_domain_name: normalized_parent_domain_name.to_string(),
                records: Records {
                    dash_unique_identity_id: Identifier::random_with_rng(&mut rng),
                },
                preorder_salt: Bytes32::random_with_rng(&mut rng),
                subdomain_rules: SubdomainRules {
                    allow_subdomains: false,
                },
            };
            vec.push(domain);
        }
        vec
    }
}

#[cfg(feature = "full")]
/// Adds `count` random domain names to the given contract
pub fn add_domains_to_contract(
    drive: &Drive,
    contract: &Contract,
    transaction: TransactionArg,
    count: u32,
    seed: u64,
) {
    let domains = Domain::random_domains_in_parent(count, seed, "dash");
    let document_type = contract
        .document_type_for_name("domain")
        .expect("expected to get document type");
    for domain in domains {
        let value = platform_value::to_value(domain).expect("expected value");
        let document = Document::from_platform_value(value).expect("expected value");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                true,
                BlockInfo::genesis(),
                true,
                transaction,
            )
            .expect("document should be inserted");
    }
}

#[cfg(feature = "full")]
/// Sets up and inserts random domain name data to the DPNS contract to test queries on.
pub fn setup_dpns_tests_with_batches(count: u32, seed: u64) -> (Drive, Contract) {
    let drive = setup_drive(Some(DriveConfig::default()));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction))
        .expect("expected to create contracts tree successfully");

    // setup code
    let contract = setup_contract(
        &drive,
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        None,
        Some(&db_transaction),
    );

    add_domains_to_contract(&drive, &contract, Some(&db_transaction), count, seed);
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "full")]
/// Sets up and inserts random domain name data to the DPNS contract to test queries on.
pub fn setup_dpns_tests_label_not_required(count: u32, seed: u64) -> (Drive, Contract) {
    let drive = setup_drive(Some(DriveConfig::default()));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction))
        .expect("expected to create contracts tree successfully");

    // setup code
    let contract = setup_contract(
        &drive,
        "tests/supporting_files/contract/dpns/dpns-contract-label-not-required.json",
        None,
        Some(&db_transaction),
    );

    add_domains_to_contract(&drive, &contract, Some(&db_transaction), count, seed);
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "full")]
/// Sets up the DPNS contract and inserts data from the given path to test queries on.
pub fn setup_dpns_test_with_data(path: &str) -> (Drive, Contract) {
    let drive = setup_drive(None);

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction))
        .expect("expected to create contracts tree successfully");

    let contract = setup_contract(
        &drive,
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        None,
        Some(&db_transaction),
    );

    let file = File::open(path).expect("should read domains from file");

    for domain_json in io::BufReader::new(file).lines().flatten() {
        let domain_json: serde_json::Value =
            serde_json::from_str(&domain_json).expect("should parse json");

        let domain_cbor = cbor_serializer::serializable_value_to_cbor(
            &domain_json,
            Some(drive::drive::defaults::PROTOCOL_VERSION),
        )
        .expect("expected to serialize to cbor");

        let domain = Document::from_cbor(&domain_cbor, None, None)
            .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected to get document type");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&domain, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::genesis(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");
    }
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "full")]
#[test]
#[ignore]
fn test_query_many() {
    let (drive, contract) = setup_family_tests(1600, 73509);
    let db_transaction = drive.grove.start_transaction();

    let people = Person::random_people(10, 73409);
    for person in people {
        let value = serde_json::to_value(person).expect("serialized person");
        let document_cbor = cbor_serializer::serializable_value_to_cbor(
            &value,
            Some(drive::drive::defaults::PROTOCOL_VERSION),
        )
        .expect("expected to serialize to cbor");
        let document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("document should be properly deserialized");
        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::genesis(),
                true,
                Some(&db_transaction),
            )
            .expect("document should be inserted");
    }
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");
}

#[cfg(feature = "full")]
#[test]
fn test_reference_proof_single_index() {
    let (drive, contract) = setup_family_tests_only_first_name_index(1, 73509);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    // A query getting all elements by firstName

    let query_value = json!({
        "where": [
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_non_existence_reference_proof_single_index() {
    let (drive, contract) = setup_family_tests_only_first_name_index(0, 73509);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    // A query getting all elements by firstName

    let query_value = json!({
        "where": [
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_family_basic_queries() {
    let (drive, contract) = setup_family_tests(10, 73509);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        181, 109, 8, 213, 170, 216, 114, 214, 14, 29, 33, 19, 214, 194, 147, 208, 69, 229, 255, 17,
        194, 142, 198, 188, 9, 141, 124, 102, 165, 16, 120, 237,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    // A query getting all people who's first name is Adey (which should exist)
    let query_value = json!({
        "where": [
            ["firstName", "==", "Adey"]
        ]
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");
    assert_eq!(results.len(), 1);

    let (proof_root_hash, proof_results, _) = drive
        .query_proof_of_documents_using_cbor_encoded_query_only_get_elements(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    // A query getting all people who's first name is Adey and lastName Randolf

    let query_value = json!({
        "where": [
            ["firstName", "==", "Adey"],
            ["lastName", "==", "Randolf"]
        ],
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let (proof_root_hash, proof_results, _) = drive
        .query_proof_of_documents_using_cbor_encoded_query_only_get_elements(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let document = Document::from_bytes(results.first().unwrap().as_slice(), person_document_type)
        .expect("we should be able to deserialize from bytes");
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

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let (proof_root_hash, proof_results, _) = drive
        .query_proof_of_documents_using_cbor_encoded_query_only_get_elements(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    // A query getting all people who's first name is Adey, order by lastName (which should exist)

    let query_value = json!({
        "where": [
            ["firstName", "==", "Adey"]
        ],
        "orderBy": [
            ["lastName", "asc"]
        ]
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let (proof_root_hash, proof_results, _) = drive
        .query_proof_of_documents_using_cbor_encoded_query_only_get_elements(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let document = Document::from_bytes(results.first().unwrap().as_slice(), person_document_type)
        .expect("we should be able to deserialize from bytes");
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

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 0);

    let (proof_root_hash, proof_results, _) = drive
        .query_proof_of_documents_using_cbor_encoded_query_only_get_elements(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    // A query getting a middle name

    let query_value = json!({
        "where": [
            ["middleName", "==", "Briggs"]
        ]
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let (proof_root_hash, proof_results, _) = drive
        .query_proof_of_documents_using_cbor_encoded_query_only_get_elements(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");
    assert_eq!(results.len(), 5);

    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let query_value = json!({
        "where": [
            ["firstName", "in", names]
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let ages: HashMap<String, u8> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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
            let age: u8 = age_value.to_integer().expect("expected u8 value");
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
    let serialized_person = serde_json::to_value(fixed_person).expect("serialized person");
    let person_cbor = cbor_serializer::serializable_value_to_cbor(
        &serialized_person,
        Some(drive::drive::defaults::PROTOCOL_VERSION),
    )
    .expect("expected to serialize to cbor");
    let document = Document::from_cbor(person_cbor.as_slice(), None, None)
        .expect("document should be properly deserialized");

    let document_type = contract
        .document_type_for_name("person")
        .expect("expected to get document type");

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, storage_flags)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::genesis(),
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
    let serialized_person = serde_json::to_value(next_person).expect("serialized person");
    let person_cbor = cbor_serializer::serializable_value_to_cbor(
        &serialized_person,
        Some(drive::drive::defaults::PROTOCOL_VERSION),
    )
    .expect("expected to serialize to cbor");
    let document = Document::from_cbor(person_cbor.as_slice(), None, None)
        .expect("document should be properly deserialized");

    let document_type = contract
        .document_type_for_name("person")
        .expect("expected to get document type");

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, storage_flags)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::genesis(),
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    let query_value = json!({
        "where": [
            ["$id", "in", vec![String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // TODO: Add test for proofs after transaction
    // drive.grove.commit_transaction(db_transaction).expect("unable to commit transaction");
    // let (proof_root_hash, proof_results) = drive
    //     .query_documents_from_contract_as_grove_proof_only_get_elements(
    //         &contract,
    //         person_document_type,
    //         query_cbor.as_slice(),
    //         None,
    //     )
    //     .expect("query should be executed");
    // assert_eq!(root_hash, proof_root_hash);
    // assert_eq!(results, proof_results);
    // let db_transaction = drive.grove.start_transaction();

    // fetching by $id with order by

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "orderBy": [["$id", "asc"]],
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 2);

    let last_person = Document::from_bytes(results.first().unwrap().as_slice(), document_type)
        .expect("we should be able to deserialize the document");

    assert_eq!(
        last_person.id.to_vec(),
        vec![
            76, 161, 17, 201, 152, 232, 129, 48, 168, 13, 49, 10, 218, 53, 118, 136, 165, 198, 189,
            116, 116, 22, 133, 92, 104, 165, 186, 249, 94, 81, 45, 20,
        ]
    );

    // fetching by $id with order by desc

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "orderBy": [["$id", "desc"]],
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 2);

    let last_person = Document::from_bytes(results.first().unwrap().as_slice(), document_type)
        .expect("we should be able to deserialize the document");

    assert_eq!(
        last_person.id.to_vec(),
        vec![
            140, 161, 17, 201, 152, 232, 129, 48, 168, 13, 49, 10, 218, 53, 118, 136, 165, 198,
            189, 116, 116, 22, 133, 92, 104, 165, 186, 249, 94, 81, 45, 20,
        ]
    );

    //
    // // fetching with empty where and orderBy
    //
    let query_value = json!({});

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
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

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 12);

    let last_person = Document::from_bytes(results.first().unwrap().as_slice(), document_type)
        .expect("we should be able to deserialize the document");

    assert_eq!(
        last_person.id.to_vec(),
        vec![
            249, 170, 70, 122, 181, 31, 35, 176, 175, 131, 70, 150, 250, 223, 194, 203, 175, 200,
            107, 252, 199, 227, 154, 105, 89, 57, 38, 85, 236, 192, 254, 88,
        ]
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

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
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

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // query empty contract with nested path queries

    let contract_cbor = hex::decode("01a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

    drive
        .apply_contract_cbor(
            contract_cbor.clone(),
            None,
            BlockInfo::genesis(),
            true,
            StorageFlags::optional_default_as_cow(),
            Some(&db_transaction),
        )
        .expect("expected to apply contract successfully");

    let query_value = json!({
        "where": [
            ["$ownerId", "==", "BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj"],
            ["toUserId", "==", "BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj"],
        ],
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let (results, _, _) = drive
        .query_raw_documents_from_contract_cbor_using_cbor_encoded_query_with_cost(
            query_cbor.as_slice(),
            contract_cbor.as_slice(),
            String::from("contact"),
            None,
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

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
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

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let result = drive.query_documents_cbor_from_contract(
        &contract,
        person_document_type,
        query_cbor.as_slice(),
        None,
        Some(&db_transaction),
    );

    assert!(
        matches!(result, Err(Error::Query(QueryError::StartDocumentNotFound(message))) if message == "startAt document not found")
    );

    // using non existing document in startAfter

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "startAfter": String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF178"),
        "orderBy": [["$id", "asc"]],
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    let result = drive.query_documents_cbor_from_contract(
        &contract,
        person_document_type,
        query_cbor.as_slice(),
        None,
        Some(&db_transaction),
    );

    assert!(
        matches!(result, Err(Error::Query(QueryError::StartDocumentNotFound(message))) if message == "startAfter document not found")
    );

    // validate eventual root hash

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    assert_eq!(
        root_hash.as_slice(),
        vec![
            122, 226, 108, 85, 10, 26, 53, 19, 226, 252, 13, 68, 155, 226, 231, 159, 52, 180, 74,
            65, 95, 241, 28, 9, 64, 194, 200, 104, 34, 44, 172, 217,
        ],
    );
}

#[cfg(feature = "full")]
#[test]
fn test_family_starts_at_queries() {
    let (drive, contract) = setup_family_tests(10, 73509);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        181, 109, 8, 213, 170, 216, 114, 214, 14, 29, 33, 19, 214, 194, 147, 208, 69, 229, 255, 17,
        194, 142, 198, 188, 9, 141, 124, 102, 165, 16, 120, 237,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

    // let all_names = [
    //     "Adey".to_string(),
    //     "Briney".to_string(),
    //     "Cammi".to_string(),
    //     "Celinda".to_string(),
    //     "Dalia".to_string(),
    //     "Gilligan".to_string(),
    //     "Kevina".to_string(),
    //     "Meta".to_string(),
    //     "Noellyn".to_string(),
    //     "Prissie".to_string(),
    // ];

    let kevina_encoded_id = "B4zLoYmSGz5SyD7QjAvcjAWtzGCfnQDCti3o7V2ZBDNo".to_string();

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");

    let reduced_names_after: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");

    let reduced_names_after: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let query_value = json!({
        "where": [
            ["firstName", ">", "Chris"],
            ["firstName", "<=", "Noellyn"]
        ],
        "startAt": kevina_encoded_id, //Kevina
        "limit": 100,
        "orderBy": [
            ["firstName", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");

    let reduced_names_after: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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
        "Gilligan".to_string(),
        "Dalia".to_string(),
    ];

    assert_eq!(reduced_names_after, expected_reduced_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    // Now lets try startsAfter

    let query_value = json!({
        "where": [
            ["firstName", ">", "Chris"],
            ["firstName", "<=", "Noellyn"]
        ],
        "startAfter": kevina_encoded_id, //Kevina
        "limit": 100,
        "orderBy": [
            ["firstName", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None)
        .expect("proof should be executed");
    assert_eq!(results.len(), 2);

    let reduced_names_after: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
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

    let expected_reduced_names = ["Gilligan".to_string(), "Dalia".to_string()];

    assert_eq!(reduced_names_after, expected_reduced_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_family_sql_query() {
    // These helpers confirm that sql statements produce the same drive query
    // as their json counterparts, helpers above confirm that the json queries
    // produce the correct result set
    let (_, contract) = setup_family_tests(10, 73509);
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");

    // Empty where clause
    let query_cbor = cbor_serializer::serializable_value_to_cbor(
        &json!({
            "where": [],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"]
            ]
        }),
        None,
    )
    .expect("expected to serialize to cbor");
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, person_document_type)
        .expect("should build query");

    let sql_string = "select * from person order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);

    // Equality clause
    let query_cbor = cbor_serializer::serializable_value_to_cbor(
        &json!({
            "where": [
                ["firstName", "==", "Chris"]
            ]
        }),
        None,
    )
    .expect("expected to serialize to cbor");
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, person_document_type)
        .expect("should build query");

    let sql_string = "select * from person where firstName = 'Chris'";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);

    // Less than
    let query_cbor = cbor_serializer::serializable_value_to_cbor(
        &json!({
            "where": [
                ["firstName", "<", "Chris"]
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"]
            ]
        }),
        None,
    )
    .expect("expected to serialize to cbor");
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, person_document_type)
        .expect("should build query");

    let sql_string =
        "select * from person where firstName < 'Chris' order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);

    // Starts with
    let query_cbor = cbor_serializer::serializable_value_to_cbor(
        &json!({
            "where": [
                ["firstName", "StartsWith", "C"]
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"]
            ]
        }),
        None,
    )
    .expect("expected to serialize to cbor");
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, person_document_type)
        .expect("should build query");

    let sql_string =
        "select * from person where firstName like 'C%' order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);

    // Range combination
    let query_cbor = cbor_serializer::serializable_value_to_cbor(
        &json!({
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
    )
    .expect("expected to serialize to cbor");
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, person_document_type)
        .expect("should build query");

    let sql_string = "select * from person where firstName > 'Chris' and firstName <= 'Noellyn' order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);

    // In clause
    let names = vec![String::from("a"), String::from("b")];
    let query_cbor = cbor_serializer::serializable_value_to_cbor(
        &json!({
            "where": [
                ["firstName", "in", names]
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"]
            ],
        }),
        None,
    )
    .expect("expected to serialize to cbor");
    let query1 = DriveQuery::from_cbor(query_cbor.as_slice(), &contract, person_document_type)
        .expect("should build query");

    let sql_string =
        "select * from person where firstName in ('a', 'b') order by firstName limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

    assert_eq!(query1, query2);
}

#[cfg(feature = "full")]
#[test]
fn test_family_with_nulls_query() {
    let (drive, contract) = setup_family_tests_with_nulls(10, 30004);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        35, 178, 180, 34, 225, 111, 160, 55, 202, 140, 247, 123, 80, 10, 164, 156, 176, 97, 11,
        225, 137, 192, 40, 254, 180, 159, 110, 122, 148, 85, 21, 195,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

    let all_names = [
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "Alexia".to_string(),
        "Gerti".to_string(),
        "Latisha".to_string(),
        "Norry".to_string(),
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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_types
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .clone()
        .into_iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            if first_name_value.is_null() {
                String::from("")
            } else {
                let first_name = first_name_value
                    .as_text()
                    .expect("the first name should be a string");
                String::from(first_name)
            }
        })
        .collect();

    assert_eq!(names, all_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let ids: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), person_document_type)
                .expect("we should be able to deserialize the document");
            base64::encode(document.id.as_slice())
        })
        .collect();

    for i in 0..10 {
        drive
            .delete_document_for_contract(
                base64::decode(ids.get(i).unwrap())
                    .expect("expected to decode from base64")
                    .try_into()
                    .expect("expected to get 32 bytes"),
                &contract,
                "person",
                None,
                BlockInfo::genesis(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");
    }

    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("unable to commit transaction");
}

#[cfg(feature = "full")]
#[test]
fn test_query_with_cached_contract() {
    let (drive, contract) = setup_family_tests(10, 73509);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    // Make sure the state is deterministic
    let expected_app_hash = vec![
        181, 109, 8, 213, 170, 216, 114, 214, 14, 29, 33, 19, 214, 194, 147, 208, 69, 229, 255, 17,
        194, 142, 198, 188, 9, 141, 124, 102, 165, 16, 120, 237,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

    // Make sure contract is not cached
    let contract_ref =
        drive.get_cached_contract_with_fetch_info(*contract.id.as_bytes(), Some(&db_transaction));

    assert!(contract_ref.is_none());

    // A query getting all elements by firstName

    let query_value = json!({
        "where": [
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let QuerySerializedDocumentsOutcome { items, .. } = drive
        .query_documents_cbor_with_document_type_lookup(
            where_cbor.as_slice(),
            *contract.id.as_bytes(),
            "person",
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(items.len(), 10);

    // Cache was populated and there only two ref two the cached fetched info (here and cache)
    let contract_ref = drive
        .get_cached_contract_with_fetch_info(*contract.id.as_bytes(), Some(&db_transaction))
        .expect("expected a reference counter to the contract");

    assert_eq!(Arc::strong_count(&contract_ref), 2);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query() {
    let (drive, contract) = setup_dpns_tests_with_batches(10, 11456);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        54, 250, 97, 187, 157, 71, 4, 189, 235, 29, 185, 87, 218, 5, 29, 24, 104, 209, 223, 84, 79,
        98, 29, 243, 225, 220, 244, 162, 174, 131, 72, 224,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let ids: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
            hex::encode(document.id.as_slice())
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
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    // A query getting one element starting with a in dash parent domain desc

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
            ["normalizedLabel", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let record_id_base68: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");

            let records_value = document
                .properties
                .get("records")
                .expect("we should be able to get the records");
            let map_records_value = records_value.as_map().expect("this should be a map");
            let record_dash_unique_identity_id =
                Value::inner_optional_bytes_value(map_records_value, "dashUniqueIdentityId")
                    .unwrap()
                    .expect("there should be a dashUniqueIdentityId");
            bs58::encode(record_dash_unique_identity_id).into_string()
        })
        .collect();

    let a_record_id_base58 = ["5hXRj1xmmnNQ7RN1ATYym4x6bQugxcKn7FWiMnkQTQpF".to_string()];

    assert_eq!(record_id_base68, a_record_id_base58);

    // A query getting elements by the dashUniqueIdentityId desc

    let query_value = json!({
        "where": [
            ["records.dashUniqueIdentityId", "<=", "5hXRj1xmmnNQ7RN1ATYym4x6bQugxcKn7FWiMnkQTQpF"],
        ],
        "limit": 10,
        "orderBy": [
            ["records.dashUniqueIdentityId", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    // A query getting 2 elements asc by the dashUniqueIdentityId

    let query_value = json!({
        "where": [
            ["records.dashUniqueIdentityId", "<=", "5hXRj1xmmnNQ7RN1ATYym4x6bQugxcKn7FWiMnkQTQpF"],
        ],
        "limit": 2,
        "orderBy": [
            ["records.dashUniqueIdentityId", "asc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    // A query getting all elements

    let query_value = json!({
        "orderBy": [
            ["records.dashUniqueIdentityId", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");

    assert_eq!(results.len(), 10);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_insertion_no_aliases() {
    // using ascending order with rangeTo operators
    let (drive, contract) =
        setup_dpns_test_with_data("tests/supporting_files/contract/dpns/domains-no-alias.json");

    let db_transaction = drive.grove.start_transaction();

    let query_value = json!({
        "orderBy": [["records.dashUniqueIdentityId", "desc"]],
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");

    let result = drive
        .query_documents_cbor_from_contract(
            &contract,
            domain_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("should perform query");

    assert_eq!(result.0.len(), 15);

    let (proof_root_hash, proof_results, _) = drive
        .query_proof_of_documents_using_cbor_encoded_query_only_get_elements(
            &contract,
            domain_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");
    assert_eq!(
        drive
            .grove
            .root_hash(None)
            .unwrap()
            .expect("should get root hash"),
        proof_root_hash
    );
    assert_eq!(result.0, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_insertion_with_aliases() {
    // using ascending order with rangeTo operators
    let (drive, contract) =
        setup_dpns_test_with_data("tests/supporting_files/contract/dpns/domains.json");

    let db_transaction = drive.grove.start_transaction();

    let query_value = json!({
        "orderBy": [["records.dashUniqueIdentityId", "desc"]],
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");

    let result = drive
        .query_documents_cbor_from_contract(
            &contract,
            domain_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("should perform query");

    assert_eq!(result.0.len(), 24);

    let (proof_root_hash, proof_results, _) = drive
        .query_proof_of_documents_using_cbor_encoded_query_only_get_elements(
            &contract,
            domain_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");
    assert_eq!(
        drive
            .grove
            .root_hash(None)
            .unwrap()
            .expect("should get root hash"),
        proof_root_hash
    );
    assert_eq!(result.0, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_start_at() {
    // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
    let (drive, contract) = setup_dpns_tests_with_batches(10, 11456);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        54, 250, 97, 187, 157, 71, 4, 189, 235, 29, 185, 87, 218, 5, 29, 24, 104, 209, 223, 84, 79,
        98, 29, 243, 225, 220, 244, 162, 174, 131, 72, 224,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash,);

    // let all_names = [
    //     "amalle".to_string(),
    //     "anna-diane".to_string(),
    //     "atalanta".to_string(),
    //     "eden".to_string(),
    //     "laureen".to_string(),
    //     "leone".to_string(),
    //     "marilyn".to_string(),
    //     "minna".to_string(),
    //     "mora".to_string(),
    //     "phillie".to_string(),
    // ];

    // A query getting one element starting with a in dash parent domain asc

    let anna_id = hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
        .expect("expected to decode id");
    let encoded_start_at = bs58::encode(anna_id).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"]
        ],
        "startAt":  encoded_start_at,
        "limit": 1,
        "orderBy": [
            ["normalizedLabel", "asc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
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

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_start_after() {
    // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
    let (drive, contract) = setup_dpns_tests_with_batches(10, 11456);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        54, 250, 97, 187, 157, 71, 4, 189, 235, 29, 185, 87, 218, 5, 29, 24, 104, 209, 223, 84, 79,
        98, 29, 243, 225, 220, 244, 162, 174, 131, 72, 224,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

    // let all_names = [
    //     "amalle".to_string(),
    //     "anna-diane".to_string(),
    //     "atalanta".to_string(),
    //     "eden".to_string(),
    //     "laureen".to_string(),
    //     "leone".to_string(),
    //     "marilyn".to_string(),
    //     "minna".to_string(),
    //     "mora".to_string(),
    //     "phillie".to_string(),
    // ];

    // A query getting one element starting with a in dash parent domain asc

    let anna_id = hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
        .expect("expected to decode id");
    let encoded_start_at = bs58::encode(anna_id).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"]
        ],
        "startAfter":  encoded_start_at,
        "limit": 2,
        "orderBy": [
            ["normalizedLabel", "asc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
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

    let a_names = ["atalanta".to_string(), "eden".to_string()];

    assert_eq!(names, a_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_start_at_desc() {
    // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
    let (drive, contract) = setup_dpns_tests_with_batches(10, 11456);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        54, 250, 97, 187, 157, 71, 4, 189, 235, 29, 185, 87, 218, 5, 29, 24, 104, 209, 223, 84, 79,
        98, 29, 243, 225, 220, 244, 162, 174, 131, 72, 224,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

    // let all_names = [
    //     "amalle".to_string(),
    //     "anna-diane".to_string(),
    //     "atalanta".to_string(),
    //     "eden".to_string(),
    //     "laureen".to_string(),
    //     "leone".to_string(),
    //     "marilyn".to_string(),
    //     "minna".to_string(),
    //     "mora".to_string(),
    //     "phillie".to_string(),
    // ];

    // A query getting one element starting with a in dash parent domain asc

    let anna_id = hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
        .expect("expected to decode id");
    let encoded_start_at = bs58::encode(anna_id).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"]
        ],
        "startAt": encoded_start_at,
        "limit": 2,
        "orderBy": [
            ["normalizedLabel", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
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

    let a_names = ["anna-diane".to_string(), "amalle".to_string()];

    assert_eq!(names, a_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_start_after_desc() {
    // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
    let (drive, contract) = setup_dpns_tests_with_batches(10, 11456);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        54, 250, 97, 187, 157, 71, 4, 189, 235, 29, 185, 87, 218, 5, 29, 24, 104, 209, 223, 84, 79,
        98, 29, 243, 225, 220, 244, 162, 174, 131, 72, 224,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

    // let all_names = [
    //     "amalle".to_string(),
    //     "anna-diane".to_string(),
    //     "atalanta".to_string(),
    //     "eden".to_string(),
    //     "laureen".to_string(),
    //     "leone".to_string(),
    //     "marilyn".to_string(),
    //     "minna".to_string(),
    //     "mora".to_string(),
    //     "phillie".to_string(),
    // ];

    // A query getting one element starting with a in dash parent domain asc

    let anna_id = hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
        .expect("expected to decode id");
    let encoded_start_at = bs58::encode(anna_id).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"]
        ],
        "startAfter": encoded_start_at,
        "limit": 2,
        "orderBy": [
            ["normalizedLabel", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
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

    let a_names = ["amalle".to_string()];

    assert_eq!(names, a_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_start_at_with_null_id() {
    // The point of this test is to test the situation where we have a start at inside an index with a null value
    // While dpns doesn't really support this, other contracts might allow null values.
    // We are just using the DPNS contract because it is handy.
    let (drive, contract) = setup_dpns_tests_label_not_required(10, 11456);

    let document_type = contract
        .document_type_for_name("domain")
        .expect("expected to get document type");

    let db_transaction = drive.grove.start_transaction();

    let mut rng = rand::rngs::StdRng::seed_from_u64(11456);

    let domain0_id = Identifier::random_with_rng(&mut rng);
    let domain0 = Domain {
        id: domain0_id,
        owner_id: Identifier::random_with_rng(&mut rng),
        label: None,
        normalized_label: None,
        normalized_parent_domain_name: "dash".to_string(),
        records: Records {
            dash_unique_identity_id: Identifier::random_with_rng(&mut rng),
        },
        preorder_salt: Bytes32::random_with_rng(&mut rng),
        subdomain_rules: SubdomainRules {
            allow_subdomains: false,
        },
    };

    let value0 = platform_value::to_value(domain0).expect("serialized domain");
    let document0 =
        platform_value::from_value(value0).expect("document should be properly deserialized");

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document0, storage_flags)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::genesis(),
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    let domain1_id = Identifier::random_with_rng(&mut rng);

    let domain1 = Domain {
        id: domain1_id,
        owner_id: Identifier::random_with_rng(&mut rng),
        label: None,
        normalized_label: None,
        normalized_parent_domain_name: "dash".to_string(),
        records: Records {
            dash_unique_identity_id: Identifier::random_with_rng(&mut rng),
        },
        preorder_salt: Bytes32::random_with_rng(&mut rng),
        subdomain_rules: SubdomainRules {
            allow_subdomains: false,
        },
    };

    let value1 = serde_json::to_value(domain1).expect("serialized domain");
    let document_cbor1 = cbor_serializer::serializable_value_to_cbor(
        &value1,
        Some(drive::drive::defaults::PROTOCOL_VERSION),
    )
    .expect("expected to serialize to cbor");
    let document1 = Document::from_cbor(document_cbor1.as_slice(), None, None)
        .expect("document should be properly deserialized");

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document1, storage_flags)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::genesis(),
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        190, 40, 187, 74, 248, 47, 25, 228, 232, 188, 239, 54, 120, 139, 47, 200, 120, 105, 80, 8,
        179, 214, 201, 248, 166, 6, 254, 182, 96, 41, 55, 127,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

    // let all_names = [
    //     "".to_string(), x2
    //     "amalle".to_string(),
    //     "anna-diane".to_string(),
    //     "atalanta".to_string(),
    //     "eden".to_string(),
    //     "laureen".to_string(),
    //     "leone".to_string(),
    //     "marilyn".to_string(),
    //     "minna".to_string(),
    //     "mora".to_string(),
    //     "phillie".to_string(),
    // ];

    // A query getting one element starting with a in dash parent domain asc

    let encoded_start_at = bs58::encode(domain0_id).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"]
        ],
        "startAt":  encoded_start_at,
        "limit": 3,
        "orderBy": [
            ["normalizedLabel", "asc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");

    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
            let normalized_label_value = document
                .properties
                .get("normalizedLabel")
                .expect("we should be able to get the first name");
            if normalized_label_value.is_null() {
                String::from("")
            } else {
                let normalized_label = normalized_label_value
                    .as_text()
                    .expect("the normalized label should be a string");
                String::from(normalized_label)
            }
        })
        .collect();

    let a_names = [
        "".to_string(),
        "amalle".to_string(),
        "anna-diane".to_string(),
    ];

    assert_eq!(names, a_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_start_after_with_null_id() {
    // The point of this test is to test the situation where we have a start at inside an index with a null value
    // While dpns doesn't really support this, other contracts might allow null values.
    // We are just using the DPNS contract because it is handy.
    let (drive, contract) = setup_dpns_tests_label_not_required(10, 11456);

    let document_type = contract
        .document_type_for_name("domain")
        .expect("expected to get document type");

    let db_transaction = drive.grove.start_transaction();

    let mut rng = rand::rngs::StdRng::seed_from_u64(11456);

    let domain0_id = Identifier::random_with_rng(&mut rng);
    let domain0 = Domain {
        id: domain0_id,
        owner_id: Identifier::random_with_rng(&mut rng),
        label: None,
        normalized_label: None,
        normalized_parent_domain_name: "dash".to_string(),
        records: Records {
            dash_unique_identity_id: Identifier::random_with_rng(&mut rng),
        },
        preorder_salt: Bytes32::random_with_rng(&mut rng),
        subdomain_rules: SubdomainRules {
            allow_subdomains: false,
        },
    };

    let value0 = serde_json::to_value(domain0).expect("serialized domain");
    let document_cbor0 = cbor_serializer::serializable_value_to_cbor(
        &value0,
        Some(drive::drive::defaults::PROTOCOL_VERSION),
    )
    .expect("expected to serialize to cbor");
    let document0 = Document::from_cbor(document_cbor0.as_slice(), None, None)
        .expect("document should be properly deserialized");

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document0, storage_flags)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::genesis(),
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    let domain1_id = Identifier::random_with_rng(&mut rng);

    let domain1 = Domain {
        id: domain1_id,
        owner_id: Identifier::random_with_rng(&mut rng),
        label: None,
        normalized_label: None,
        normalized_parent_domain_name: "dash".to_string(),
        records: Records {
            dash_unique_identity_id: Identifier::random_with_rng(&mut rng),
        },
        preorder_salt: Bytes32::random_with_rng(&mut rng),
        subdomain_rules: SubdomainRules {
            allow_subdomains: false,
        },
    };

    let value1 = serde_json::to_value(domain1).expect("serialized domain");
    let document_cbor1 = cbor_serializer::serializable_value_to_cbor(
        &value1,
        Some(drive::drive::defaults::PROTOCOL_VERSION),
    )
    .expect("expected to serialize to cbor");
    let document1 = Document::from_cbor(document_cbor1.as_slice(), None, None)
        .expect("document should be properly deserialized");

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document1, storage_flags)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::genesis(),
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        190, 40, 187, 74, 248, 47, 25, 228, 232, 188, 239, 54, 120, 139, 47, 200, 120, 105, 80, 8,
        179, 214, 201, 248, 166, 6, 254, 182, 96, 41, 55, 127,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

    // let all_names = [
    //     "".to_string(), x2
    //     "amalle".to_string(),
    //     "anna-diane".to_string(),
    //     "atalanta".to_string(),
    //     "eden".to_string(),
    //     "laureen".to_string(),
    //     "leone".to_string(),
    //     "marilyn".to_string(),
    //     "minna".to_string(),
    //     "mora".to_string(),
    //     "phillie".to_string(),
    // ];

    // A query getting one element starting with a in dash parent domain asc

    let encoded_start_at = bs58::encode(domain0_id).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"]
        ],
        "startAfter":  encoded_start_at,
        "limit": 2,
        "orderBy": [
            ["normalizedLabel", "asc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");

    // We are commenting this out on purpose to make it easier to find
    // let mut query_operations: Vec<QueryOperation> = vec![];
    // let path_query = query
    //     .construct_path_query_operations(&drive, Some(&db_transaction), &mut query_operations)
    //     .expect("expected to construct a path query");
    // println!("{:#?}", path_query);
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
            let normalized_label_value = document
                .properties
                .get("normalizedLabel")
                .expect("we should be able to get the first name");
            if normalized_label_value.is_null() {
                String::from("")
            } else {
                let normalized_label = normalized_label_value
                    .as_text()
                    .expect("the normalized label should be a string");
                String::from(normalized_label)
            }
        })
        .collect();

    let a_names = ["amalle".to_string(), "anna-diane".to_string()];

    assert_eq!(names, a_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_start_after_with_null_id_desc() {
    // The point of this test is to test the situation where we have a start at inside an index with a null value
    // While dpns doesn't really support this, other contracts might allow null values.
    // We are just using the DPNS contract because it is handy.
    let (drive, contract) = setup_dpns_tests_label_not_required(10, 11456);

    let document_type = contract
        .document_type_for_name("domain")
        .expect("expected to get document type");

    let db_transaction = drive.grove.start_transaction();

    let mut rng = rand::rngs::StdRng::seed_from_u64(11456);

    let domain0_id = Identifier::random_with_rng(&mut rng);
    let domain0 = Domain {
        id: domain0_id,
        owner_id: Identifier::random_with_rng(&mut rng),
        label: None,
        normalized_label: None,
        normalized_parent_domain_name: "dash".to_string(),
        records: Records {
            dash_unique_identity_id: Identifier::random_with_rng(&mut rng),
        },
        preorder_salt: Bytes32::random_with_rng(&mut rng),
        subdomain_rules: SubdomainRules {
            allow_subdomains: false,
        },
    };

    let value0 = serde_json::to_value(domain0).expect("serialized domain");
    let document_cbor0 = cbor_serializer::serializable_value_to_cbor(
        &value0,
        Some(drive::drive::defaults::PROTOCOL_VERSION),
    )
    .expect("expected to serialize to cbor");
    let document0 = Document::from_cbor(document_cbor0.as_slice(), None, None)
        .expect("document should be properly deserialized");

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document0, storage_flags)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::genesis(),
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    let domain1_id = Identifier::random_with_rng(&mut rng);

    let domain1 = Domain {
        id: domain1_id,
        owner_id: Identifier::random_with_rng(&mut rng),
        label: None,
        normalized_label: None,
        normalized_parent_domain_name: "dash".to_string(),
        records: Records {
            dash_unique_identity_id: Identifier::random_with_rng(&mut rng),
        },
        preorder_salt: Bytes32::random_with_rng(&mut rng),
        subdomain_rules: SubdomainRules {
            allow_subdomains: false,
        },
    };

    let value1 = serde_json::to_value(domain1).expect("serialized domain");
    let document_cbor1 = cbor_serializer::serializable_value_to_cbor(
        &value1,
        Some(drive::drive::defaults::PROTOCOL_VERSION),
    )
    .expect("expected to serialize to cbor");
    let document1 = Document::from_cbor(document_cbor1.as_slice(), None, None)
        .expect("document should be properly deserialized");

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document1, storage_flags)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::genesis(),
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        190, 40, 187, 74, 248, 47, 25, 228, 232, 188, 239, 54, 120, 139, 47, 200, 120, 105, 80, 8,
        179, 214, 201, 248, 166, 6, 254, 182, 96, 41, 55, 127,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash,);

    // let all_names = [
    //     "".to_string(), x2
    //     "amalle".to_string(),
    //     "anna-diane".to_string(),
    //     "atalanta".to_string(),
    //     "eden".to_string(),
    //     "laureen".to_string(),
    //     "leone".to_string(),
    //     "marilyn".to_string(),
    //     "minna".to_string(),
    //     "mora".to_string(),
    //     "phillie".to_string(),
    // ];

    assert_eq!(
        hex::encode(domain0_id.as_slice()),
        "8795eaa85e6f39a0d99ac8642a39e273204c57b1594dcd4f53f549fb5160fa32"
    );
    assert_eq!(
        hex::encode(domain1_id.as_slice()),
        "0baa338e26a9344b6d0fcf8f525dfc160c160a7a52ef3301a7e55fccf41d7385"
    );

    // A query getting two elements starting with domain0
    // We should get domain0 only because we have an ascending order on the ids always
    // And also because there is nothing below ""
    let encoded_start_at = bs58::encode(domain0_id).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"]
        ],
        "startAt":  encoded_start_at,
        "limit": 2,
        "orderBy": [
            ["normalizedLabel", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let docs: Vec<Vec<u8>> = results
        .clone()
        .into_iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
            document.id.to_vec()
        })
        .collect();

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    // The explanation is a little interesting
    // domain1 is smaller than domain0
    // however on the lowest lever the order never matters, so we are always ascending on the id
    // hence we will get domain1
    let expected_docs = [domain0_id.to_vec()];

    assert_eq!(docs, expected_docs);

    // A query getting two elements starting with domain1
    // We should get domain1, domain0 only because we have an ascending order on the ids always
    let encoded_start_at = bs58::encode(domain1_id).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"]
        ],
        "startAt":  encoded_start_at,
        "limit": 2,
        "orderBy": [
            ["normalizedLabel", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let docs: Vec<Vec<u8>> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
            document.id.to_vec()
        })
        .collect();

    // The explanation is a little interesting
    // domain1 is smaller than domain0
    // however on the lowest lever the order never matters, so we are always ascending on the id
    // hence we will get domain1
    let expected_docs = [domain1_id.to_vec(), domain0_id.to_vec()];

    assert_eq!(docs, expected_docs);
    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    // A query getting one element starting with a in dash parent domain asc

    let anna_id = hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
        .expect("expected to decode id");
    let encoded_start_at = bs58::encode(anna_id).into_string();

    let query_value = json!({
        "where": [
            ["normalizedParentDomainName", "==", "dash"]
        ],
        "startAfter":  encoded_start_at,
        "limit": 2,
        "orderBy": [
            ["normalizedLabel", "desc"]
        ]
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let domain_document_type = contract
        .document_types
        .get("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, domain_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_bytes(result.as_slice(), domain_document_type)
                .expect("we should be able to deserialize the document");
            let normalized_label_value = document
                .properties
                .get("normalizedLabel")
                .expect("we should be able to get the first name");
            if normalized_label_value.is_null() {
                String::from("")
            } else {
                let normalized_label = normalized_label_value
                    .as_text()
                    .expect("the normalized label should be a string");
                String::from(normalized_label)
            }
        })
        .collect();

    let a_names = ["amalle".to_string(), "".to_string()];

    assert_eq!(names, a_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_query_a_b_c_d_e_contract() {
    let tmp_dir = TempDir::new().unwrap();

    let drive: Drive = Drive::open(&tmp_dir, None).expect("expected to open Drive successfully");

    drive
        .create_initial_state_structure(None)
        .expect("expected to create root tree successfully");

    // Create a contract

    let block_info = BlockInfo::default();
    let owner_id = dpp::identifier::Identifier::new([2u8; 32]);

    let documents = platform_value!({
      "testDocument": {
        "type": "object",
        "properties": {
          "a": {
            "type": "integer"
          },
          "b": {
            "type": "integer"
          },
          "c": {
            "type": "integer"
          },
          "d": {
            "type": "integer"
          },
          "e": {
            "type": "integer"
          }
        },
        "additionalProperties": false,
        "indices": [
          {
            "name": "abcde",
            "properties": [
              {
                "a": "asc"
              },
              {
                "b": "asc"
              },
              {
                "c": "asc"
              },
              {
                "d": "asc"
              },
              {
                "e": "asc"
              }
            ]
          },
        ]
      }
    });

    let protocol_version_validator =
        ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone());

    let data_contract_validator = DataContractValidator::new(Arc::new(protocol_version_validator));
    let factory = DataContractFactory::new(1, Arc::new(data_contract_validator));

    let contract = factory
        .create(owner_id, documents, None, None)
        .expect("data in fixture should be correct");

    let contract_cbor = contract.to_cbor().expect("should encode contract to cbor");

    // TODO: Create method doesn't initiate document_types. It must be fixed
    let contract = DataContract::from_cbor(contract_cbor.clone())
        .expect("should create decode contract from cbor");

    drive
        .apply_contract_with_serialization(
            &contract,
            contract_cbor,
            block_info,
            true,
            StorageFlags::optional_default_as_cow(),
            None,
        )
        .expect("should apply contract");

    // Perform query

    let document_type = "testDocument".to_string();

    let query_json = json!({
        "where": [
            ["a","==",1],
            ["b","==",2],
            ["c","==",3],
            ["d","in",[1,2]]],
        "orderBy":[
            ["d","desc"],
            ["e","asc"]
        ]
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_json, None)
        .expect("expected to serialize to cbor");

    drive
        .query_documents_cbor_from_contract(
            &contract,
            contract
                .document_types
                .get(&document_type)
                .expect("should have this document type"),
            &query_cbor,
            None,
            None,
        )
        .expect("should perform query");
}

#[cfg(feature = "full")]
#[test]
fn test_query_documents_by_created_at() {
    let drive = setup_drive_with_initial_state_structure();

    let contract = platform_value!({
        "protocolVersion": 1,
        "$id": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
        "$schema": "https://schema.dash.org/dpp-0-4-0/meta/data-contract",
        "version": 1,
        "ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
        "documents": {
            "indexedDocument": {
                "type": "object",
                "indices": [
                    {"name":"index1", "properties": [{"$ownerId":"asc"}, {"firstName":"desc"}], "unique":true},
                    {"name":"index2", "properties": [{"$ownerId":"asc"}, {"lastName":"desc"}], "unique":true},
                    {"name":"index3", "properties": [{"lastName":"asc"}]},
                    {"name":"index4", "properties": [{"$createdAt":"asc"}, {"$updatedAt":"asc"}]},
                    {"name":"index5", "properties": [{"$updatedAt":"asc"}]},
                    {"name":"index6", "properties": [{"$createdAt":"asc"}]}
                ],
                "properties":{
                    "firstName": {
                        "type": "string",
                        "maxLength": 63,
                    },
                    "lastName": {
                        "type": "string",
                        "maxLength": 63,
                    }
                },
                "required": ["firstName", "$createdAt", "$updatedAt", "lastName"],
                "additionalProperties": false,
            },
        },
    });

    let contract_cbor =
        cbor_serializer::serializable_value_to_cbor(&contract, Some(defaults::PROTOCOL_VERSION))
            .expect("expected to serialize to cbor");

    let contract =
        DataContract::from_raw_object(contract).expect("should create a contract from cbor");

    drive
        .apply_contract_with_serialization(
            &contract,
            contract_cbor.clone(),
            BlockInfo::default(),
            true,
            None,
            None,
        )
        .expect("should apply contract");

    // Create document

    let created_at = 1647535750329_u64;

    let document = platform_value!({
       "$protocolVersion": 1u32,
       "$id": "DLRWw2eRbLAW5zDU2c7wwsSFQypTSZPhFYzpY48tnaXN",
       "$type": "indexedDocument",
       "$dataContractId": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
       "$ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
       "$revision": 1 as Revision,
       "firstName": "myName",
       "lastName": "lastName",
       "$createdAt": created_at,
       "$updatedAt": created_at,
    });

    let serialized_document =
        cbor_serializer::serializable_value_to_cbor(&document, Some(defaults::PROTOCOL_VERSION))
            .expect("expected to serialize to cbor");

    drive
        .add_cbor_serialized_document_for_serialized_contract(
            serialized_document.as_slice(),
            contract_cbor.as_slice(),
            "indexedDocument",
            None,
            true,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
        )
        .expect("should add document");

    // Query document

    let query_cbor = cbor!({
        "where" => [
            ["$createdAt", "==", created_at]
        ],
    })
    .expect("should create cbor");

    let query_bytes = cbor_serializer::serializable_value_to_cbor(&query_cbor, None)
        .expect("should serialize cbor value to bytes");

    let document_type = contract
        .document_type_for_name("indexedDocument")
        .expect("should get document type");

    let query = DriveQuery::from_cbor(&query_bytes, &contract, document_type)
        .expect("should create a query from cbor");

    assert_eq!(
        query.internal_clauses.equal_clauses.get("$createdAt"),
        Some(&WhereClause {
            field: "$createdAt".to_string(),
            operator: WhereOperator::Equal,
            value: Value::U64(created_at)
        })
    );

    let query_result = drive
        .query_documents(query, None, false, None)
        .expect("should query documents");

    assert_eq!(query_result.documents.len(), 1);
}

#[cfg(feature = "full")]
#[test]
#[ignore]
fn pwd() {
    let working_dir = std::env::current_dir().unwrap();
    println!("{}", working_dir.display());
}
