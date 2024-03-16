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
use dpp::data_contract::DataContractFactory;
#[cfg(feature = "full")]
use drive::common;
#[cfg(feature = "full")]
use drive::common::setup_contract;
#[cfg(feature = "full")]
use drive::drive::batch::GroveDbOpBatch;
#[cfg(feature = "full")]
use drive::drive::config::DriveConfig;
#[cfg(feature = "full")]
use drive::drive::flags::StorageFlags;
#[cfg(feature = "full")]
use drive::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
#[cfg(feature = "full")]
use drive::drive::Drive;
#[cfg(feature = "full")]
use drive::error::{query::QuerySyntaxError, Error};
#[cfg(feature = "full")]
use drive::query::DriveQuery;
#[cfg(feature = "full")]
#[cfg(test)]
use drive::tests::helpers::setup::setup_drive;
#[cfg(feature = "full")]
use grovedb::TransactionArg;
use rand::random;
#[cfg(feature = "full")]
use rand::seq::SliceRandom;
#[cfg(feature = "full")]
use rand::{Rng, SeedableRng};
#[cfg(feature = "full")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "full")]
use serde_json::json;
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
use dpp::document::Document;
#[cfg(feature = "full")]
use dpp::platform_value::Value;
use dpp::platform_value::{platform_value, Bytes32, Identifier};

#[cfg(feature = "full")]
use base64::Engine;
#[cfg(feature = "full")]
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::serialization_traits::{
    DocumentCborMethodsV0, DocumentPlatformConversionMethodsV0, DocumentPlatformValueMethodsV0,
};
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::identity::TimestampMillis;
use dpp::platform_value;

#[cfg(feature = "full")]
use dpp::prelude::DataContract;

use dpp::tests::json_document::json_document_to_contract;
#[cfg(feature = "full")]
use dpp::util::cbor_serializer;

use dpp::version::PlatformVersion;

use drive::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
#[cfg(feature = "full")]
use drive::drive::contract::test_helpers::add_init_contracts_structure_operations;

use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
#[cfg(feature = "full")]
use drive::drive::document::query::QuerySerializedDocumentsOutcome;
use drive::drive::object_size_info::DocumentInfo;
use drive::drive::object_size_info::DocumentInfo::DocumentRefInfo;

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
pub fn setup_family_tests(count: u32, seed: u64) -> (Drive, DataContract) {
    let drive_config = DriveConfig::default();

    let drive = setup_drive(Some(drive_config));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    let platform_version = PlatformVersion::latest();

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction), &platform_version.drive)
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
        let document_cbor = cbor_serializer::serializable_value_to_cbor(&value, Some(0))
            .expect("expected to serialize to cbor");
        let document = Document::from_cbor(document_cbor.as_slice(), None, None, platform_version)
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
                platform_version,
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
pub fn setup_family_tests_with_nulls(count: u32, seed: u64) -> (Drive, DataContract) {
    let drive_config = DriveConfig::default();

    let drive = setup_drive(Some(drive_config));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    let platform_version = PlatformVersion::latest();

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction), &platform_version.drive)
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
        let document_cbor = cbor_serializer::serializable_value_to_cbor(&value, Some(0))
            .expect("expected to serialize to cbor");
        let document = Document::from_cbor(document_cbor.as_slice(), None, None, platform_version)
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
                platform_version,
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
pub fn setup_family_tests_only_first_name_index(count: u32, seed: u64) -> (Drive, DataContract) {
    let drive_config = DriveConfig::default();

    let drive = setup_drive(Some(drive_config));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    let platform_version = PlatformVersion::latest();

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction), &platform_version.drive)
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
        let document_cbor = cbor_serializer::serializable_value_to_cbor(&value, Some(0))
            .expect("expected to serialize to cbor");
        let document = Document::from_cbor(document_cbor.as_slice(), None, None, platform_version)
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
                platform_version,
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
    let platform_version = PlatformVersion::latest();

    let domains = Domain::random_domains_in_parent(20, 100, "dash");
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        false,
        platform_version,
    )
    .expect("expected to get cbor contract");

    let document_type = contract
        .document_type_for_name("domain")
        .expect("expected to get document type");
    for domain in domains {
        let value = platform_value::to_value(domain).expect("expected value");

        let mut document =
            Document::from_platform_value(value, platform_version).expect("expected value");
        document.set_revision(Some(1));
        let serialized = <Document as DocumentPlatformConversionMethodsV0>::serialize(
            &document,
            document_type,
            platform_version,
        )
        .expect("should serialize");
        let _deserialized = Document::from_bytes(&serialized, document_type, platform_version)
            .expect("expected to deserialize domain document");
    }
}

#[cfg(feature = "full")]
#[test]
fn test_serialization_and_deserialization_with_null_values_should_fail_if_required() {
    let platform_version = PlatformVersion::latest();

    let contract = json_document_to_contract(
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        false,
        platform_version,
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

    let value = platform_value::to_value(domain).expect("expected value");
    let mut document =
        Document::from_platform_value(value, platform_version).expect("expected value");
    document.set_revision(Some(1));

    <Document as DocumentPlatformConversionMethodsV0>::serialize(
        &document,
        document_type,
        platform_version,
    )
    .expect_err("expected to not be able to serialize domain document");
}

#[cfg(feature = "full")]
#[test]
fn test_serialization_and_deserialization_with_null_values() {
    let platform_version = PlatformVersion::latest();
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/dpns/dpns-contract-label-not-required.json",
        false,
        platform_version,
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
    let mut document =
        Document::from_platform_value(value, platform_version).expect("expected value");
    document.set_revision(Some(1));
    let serialized =
        DocumentPlatformConversionMethodsV0::serialize(&document, document_type, platform_version)
            .expect("expected to be able to serialize domain document");

    Document::from_bytes(&serialized, document_type, platform_version)
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
    contract: &DataContract,
    transaction: TransactionArg,
    count: u32,
    seed: u64,
) {
    let platform_version = PlatformVersion::latest();
    let domains = Domain::random_domains_in_parent(count, seed, "dash");
    let document_type = contract
        .document_type_for_name("domain")
        .expect("expected to get document type");
    for domain in domains {
        let value = platform_value::to_value(domain).expect("expected value");
        let document =
            Document::from_platform_value(value, platform_version).expect("expected value");

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
                platform_version,
            )
            .expect("document should be inserted");
    }
}

#[cfg(feature = "full")]
/// Sets up and inserts random domain name data to the DPNS contract to test queries on.
pub fn setup_dpns_tests_with_batches(count: u32, seed: u64) -> (Drive, DataContract) {
    let drive = setup_drive(Some(DriveConfig::default()));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    let platform_version = PlatformVersion::latest();

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction), &platform_version.drive)
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
/// Sets up the References contract to test queries on.
pub fn setup_references_tests(_count: u32, _seed: u64) -> (Drive, DataContract) {
    let drive = setup_drive(Some(DriveConfig::default()));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    let platform_version = PlatformVersion::latest();

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction), &platform_version.drive)
        .expect("expected to create contracts tree successfully");

    // setup code
    let contract = setup_contract(
        &drive,
        "tests/supporting_files/contract/references/references_with_contract_history.json",
        None,
        Some(&db_transaction),
    );

    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "full")]
/// Sets up and inserts random domain name data to the DPNS contract to test queries on.
pub fn setup_dpns_tests_label_not_required(count: u32, seed: u64) -> (Drive, DataContract) {
    let drive = setup_drive(Some(DriveConfig::default()));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    let platform_version = PlatformVersion::latest();

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction), &platform_version.drive)
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
pub fn setup_dpns_test_with_data(path: &str) -> (Drive, DataContract) {
    let drive = setup_drive(None);

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    let platform_version = PlatformVersion::latest();

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction), &platform_version.drive)
        .expect("expected to create contracts tree successfully");

    let contract = setup_contract(
        &drive,
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        None,
        Some(&db_transaction),
    );

    let file = File::open(path).expect("should read domains from file");

    for domain_json in io::BufReader::new(file).lines().map_while(Result::ok) {
        let domain_json: serde_json::Value =
            serde_json::from_str(&domain_json).expect("should parse json");

        let domain_cbor = cbor_serializer::serializable_value_to_cbor(&domain_json, Some(0))
            .expect("expected to serialize to cbor");

        let domain = Document::from_cbor(&domain_cbor, None, None, platform_version)
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
                platform_version,
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

    let platform_version = PlatformVersion::latest();

    let people = Person::random_people(10, 73409);
    for person in people {
        let value = serde_json::to_value(person).expect("serialized person");
        let document_cbor = cbor_serializer::serializable_value_to_cbor(&value, Some(0))
            .expect("expected to serialize to cbor");
        let document = Document::from_cbor(document_cbor.as_slice(), None, None, platform_version)
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
                platform_version,
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

    let platform_version = PlatformVersion::latest();

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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_non_existence_reference_proof_single_index() {
    let (drive, contract) = setup_family_tests_only_first_name_index(0, 73509);

    let platform_version = PlatformVersion::latest();

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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_family_basic_queries() {
    let (drive, contract) = setup_family_tests(10, 73509);
    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        66, 138, 16, 202, 64, 147, 26, 115, 247, 29, 235, 148, 80, 154, 249, 204, 148, 59, 248, 83,
        118, 140, 196, 38, 24, 59, 190, 48, 212, 130, 220, 215,
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
            Some(platform_version.protocol_version),
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
            Some(platform_version.protocol_version),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
            Some(platform_version.protocol_version),
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
            Some(platform_version.protocol_version),
        )
        .expect("query should be executed");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let document = Document::from_bytes(
        results.first().unwrap().as_slice(),
        person_document_type,
        platform_version,
    )
    .expect("we should be able to deserialize from bytes");
    let last_name = document
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
            Some(platform_version.protocol_version),
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
            Some(platform_version.protocol_version),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
            Some(platform_version.protocol_version),
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
            Some(platform_version.protocol_version),
        )
        .expect("query should be executed");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let document = Document::from_bytes(
        results.first().unwrap().as_slice(),
        person_document_type,
        platform_version,
    )
    .expect("we should be able to deserialize from bytes");
    let last_name = document
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
            Some(platform_version.protocol_version),
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
            Some(platform_version.protocol_version),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
            Some(platform_version.protocol_version),
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
            Some(platform_version.protocol_version),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");
    assert_eq!(results.len(), 5);

    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let ages: HashMap<String, u8> = results
        .into_iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let name_value = document
                .get("firstName")
                .expect("we should be able to get the first name");
            let name = name_value
                .as_text()
                .expect("the first name should be a string")
                .to_string();
            let age_value = document
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
    let person_cbor = cbor_serializer::serializable_value_to_cbor(&serialized_person, Some(0))
        .expect("expected to serialize to cbor");
    let document = Document::from_cbor(person_cbor.as_slice(), None, None, platform_version)
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
            platform_version,
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
    let person_cbor = cbor_serializer::serializable_value_to_cbor(&serialized_person, Some(0))
        .expect("expected to serialize to cbor");
    let document = Document::from_cbor(person_cbor.as_slice(), None, None, platform_version)
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
            platform_version,
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 2);

    let last_person = Document::from_bytes(
        results.first().unwrap().as_slice(),
        document_type,
        platform_version,
    )
    .expect("we should be able to deserialize the document");

    assert_eq!(
        last_person.id().to_vec(),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 2);

    let last_person = Document::from_bytes(
        results.first().unwrap().as_slice(),
        document_type,
        platform_version,
    )
    .expect("we should be able to deserialize the document");

    assert_eq!(
        last_person.id().to_vec(),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 12);

    let last_person = Document::from_bytes(
        results.first().unwrap().as_slice(),
        document_type,
        platform_version,
    )
    .expect("we should be able to deserialize the document");

    assert_eq!(
        last_person.id().to_vec(),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // query empty contract with nested path queries

    let dashpay_contract = json_document_to_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract.json",
        false,
        platform_version,
    )
    .expect("expected to get cbor document");

    drive
        .apply_contract(
            &dashpay_contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
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
        .query_documents_cbor_from_contract(
            &dashpay_contract,
            dashpay_contract
                .document_type_for_name("contactRequest")
                .expect("should have contact document type"),
            &query_cbor,
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_cbor_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let result = drive.query_documents_cbor_from_contract(
        &contract,
        person_document_type,
        query_cbor.as_slice(),
        None,
        Some(&db_transaction),
        Some(platform_version.protocol_version),
    );

    assert!(
        matches!(result, Err(Error::Query(QuerySyntaxError::StartDocumentNotFound(message))) if message == "startAt document not found")
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");

    let result = drive.query_documents_cbor_from_contract(
        &contract,
        person_document_type,
        query_cbor.as_slice(),
        None,
        Some(&db_transaction),
        Some(platform_version.protocol_version),
    );

    assert!(
        matches!(result, Err(Error::Query(QuerySyntaxError::StartDocumentNotFound(message))) if message == "startAfter document not found")
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
            183, 156, 84, 144, 94, 45, 111, 118, 255, 200, 166, 248, 252, 127, 216, 105, 83, 124,
            126, 241, 19, 191, 32, 188, 112, 101, 249, 194, 188, 224, 188, 162
        ],
    );
}

#[cfg(feature = "full")]
#[test]
fn test_family_person_update() {
    let (drive, contract) = setup_family_tests(10, 73509);

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let mut rng = rand::rngs::StdRng::seed_from_u64(84594);
    let id_bytes = bs58::decode("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD")
        .into_vec()
        .expect("this should decode");

    let owner_id_bytes = bs58::decode("BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj")
        .into_vec()
        .expect("this should decode");

    let fixed_person = Person {
        id: id_bytes.clone(),
        owner_id: owner_id_bytes.clone(),
        first_name: String::from("Wisdom"),
        middle_name: String::from("Madman"),
        last_name: String::from("Ogwu"),
        age: rng.gen_range(0..85),
    };
    let serialized_person = serde_json::to_value(fixed_person).expect("serialized person");
    let person_cbor = cbor_serializer::serializable_value_to_cbor(&serialized_person, Some(0))
        .expect("expected to serialize to cbor");
    let document = Document::from_cbor(person_cbor.as_slice(), None, None, platform_version)
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
            platform_version,
        )
        .expect("document should be inserted");

    let updated_fixed_person = Person {
        id: id_bytes,
        owner_id: owner_id_bytes,
        first_name: String::from("Wisdom"),
        middle_name: String::from("Madabuchukwu"),
        last_name: String::from("Ogwu"),
        age: rng.gen_range(0..85),
    };
    let serialized_person = serde_json::to_value(updated_fixed_person).expect("serialized person");
    let person_cbor = cbor_serializer::serializable_value_to_cbor(&serialized_person, Some(0))
        .expect("expected to serialize to cbor");
    let document = Document::from_cbor(person_cbor.as_slice(), None, None, platform_version)
        .expect("document should be properly deserialized");

    let fee = drive
        .update_document_for_contract(
            &document,
            &contract,
            document_type,
            None,
            BlockInfo::genesis(),
            true,
            None,
            Some(&db_transaction),
            platform_version,
        )
        .expect("expected to override document");
    assert!(fee.storage_fee > 0);

    let query_value = json!({
        "where": [
            ["firstName", "==", "Wisdom"]
        ],
        "limit": 1,
    });
    let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");
    let person_document_type = contract
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");

    assert_eq!(results.len(), 1);

    drive
        .commit_transaction(db_transaction, &platform_version.drive)
        .expect("expected to commit transaction");

    let (proof, _fee) = query
        .clone()
        .execute_with_proof(&drive, None, None, platform_version)
        .expect("expected proof to be generated");

    let (_root_hash, documents) = query
        .verify_proof(&proof, platform_version)
        .expect("expected to verify proof");

    assert_eq!(documents.len(), 1);
}

#[cfg(feature = "full")]
#[test]
fn test_family_starts_at_queries() {
    let (drive, contract) = setup_family_tests(10, 73509);

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        66, 138, 16, 202, 64, 147, 26, 115, 247, 29, 235, 148, 80, 154, 249, 204, 148, 59, 248, 83,
        118, 140, 196, 38, 24, 59, 190, 48, 212, 130, 220, 215,
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");

    let reduced_names_after: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");

    let reduced_names_after: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");

    let reduced_names_after: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("proof should be executed");
    assert_eq!(results.len(), 2);

    let reduced_names_after: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let first_name_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
    let (drive, contract) = setup_family_tests(10, 73509);
    let person_document_type = contract
        .document_type_for_name("person")
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
    let query1 = DriveQuery::from_cbor(
        query_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("should build query");

    let sql_string = "select * from person order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
        .expect("should build query");

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
    let query1 = DriveQuery::from_cbor(
        query_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("should build query");

    let sql_string = "select * from person where firstName = 'Chris'";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
        .expect("should build query");

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
    let query1 = DriveQuery::from_cbor(
        query_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("should build query");

    let sql_string =
        "select * from person where firstName < 'Chris' order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
        .expect("should build query");

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
    let query1 = DriveQuery::from_cbor(
        query_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("should build query");

    let sql_string =
        "select * from person where firstName like 'C%' order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
        .expect("should build query");

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
    let query1 = DriveQuery::from_cbor(
        query_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("should build query");

    let sql_string = "select * from person where firstName > 'Chris' and firstName <= 'Noellyn' order by firstName asc limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
        .expect("should build query");

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
    let query1 = DriveQuery::from_cbor(
        query_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("should build query");

    let sql_string =
        "select * from person where firstName in ('a', 'b') order by firstName limit 100";
    let query2 = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
        .expect("should build query");

    assert_eq!(query1, query2);
}

#[cfg(feature = "full")]
#[test]
fn test_family_with_nulls_query() {
    let (drive, contract) = setup_family_tests_with_nulls(10, 30004);

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        120, 51, 207, 201, 227, 63, 44, 130, 175, 117, 220, 200, 56, 145, 137, 7, 203, 88, 178, 88,
        211, 192, 197, 201, 253, 79, 18, 50, 43, 156, 204, 153,
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
        .document_type_for_name("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        person_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .clone()
        .into_iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            document
                .get("firstName")
                .map(|value| {
                    let first_name_value = value
                        .as_text()
                        .expect("the normalized label should be a string");
                    String::from(first_name_value)
                })
                .unwrap_or_default()
        })
        .collect();

    assert_eq!(names, all_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let ids: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), person_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            base64::engine::general_purpose::STANDARD.encode(document.id().as_slice())
        })
        .collect();

    for i in 0..10 {
        drive
            .delete_document_for_contract(
                base64::engine::general_purpose::STANDARD
                    .decode(ids.get(i).unwrap())
                    .expect("expected to decode from base64")
                    .try_into()
                    .expect("expected to get 32 bytes"),
                &contract,
                "person",
                BlockInfo::genesis(),
                true,
                Some(&db_transaction),
                platform_version,
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

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    // Make sure the state is deterministic
    let expected_app_hash = vec![
        66, 138, 16, 202, 64, 147, 26, 115, 247, 29, 235, 148, 80, 154, 249, 204, 148, 59, 248, 83,
        118, 140, 196, 38, 24, 59, 190, 48, 212, 130, 220, 215,
    ];

    assert_eq!(root_hash.as_slice(), expected_app_hash);

    // Make sure contract is not cached
    let contract_ref = drive
        .get_cached_contract_with_fetch_info(
            *contract.id_ref().as_bytes(),
            Some(&db_transaction),
            &platform_version.drive,
        )
        .expect("should return a contract ref");

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
            *contract.id_ref().as_bytes(),
            "person",
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
        )
        .expect("query should be executed");

    assert_eq!(items.len(), 10);

    // Cache was populated and there only two ref two the cached fetched info (here and cache)
    let contract_ref = drive
        .get_cached_contract_with_fetch_info(
            *contract.id_ref().as_bytes(),
            Some(&db_transaction),
            &platform_version.drive,
        )
        .expect("should return a contract ref")
        .expect("expected a reference counter to the contract");

    assert_eq!(Arc::strong_count(&contract_ref), 2);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_contract_verification() {
    let (drive, contract) = setup_dpns_tests_with_batches(10, 11456);

    let platform_version = PlatformVersion::latest();

    let root_hash = drive
        .grove
        .root_hash(None)
        .unwrap()
        .expect("there is always a root hash");

    let contract_proof = drive
        .prove_contract(contract.id().into_buffer(), None, platform_version)
        .expect("expected to get proof");
    let (proof_root_hash, proof_returned_contract) = Drive::verify_contract(
        contract_proof.as_slice(),
        None,
        false,
        false,
        contract.id().into_buffer(),
        platform_version,
    )
    .expect("expected to get contract from proof");

    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(
        contract,
        proof_returned_contract.expect("expected to get a contract")
    );
}

#[test]
fn test_contract_keeps_history_fetch_and_verification() {
    let (drive, contract) = setup_references_tests(10, 3334);

    let platform_version = PlatformVersion::latest();

    let root_hash = drive
        .grove
        .root_hash(None)
        .unwrap()
        .expect("there is always a root hash");

    drive
        .fetch_contract(
            contract.id().to_buffer(),
            None,
            None,
            None,
            platform_version,
        )
        .unwrap()
        .expect("expected to be able to fetch a contract")
        .expect("expected a contract to be present");

    let contract_proof = drive
        .prove_contract(contract.id().into_buffer(), None, platform_version)
        .expect("expected to get proof");
    let (proof_root_hash, proof_returned_contract) = Drive::verify_contract(
        contract_proof.as_slice(),
        None,
        false,
        false,
        contract.id().into_buffer(),
        platform_version,
    )
    .expect("expected to get contract from proof");

    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(
        contract,
        proof_returned_contract.expect("expected to get a contract")
    );
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query() {
    let (drive, contract) = setup_dpns_tests_with_batches(10, 11456);

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        104, 127, 253, 216, 170, 92, 9, 250, 226, 44, 134, 38, 89, 10, 135, 4, 121, 65, 255, 136,
        149, 217, 217, 169, 108, 37, 241, 86, 57, 214, 131, 96,
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let ids: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            hex::encode(document.id().as_slice())
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);

    let record_id_base68: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");

            let records_value = document
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");

    assert_eq!(results.len(), 10);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let query_value = json!({
        "orderBy": [["records.dashUniqueIdentityId", "desc"]],
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let domain_document_type = contract
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");

    let result = drive
        .query_documents_cbor_from_contract(
            &contract,
            domain_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
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
            Some(platform_version.protocol_version),
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

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let query_value = json!({
        "orderBy": [["records.dashUniqueIdentityId", "desc"]],
    });

    let query_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("expected to serialize to cbor");

    let domain_document_type = contract
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");

    let result = drive
        .query_documents_cbor_from_contract(
            &contract,
            domain_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
            Some(platform_version.protocol_version),
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
            Some(platform_version.protocol_version),
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

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        104, 127, 253, 216, 170, 92, 9, 250, 226, 44, 134, 38, 89, 10, 135, 4, 121, 65, 255, 136,
        149, 217, 217, 169, 108, 37, 241, 86, 57, 214, 131, 96,
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_start_after() {
    // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
    let (drive, contract) = setup_dpns_tests_with_batches(10, 11456);

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        104, 127, 253, 216, 170, 92, 9, 250, 226, 44, 134, 38, 89, 10, 135, 4, 121, 65, 255, 136,
        149, 217, 217, 169, 108, 37, 241, 86, 57, 214, 131, 96,
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_start_at_desc() {
    // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
    let (drive, contract) = setup_dpns_tests_with_batches(10, 11456);

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        104, 127, 253, 216, 170, 92, 9, 250, 226, 44, 134, 38, 89, 10, 135, 4, 121, 65, 255, 136,
        149, 217, 217, 169, 108, 37, 241, 86, 57, 214, 131, 96,
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_dpns_query_start_after_desc() {
    // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
    let (drive, contract) = setup_dpns_tests_with_batches(10, 11456);

    let platform_version = PlatformVersion::latest();

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");

    let expected_app_hash = vec![
        104, 127, 253, 216, 170, 92, 9, 250, 226, 44, 134, 38, 89, 10, 135, 4, 121, 65, 255, 136,
        149, 217, 217, 169, 108, 37, 241, 86, 57, 214, 131, 96,
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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

    let platform_version = PlatformVersion::latest();

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
    let document0 = Document::from_platform_value(value0, platform_version)
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
            platform_version,
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
    let document_cbor1 = cbor_serializer::serializable_value_to_cbor(&value1, Some(0))
        .expect("expected to serialize to cbor");
    let document1 = Document::from_cbor(document_cbor1.as_slice(), None, None, platform_version)
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
            platform_version,
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
        11, 19, 155, 212, 245, 198, 124, 117, 180, 86, 195, 68, 236, 38, 209, 25, 16, 186, 122, 12,
        9, 73, 22, 156, 179, 16, 190, 94, 19, 60, 210, 82,
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");

    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            document
                .get("normalizedLabel")
                .map(|value| {
                    let normalized_label = value
                        .as_text()
                        .expect("the normalized label should be a string");
                    String::from(normalized_label)
                })
                .unwrap_or_default()
        })
        .collect();

    let a_names = [
        "".to_string(),
        "amalle".to_string(),
        "anna-diane".to_string(),
    ];

    assert_eq!(names, a_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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

    let platform_version = PlatformVersion::latest();

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
    let document_cbor0 = cbor_serializer::serializable_value_to_cbor(&value0, Some(0))
        .expect("expected to serialize to cbor");
    let document0 = Document::from_cbor(document_cbor0.as_slice(), None, None, platform_version)
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
            platform_version,
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
    let document_cbor1 = cbor_serializer::serializable_value_to_cbor(&value1, Some(0))
        .expect("expected to serialize to cbor");
    let document1 = Document::from_cbor(document_cbor1.as_slice(), None, None, platform_version)
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
            platform_version,
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
        11, 19, 155, 212, 245, 198, 124, 117, 180, 86, 195, 68, 236, 38, 209, 25, 16, 186, 122, 12,
        9, 73, 22, 156, 179, 16, 190, 94, 19, 60, 210, 82,
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");

    // We are commenting this out on purpose to make it easier to find
    // let mut query_operations: Vec<QueryOperation> = vec![];
    // let path_query = query
    //     .construct_path_query_operations(&drive, Some(&db_transaction), &mut query_operations)
    //     .expect("expected to construct a path query");
    // println!("{:#?}", path_query);
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            let normalized_label_value = document
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
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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

    let platform_version = PlatformVersion::latest();

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
    let document_cbor0 = cbor_serializer::serializable_value_to_cbor(&value0, Some(0))
        .expect("expected to serialize to cbor");
    let document0 = Document::from_cbor(document_cbor0.as_slice(), None, None, platform_version)
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
            platform_version,
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
    let document_cbor1 = cbor_serializer::serializable_value_to_cbor(&value1, Some(0))
        .expect("expected to serialize to cbor");
    let document1 = Document::from_cbor(document_cbor1.as_slice(), None, None, platform_version)
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
            platform_version,
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
        11, 19, 155, 212, 245, 198, 124, 117, 180, 86, 195, 68, 236, 38, 209, 25, 16, 186, 122, 12,
        9, 73, 22, 156, 179, 16, 190, 94, 19, 60, 210, 82,
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let docs: Vec<Vec<u8>> = results
        .clone()
        .into_iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            document.id().to_vec()
        })
        .collect();

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let docs: Vec<Vec<u8>> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            document.id().to_vec()
        })
        .collect();

    // The explanation is a little interesting
    // domain1 is smaller than domain0
    // however on the lowest lever the order never matters, so we are always ascending on the id
    // hence we will get domain1
    let expected_docs = [domain1_id.to_vec(), domain0_id.to_vec()];

    assert_eq!(docs, expected_docs);
    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
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
        .document_type_for_name("domain")
        .expect("contract should have a domain document type");
    let query = DriveQuery::from_cbor(
        where_cbor.as_slice(),
        &contract,
        domain_document_type,
        &drive.config,
    )
    .expect("query should be built");
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document =
                Document::from_bytes(result.as_slice(), domain_document_type, platform_version)
                    .expect("we should be able to deserialize the document");
            document
                .get("normalizedLabel")
                .map(|value| {
                    let normalized_label = value
                        .as_text()
                        .expect("the normalized label should be a string");
                    String::from(normalized_label)
                })
                .unwrap_or_default()
        })
        .collect();

    let a_names = ["amalle".to_string(), "".to_string()];

    assert_eq!(names, a_names);

    let (proof_root_hash, proof_results, _) = query
        .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
        .expect("we should be able to a proof");
    assert_eq!(root_hash, proof_root_hash);
    assert_eq!(results, proof_results);
}

#[cfg(feature = "full")]
#[test]
fn test_query_a_b_c_d_e_contract() {
    let drive: Drive = setup_drive_with_initial_state_structure();

    let platform_version = PlatformVersion::latest();

    // Create a contract

    let block_info = BlockInfo::default();
    let owner_id = dpp::identifier::Identifier::new([2u8; 32]);

    let documents = platform_value!({
      "testDocument": {
        "type": "object",
        "properties": {
          "a": {
            "type": "integer",
            "position": 0
          },
          "b": {
            "type": "integer",
            "position": 1
          },
          "c": {
            "type": "integer",
            "position": 2
          },
          "d": {
            "type": "integer",
            "position": 3
          },
          "e": {
            "type": "integer",
            "position": 4
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

    let factory =
        DataContractFactory::new(platform_version.protocol_version).expect("should create factory");

    let contract = factory
        .create_with_value_config(owner_id, 0, documents, None, None)
        .expect("data in fixture should be correct")
        .data_contract_owned();

    drive
        .apply_contract(
            &contract,
            block_info,
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
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
                .document_type_for_name(&document_type)
                .expect("should have this document type"),
            &query_cbor,
            None,
            None,
            Some(platform_version.protocol_version),
        )
        .expect("should perform query");
}

#[cfg(feature = "full")]
#[test]
fn test_query_documents_by_created_at() {
    let drive = setup_drive_with_initial_state_structure();

    let platform_version = PlatformVersion::latest();

    let contract_value = platform_value!({
        "$format_version": "0",
        "id": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
        "version": 1,
        "ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
        "documentSchemas": {
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
                        "position": 0
                    },
                    "lastName": {
                        "type": "string",
                        "maxLength": 63,
                        "position": 1
                    }
                },
                "required": ["firstName", "$createdAt", "$updatedAt", "lastName"],
                "additionalProperties": false,
            },
        },
    });

    let contract = DataContract::from_value(contract_value, false, platform_version)
        .expect("should create a contract from cbor");

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            None,
            platform_version,
        )
        .expect("should apply contract");

    // Create document

    let created_at: TimestampMillis = 1647535750329;

    let document_value = platform_value!({
       "firstName": "myName",
       "lastName": "lastName",
       "$createdAt": created_at,
       "$updatedAt": created_at,
    });

    let document = contract
        .document_type_for_name("indexedDocument")
        .expect("should have indexedDocument type")
        .create_document_from_data(
            document_value,
            Identifier::random(),
            random(),
            random(),
            random(),
            platform_version,
        )
        .expect("should create document");

    let info = DocumentAndContractInfo {
        owned_document_info: OwnedDocumentInfo {
            document_info: DocumentInfo::DocumentOwnedInfo((document, None)),
            owner_id: None,
        },
        contract: &contract,
        document_type: contract
            .document_type_for_name("indexedDocument")
            .expect("should have indexedDocument type"),
    };

    drive
        .add_document_for_contract(
            info,
            true,
            BlockInfo::default(),
            true,
            None,
            platform_version,
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

    let query = DriveQuery::from_cbor(
        &query_bytes,
        &contract,
        document_type,
        &DriveConfig::default(),
    )
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
        .query_documents(
            query,
            None,
            false,
            None,
            Some(platform_version.protocol_version),
        )
        .expect("should query documents");

    assert_eq!(query_result.documents().len(), 1);
}

#[cfg(feature = "full")]
#[test]
#[ignore]
fn pwd() {
    let working_dir = std::env::current_dir().unwrap();
    println!("{}", working_dir.display());
}
