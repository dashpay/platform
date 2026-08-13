//! Query Tests
//!

use ciborium::cbor;
#[cfg(feature = "server")]
use dpp::data_contract::DataContractFactory;
#[cfg(feature = "server")]
use drive::config::DriveConfig;
#[cfg(feature = "server")]
use drive::drive::Drive;
#[cfg(feature = "server")]
use drive::error::{query::QuerySyntaxError, Error};
#[cfg(feature = "server")]
use drive::query::DriveDocumentQuery;
#[cfg(feature = "server")]
use drive::util::batch::GroveDbOpBatch;
#[cfg(feature = "server")]
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
#[cfg(feature = "server")]
use drive::util::storage_flags::StorageFlags;
#[cfg(feature = "server")]
#[cfg(test)]
use drive::util::test_helpers::setup::setup_drive;
#[cfg(feature = "server")]
use drive::util::test_helpers::setup_contract;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use rand::random;
#[cfg(feature = "server")]
use rand::seq::SliceRandom;
#[cfg(feature = "server")]
use rand::{Rng, SeedableRng};
#[cfg(feature = "server")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use serde_json::json;
#[cfg(feature = "server")]
use std::borrow::Cow;
use std::collections::BTreeMap;
#[cfg(feature = "server")]
use std::collections::HashMap;
#[cfg(feature = "server")]
use std::fs::File;
#[cfg(feature = "server")]
use std::io::{self, BufRead};
#[cfg(feature = "server")]
use std::option::Option::None;
#[cfg(feature = "server")]
use std::sync::Arc;

#[cfg(feature = "server")]
use dpp::document::Document;
#[cfg(feature = "server")]
use dpp::platform_value::Value;
use dpp::platform_value::{platform_value, Bytes32, Identifier};

#[cfg(feature = "server")]
use base64::Engine;
#[cfg(feature = "server")]
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::config::v0::DataContractConfigSettersV0;
use dpp::data_contract::config::v1::DataContractConfigSettersV1;
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::serialization_traits::{
    DocumentCborMethodsV0, DocumentPlatformConversionMethodsV0,
};
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::identity::TimestampMillis;
use dpp::platform_value;
use dpp::platform_value::string_encoding::Encoding;
#[cfg(feature = "server")]
use dpp::prelude::DataContract;
use dpp::serialization::ValueConvertible;
use dpp::tests::json_document::json_document_to_contract;
#[cfg(feature = "server")]
use dpp::util::cbor_serializer;
use dpp::version::fee::FeeVersion;
use dpp::version::PlatformVersion;
#[cfg(feature = "server")]
use drive::drive::contract::test_helpers::add_init_contracts_structure_operations;
use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use once_cell::sync::Lazy;
use rand::prelude::StdRng;

use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
#[cfg(feature = "server")]
use drive::drive::document::query::QuerySerializedDocumentsOutcome;
use drive::util::object_size_info::DocumentInfo;
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;

use drive::query::{WhereClause, WhereOperator};
use drive::util::test_helpers;
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;

/// Build a `Document` from an un-tagged `platform_value::Value` (e.g.
/// produced by `platform_value::to_value` over a serde-derived domain
/// struct) by inserting `$formatVersion: "0"` and routing through
/// canonical `ValueConvertible::from_object`. Replaces the deleted
/// `Document::from_platform_value` ingest path.
#[cfg(feature = "server")]
fn document_from_legacy_value(mut value: Value) -> Document {
    if let Value::Map(ref mut entries) = value {
        let has_tag = entries
            .iter()
            .any(|(k, _)| matches!(k, Value::Text(s) if s == "$formatVersion"));
        if !has_tag {
            entries.push((
                Value::Text("$formatVersion".to_string()),
                Value::Text("0".to_string()),
            ));
        }
    }
    Document::from_object(value).expect("expected document from legacy value")
}

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
impl Person {
    fn random_people(count: u32, seed: u64) -> Vec<Self> {
        let first_names = test_helpers::text_file_strings(
            "tests/supporting_files/contract/family/first-names.txt",
        );
        let middle_names = test_helpers::text_file_strings(
            "tests/supporting_files/contract/family/middle-names.txt",
        );
        let last_names = test_helpers::text_file_strings(
            "tests/supporting_files/contract/family/last-names.txt",
        );
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

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
impl PersonWithOptionalValues {
    fn random_people(count: u32, seed: u64) -> Vec<Self> {
        let first_names = test_helpers::text_file_strings(
            "tests/supporting_files/contract/family/first-names.txt",
        );
        let middle_names = test_helpers::text_file_strings(
            "tests/supporting_files/contract/family/middle-names.txt",
        );
        let last_names = test_helpers::text_file_strings(
            "tests/supporting_files/contract/family/last-names.txt",
        );
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

#[cfg(feature = "server")]
/// Inserts the test "family" contract and adds `count` documents containing randomly named people to it.
pub fn setup_family_tests(
    count: u32,
    seed: u64,
    platform_version: &PlatformVersion,
) -> (Drive, DataContract) {
    let drive_config = DriveConfig::default();

    let drive = setup_drive(Some(drive_config));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction), &platform_version.drive)
        .expect("expected to create contracts tree successfully");

    // setup code
    let contract = test_helpers::setup_contract(
        &drive,
        "tests/supporting_files/contract/family/family-contract.json",
        None,
        None,
        None::<fn(&mut DataContract)>,
        Some(&db_transaction),
        Some(platform_version),
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
                None,
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

#[cfg(feature = "server")]
/// Inserts the test "family" contract and adds `count` documents containing randomly named people to it.
pub fn setup_countable_family_tests(
    count: u32,
    seed: u64,
    platform_version: &PlatformVersion,
) -> (Drive, DataContract) {
    let drive_config = DriveConfig::default();

    let drive = setup_drive(Some(drive_config));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction), &platform_version.drive)
        .expect("expected to create contracts tree successfully");

    // setup code
    let contract = test_helpers::setup_contract(
        &drive,
        "tests/supporting_files/contract/family/family-contract-countable.json",
        None,
        None,
        None::<fn(&mut DataContract)>,
        Some(&db_transaction),
        Some(platform_version),
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
                None,
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

#[cfg(feature = "server")]
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
    let contract = test_helpers::setup_contract(
        &drive,
        "tests/supporting_files/contract/family/family-contract-fields-optional.json",
        None,
        None,
        None::<fn(&mut DataContract)>,
        Some(&db_transaction),
        None,
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
                None,
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

#[cfg(feature = "server")]
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
    let contract = test_helpers::setup_contract(
        &drive,
        "tests/supporting_files/contract/family/family-contract-only-first-name-index.json",
        None,
        None,
        None::<fn(&mut DataContract)>,
        Some(&db_transaction),
        None,
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
                None,
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

#[cfg(feature = "server")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Records {
    dash_unique_identity_id: Identifier,
}

#[cfg(feature = "server")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubdomainRules {
    allow_subdomains: bool,
}

#[cfg(feature = "server")]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Withdrawal {
    #[serde(rename = "$id")]
    pub id: Identifier, // Unique identifier for the withdrawal

    #[serde(rename = "$ownerId")]
    pub owner_id: Identifier, // Identity of the withdrawal owner

    #[serde(rename = "$createdAt")]
    pub created_at: TimestampMillis,

    #[serde(rename = "$updatedAt")]
    pub updated_at: TimestampMillis,

    pub transaction_index: Option<u32>, // Optional sequential index of the transaction
    pub transaction_sign_height: Option<u32>, // Optional Core height on which the transaction was signed
    pub amount: u64,                          // Amount to withdraw (minimum: 1000)
    pub core_fee_per_byte: u32,               // Fee in Duffs/Byte (minimum: 1, max: 4294967295)
    pub pooling: u8,                          // Pooling level (enum: 0, 1, 2)
    pub output_script: Vec<u8>,               // Byte array (size: 23-25)
    pub status: u8,                           // Status (enum: 0 - Pending, 1 - Signed, etc.)
}

#[cfg(feature = "server")]
#[test]
fn test_serialization_and_deserialization() {
    let platform_version = PlatformVersion::latest();

    let domains = Domain::random_domains_in_parent(20, None, 100, "dash");
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

        let mut document = document_from_legacy_value(value);
        document.set_revision(Some(1));
        let serialized = <Document as DocumentPlatformConversionMethodsV0>::serialize(
            &document,
            document_type,
            &contract,
            platform_version,
        )
        .expect("should serialize");
        let _deserialized = Document::from_bytes(&serialized, document_type, platform_version)
            .expect("expected to deserialize domain document");
    }
}

#[cfg(feature = "server")]
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
    let mut document = document_from_legacy_value(value);
    document.set_revision(Some(1));

    <Document as DocumentPlatformConversionMethodsV0>::serialize(
        &document,
        document_type,
        &contract,
        platform_version,
    )
    .expect_err("expected to not be able to serialize domain document");
}

#[cfg(feature = "server")]
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
    let mut document = document_from_legacy_value(value);
    document.set_revision(Some(1));
    let serialized = DocumentPlatformConversionMethodsV0::serialize(
        &document,
        document_type,
        &contract,
        platform_version,
    )
    .expect("expected to be able to serialize domain document");

    Document::from_bytes(&serialized, document_type, platform_version)
        .expect("expected to deserialize domain document");
}

#[cfg(feature = "server")]
impl Domain {
    /// Creates `count` random names as domain names for the given parent domain
    /// If total owners in None it will create a new owner id per domain.
    fn random_domains_in_parent(
        count: u32,
        total_owners: Option<u32>,
        seed: u64,
        normalized_parent_domain_name: &str,
    ) -> Vec<Self> {
        let first_names = test_helpers::text_file_strings(
            "tests/supporting_files/contract/family/first-names.txt",
        );
        let mut vec: Vec<Domain> = Vec::with_capacity(count as usize);
        let mut rng = StdRng::seed_from_u64(seed);

        let owners = if let Some(total_owners) = total_owners {
            if total_owners == 0 {
                return vec![];
            }
            (0..total_owners)
                .map(|_| Identifier::random_with_rng(&mut rng))
                .collect()
        } else {
            vec![]
        };

        for _i in 0..count {
            let label = first_names.choose(&mut rng).unwrap();
            let domain = Domain {
                id: Identifier::random_with_rng(&mut rng),
                owner_id: if total_owners.is_some() {
                    // Pick a random owner from the owners list
                    *owners.choose(&mut rng).unwrap()
                } else {
                    Identifier::random_with_rng(&mut rng)
                },
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

#[cfg(feature = "server")]
impl Withdrawal {
    /// Generate `count` random withdrawals
    /// If `total_owners` is provided, assigns withdrawals to random owners from a predefined set.
    pub fn random_withdrawals(count: u32, total_owners: Option<u32>, seed: u64) -> Vec<Self> {
        let mut rng = StdRng::seed_from_u64(seed);

        // Generate a list of random owners if `total_owners` is provided
        let owners: Vec<Identifier> = if let Some(total) = total_owners {
            (0..total)
                .map(|_| Identifier::random_with_rng(&mut rng))
                .collect()
        } else {
            vec![]
        };

        let mut next_transaction_index = 1; // Start transaction index from 1

        let mut next_timestamp = 1732192259000;

        (0..count)
            .map(|_| {
                let owner_id = if !owners.is_empty() {
                    *owners.choose(&mut rng).unwrap()
                } else {
                    Identifier::random_with_rng(&mut rng)
                };

                // Determine the status randomly
                let status = if rng.gen_bool(0.5) {
                    0
                } else {
                    rng.gen_range(1..=4)
                }; // 0 = Pending, 1-4 = other statuses

                // Determine transaction index and sign height based on status
                let (transaction_index, transaction_sign_height) = if status == 0 {
                    (None, None) // No transaction index or sign height for Pending status
                } else {
                    let index = next_transaction_index;
                    next_transaction_index += 1; // Increment index for next withdrawal
                    (Some(index), Some(rng.gen_range(1..=500000))) // Set sign height only if transaction index is set
                };

                let output_script_length = rng.gen_range(23..=25);
                let output_script: Vec<u8> = (0..output_script_length).map(|_| rng.gen()).collect();

                let created_at = next_timestamp;

                next_timestamp += rng.gen_range(0..3) * 2000;

                Withdrawal {
                    id: Identifier::random_with_rng(&mut rng),
                    owner_id,
                    transaction_index,
                    transaction_sign_height,
                    amount: rng.gen_range(1000..=1_000_000), // Example range (minimum: 1000)
                    core_fee_per_byte: 0,                    // Always 0
                    pooling: 0,                              // Always 0
                    output_script,
                    status,
                    created_at,
                    updated_at: created_at,
                }
            })
            .collect()
    }
}

#[cfg(feature = "server")]
/// Adds `count` random domain names to the given contract
pub fn add_domains_to_contract(
    drive: &Drive,
    contract: &DataContract,
    transaction: TransactionArg,
    count: u32,
    total_owners: Option<u32>,
    seed: u64,
) {
    let platform_version = PlatformVersion::latest();
    let domains = Domain::random_domains_in_parent(count, total_owners, seed, "dash");
    let document_type = contract
        .document_type_for_name("domain")
        .expect("expected to get document type");
    for domain in domains {
        let value = platform_value::to_value(domain).expect("expected value");
        let document = document_from_legacy_value(value);

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
                None,
            )
            .expect("document should be inserted");
    }
}

#[cfg(feature = "server")]
/// Adds `count` random withdrawals to the given contract
pub fn add_withdrawals_to_contract(
    drive: &Drive,
    contract: &DataContract,
    transaction: TransactionArg,
    count: u32,
    total_owners: Option<u32>,
    seed: u64,
) {
    let platform_version = PlatformVersion::latest();
    let withdrawals = Withdrawal::random_withdrawals(count, total_owners, seed);
    let document_type = contract
        .document_type_for_name("withdrawal")
        .expect("expected to get document type");
    for domain in withdrawals {
        let value = platform_value::to_value(domain).expect("expected value");
        let document = document_from_legacy_value(value);

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
                None,
            )
            .expect("document should be inserted");
    }
}

#[cfg(feature = "server")]
/// Sets up and inserts random domain name data to the DPNS contract to test queries on.
pub fn setup_dpns_tests_with_batches(
    count: u32,
    total_owners: Option<u32>,
    seed: u64,
    platform_version: &PlatformVersion,
) -> (Drive, DataContract) {
    let drive = setup_drive(Some(DriveConfig::default()));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction), &platform_version.drive)
        .expect("expected to create contracts tree successfully");

    // setup code
    let contract = setup_contract(
        &drive,
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        None,
        None,
        None::<fn(&mut DataContract)>,
        Some(&db_transaction),
        Some(platform_version),
    );

    add_domains_to_contract(
        &drive,
        &contract,
        Some(&db_transaction),
        count,
        total_owners,
        seed,
    );
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "server")]
/// Sets up and inserts random withdrawal to the Withdrawal contract to test queries on.
pub fn setup_withdrawal_tests(
    count: u32,
    total_owners: Option<u32>,
    seed: u64,
) -> (Drive, DataContract) {
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
    let mut contract = setup_contract(
        &drive,
        "tests/supporting_files/contract/withdrawals/withdrawals-contract.json",
        None,
        None,
        None::<fn(&mut DataContract)>,
        Some(&db_transaction),
        None,
    );

    contract.config_mut().set_sized_integer_types_enabled(false);

    add_withdrawals_to_contract(
        &drive,
        &contract,
        Some(&db_transaction),
        count,
        total_owners,
        seed,
    );
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "server")]
/// Sets up the References contract to test queries on.
pub fn setup_references_tests(_count: u32, _seed: u64) -> (Drive, DataContract) {
    setup_references_tests_with_keeps_history(_count, _seed, false)
}

pub fn setup_references_tests_with_keeps_history(
    _count: u32,
    _seed: u64,
    keeps_history: bool,
) -> (Drive, DataContract) {
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
        None,
        Some(|contract: &mut DataContract| {
            contract.config_mut().set_keeps_history(keeps_history);
        }),
        Some(&db_transaction),
        None,
    );

    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "server")]
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
        None,
        None::<fn(&mut DataContract)>,
        Some(&db_transaction),
        None,
    );

    add_domains_to_contract(&drive, &contract, Some(&db_transaction), count, None, seed);
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[cfg(feature = "server")]
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
        None,
        None::<fn(&mut DataContract)>,
        Some(&db_transaction),
        None,
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
                None,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "server")]
    #[test]
    fn test_reference_proof_single_index() {
        let (drive, contract) = setup_family_tests_only_first_name_index(1, 73509);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_non_existence_reference_proof_single_index() {
        let (drive, contract) = setup_family_tests_only_first_name_index(0, 73509);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_family_basic_queries_first_version() {
        let platform_version = PlatformVersion::first();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            32, 210, 24, 196, 148, 43, 20, 34, 0, 116, 183, 136, 32, 210, 163, 183, 214, 6, 152,
            86, 46, 45, 88, 13, 23, 41, 37, 70, 129, 119, 211, 12,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
                None,
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
                None,
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
                76, 161, 17, 201, 152, 232, 129, 48, 168, 13, 49, 10, 218, 53, 118, 136, 165, 198,
                189, 116, 116, 22, 133, 92, 104, 165, 186, 249, 94, 81, 45, 20,
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
                249, 170, 70, 122, 181, 31, 35, 176, 175, 131, 70, 150, 250, 223, 194, 203, 175,
                200, 107, 252, 199, 227, 154, 105, 89, 57, 38, 85, 236, 192, 254, 88,
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
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        assert_eq!(
            root_hash.as_slice(),
            vec![
                251, 69, 177, 93, 128, 236, 106, 87, 205, 123, 80, 61, 44, 107, 186, 193, 22, 192,
                239, 7, 107, 110, 97, 197, 59, 245, 26, 12, 63, 91, 248, 231
            ],
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_family_basic_queries() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            53, 9, 163, 92, 116, 134, 17, 186, 21, 68, 156, 162, 47, 181, 214, 162, 253, 4, 246, 8,
            41, 187, 151, 152, 216, 164, 206, 110, 230, 176, 124, 225,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
                None,
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
                None,
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
                76, 161, 17, 201, 152, 232, 129, 48, 168, 13, 49, 10, 218, 53, 118, 136, 165, 198,
                189, 116, 116, 22, 133, 92, 104, 165, 186, 249, 94, 81, 45, 20,
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
                249, 170, 70, 122, 181, 31, 35, 176, 175, 131, 70, 150, 250, 223, 194, 203, 175,
                200, 107, 252, 199, 227, 154, 105, 89, 57, 38, 85, 236, 192, 254, 88,
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
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        assert_eq!(
            root_hash.as_slice(),
            vec![
                144, 154, 147, 246, 236, 57, 41, 67, 21, 26, 212, 158, 68, 159, 206, 26, 158, 50,
                252, 62, 143, 176, 149, 50, 19, 226, 239, 65, 112, 243, 225, 64
            ],
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_family_person_update() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);

        let epoch_change_fee_version_test: Lazy<CachedEpochIndexFeeVersions> =
            Lazy::new(|| BTreeMap::from([(0, FeeVersion::first())]));

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
                        document_info: DocumentRefInfo((&document, storage_flags.clone())),
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
                None,
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
        let serialized_person =
            serde_json::to_value(updated_fixed_person).expect("serialized person");
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
                storage_flags,
                Some(&db_transaction),
                platform_version,
                Some(&epoch_change_fee_version_test),
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_family_starts_at_queries() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            53, 9, 163, 92, 116, 134, 17, 186, 21, 68, 156, 162, 47, 181, 214, 162, 253, 4, 246, 8,
            41, 187, 151, 152, 216, 164, 206, 110, 230, 176, 124, 225,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_family_sql_query() {
        let platform_version = PlatformVersion::latest();
        // These helpers confirm that sql statements produce the same drive query
        // as their json counterparts, helpers above confirm that the json queries
        // produce the correct result set
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);
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
        let query1 = DriveDocumentQuery::from_cbor(
            query_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("should build query");

        let sql_string = "select * from person order by firstName asc limit 100";
        let query2 = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
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
        let query1 = DriveDocumentQuery::from_cbor(
            query_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("should build query");

        let sql_string = "select * from person where firstName = 'Chris'";
        let query2 = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
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
        let query1 = DriveDocumentQuery::from_cbor(
            query_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("should build query");

        let sql_string =
            "select * from person where firstName < 'Chris' order by firstName asc limit 100";
        let query2 = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
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
        let query1 = DriveDocumentQuery::from_cbor(
            query_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("should build query");

        let sql_string =
            "select * from person where firstName like 'C%' order by firstName asc limit 100";
        let query2 = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
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
        let query1 = DriveDocumentQuery::from_cbor(
            query_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("should build query");

        let sql_string = "select * from person where firstName > 'Chris' and firstName <= 'Noellyn' order by firstName asc limit 100";
        let query2 = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
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
        let query1 = DriveDocumentQuery::from_cbor(
            query_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("should build query");

        let sql_string =
            "select * from person where firstName in ('a', 'b') order by firstName limit 100";
        let query2 = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        assert_eq!(query1, query2);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_family_with_nulls_query() {
        let (drive, contract) = setup_family_tests_with_nulls(10, 30004);

        let platform_version = PlatformVersion::latest();

        let epoch_change_fee_version_test: Lazy<CachedEpochIndexFeeVersions> =
            Lazy::new(|| BTreeMap::from([(0, FeeVersion::first())]));

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            75, 38, 164, 96, 117, 46, 13, 23, 183, 41, 83, 163, 112, 55, 172, 37, 186, 36, 223, 39,
            106, 201, 46, 222, 167, 79, 236, 122, 12, 210, 29, 123,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
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
                    Some(&epoch_change_fee_version_test),
                )
                .expect("expected to be able to delete the document");
        }

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_query_with_cached_contract() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        // Make sure the state is deterministic
        let expected_app_hash = vec![
            53, 9, 163, 92, 116, 134, 17, 186, 21, 68, 156, 162, 47, 181, 214, 162, 253, 4, 246, 8,
            41, 187, 151, 152, 216, 164, 206, 110, 230, 176, 124, 225,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_dpns_query_contract_verification() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_dpns_tests_with_batches(10, None, 11456, platform_version);

        let root_hash = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
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
            .root_hash(None, &platform_version.drive.grove_version)
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

    #[test]
    fn test_contract_keeps_history_verify_with_unknown_history_flag() {
        // Regression test: when contract_known_keeps_history is None,
        // verification must still succeed for historical contracts.
        let (drive, contract) = setup_references_tests_with_keeps_history(10, 3334, true);
        let platform_version = PlatformVersion::latest();

        // Apply an update so the contract has an actual history entry and latest historical path.
        let mut latest_contract = contract.clone();
        latest_contract.set_version(contract.version() + 1);
        drive
            .apply_contract(
                &latest_contract,
                BlockInfo {
                    time_ms: 1,
                    ..Default::default()
                },
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("contract update should be applied");

        let root_hash = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let contract_proof = drive
            .prove_contract(latest_contract.id().into_buffer(), None, platform_version)
            .expect("expected to get proof");

        // Test 1: None (unknown) on a historical contract.
        // Verification must transparently retry with history enabled and return the
        // updated historical contract — callers do not need to know the keeps_history
        // flag in advance.
        let (proof_root_hash, proof_contract) = Drive::verify_contract(
            contract_proof.as_slice(),
            None,
            false,
            false,
            latest_contract.id().into_buffer(),
            platform_version,
        )
        .expect("verification with None should succeed for a historical contract");
        assert_eq!(root_hash, proof_root_hash);
        assert_eq!(
            latest_contract,
            proof_contract.expect(
                "historical contract should be recovered via retry when keeps_history is unknown",
            ),
        );

        // Test 2: Some(true) - direct historical, must succeed since proof was generated
        // for a historical contract
        let (proof_root_hash_2, proof_contract_2) = Drive::verify_contract(
            contract_proof.as_slice(),
            Some(true),
            false,
            false,
            latest_contract.id().into_buffer(),
            platform_version,
        )
        .expect("verification with Some(true) should succeed for historical contract");
        assert_eq!(root_hash, proof_root_hash_2);
        assert_eq!(
            latest_contract,
            proof_contract_2.expect("expected contract with explicit history flag")
        );

        // Test 3: Some(false) - explicit non-historical contract must verify to existing value.
        let (non_historical_drive, non_historical_contract) =
            setup_references_tests_with_keeps_history(10, 3334, false);
        let non_historical_root_hash = non_historical_drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");
        let non_historical_proof = non_historical_drive
            .prove_contract(
                non_historical_contract.id().into_buffer(),
                None,
                platform_version,
            )
            .expect("expected to get proof");
        let (proof_root_hash_3, proof_contract_3) = Drive::verify_contract(
            non_historical_proof.as_slice(),
            Some(false),
            false,
            false,
            non_historical_contract.id().into_buffer(),
            platform_version,
        )
        .expect("verification with Some(false) should return the existing contract");
        assert_eq!(non_historical_root_hash, proof_root_hash_3);
        assert_eq!(
            non_historical_contract,
            proof_contract_3.expect("expected contract with explicit non-history flag")
        );

        // Test 4: verify_contract_return_serialization with None on a historical contract
        // mirrors Test 1 — the historical contract must be recovered via retry when the
        // keeps_history flag is unknown.
        let (proof_root_hash_4, proof_contract_4) = Drive::verify_contract_return_serialization(
            contract_proof.as_slice(),
            None,
            false,
            false,
            latest_contract.id().into_buffer(),
            platform_version,
        )
        .expect("return_serialization with None should succeed for a historical contract");
        assert_eq!(root_hash, proof_root_hash_4);
        let (proof_contract_4_data, _proof_contract_4_bytes) = proof_contract_4.expect(
            "return_serialization should recover the historical contract when keeps_history is unknown",
        );
        assert_eq!(latest_contract, proof_contract_4_data);

        // Test 5: None (unknown) for a non-existent contract — verify_contract must
        // return Ok((root_hash, None)) for a genuine absence proof rather than retrying
        // with history and turning it into an error.
        let non_existent_id = [0xffu8; 32];
        let non_existent_proof = drive
            .prove_contract(non_existent_id, None, platform_version)
            .expect("expected to get proof for non-existent contract");

        let (proof_root_hash_5, proof_contract_5) = Drive::verify_contract(
            non_existent_proof.as_slice(),
            None,
            false,
            false,
            non_existent_id,
            platform_version,
        )
        .expect("verify_contract with None must succeed for a non-existent contract");
        assert_eq!(
            root_hash, proof_root_hash_5,
            "absence proof must report the same root hash"
        );
        assert!(
            proof_contract_5.is_none(),
            "verify_contract with None must return Ok((_, None)) for a non-existent contract"
        );

        // Test 6: same coverage for verify_contract_return_serialization.
        let (proof_root_hash_6, proof_contract_6) = Drive::verify_contract_return_serialization(
            non_existent_proof.as_slice(),
            None,
            false,
            false,
            non_existent_id,
            platform_version,
        )
        .expect(
            "verify_contract_return_serialization with None must succeed for a non-existent contract",
        );
        assert_eq!(
            root_hash, proof_root_hash_6,
            "absence proof must report the same root hash for return_serialization"
        );
        assert!(
            proof_contract_6.is_none(),
            "verify_contract_return_serialization with None must return Ok((_, None)) for a non-existent contract"
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_dpns_query_first_version() {
        let platform_version = PlatformVersion::first();
        let (drive, contract) = setup_dpns_tests_with_batches(10, None, 11456, platform_version);

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            142, 246, 25, 166, 52, 184, 158, 102, 192, 111, 173, 255, 155, 125, 53, 233, 98, 241,
            201, 233, 2, 58, 47, 90, 209, 207, 147, 204, 83, 68, 183, 143,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

        let anna_id =
            hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

        let anna_id =
            hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

        // A query getting elements by the identity desc

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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

        // A query getting 2 elements asc by the identity

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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
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
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should get root hash"),
            proof_root_hash
        );
        assert_eq!(result.0, proof_results);
    }

    #[cfg(feature = "server")]
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
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should get root hash"),
            proof_root_hash
        );
        assert_eq!(result.0, proof_results);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_dpns_query_start_at_first_version() {
        let platform_version = PlatformVersion::first();
        // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
        let (drive, contract) = setup_dpns_tests_with_batches(10, None, 11456, platform_version);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            142, 246, 25, 166, 52, 184, 158, 102, 192, 111, 173, 255, 155, 125, 53, 233, 98, 241,
            201, 233, 2, 58, 47, 90, 209, 207, 147, 204, 83, 68, 183, 143,
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

        let anna_id =
            hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_dpns_query_start_at_latest_version() {
        let platform_version = PlatformVersion::latest();
        // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
        let (drive, contract) = setup_dpns_tests_with_batches(10, None, 11456, platform_version);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            235, 23, 161, 209, 153, 68, 160, 57, 151, 170, 19, 99, 64, 48, 5, 114, 233, 154, 77,
            65, 104, 102, 128, 181, 159, 124, 54, 108, 229, 88, 185, 134,
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

        let anna_id =
            hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_dpns_query_start_after() {
        let platform_version = PlatformVersion::latest();
        // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
        let (drive, contract) = setup_dpns_tests_with_batches(10, None, 11456, platform_version);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            235, 23, 161, 209, 153, 68, 160, 57, 151, 170, 19, 99, 64, 48, 5, 114, 233, 154, 77,
            65, 104, 102, 128, 181, 159, 124, 54, 108, 229, 88, 185, 134,
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

        let anna_id =
            hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_dpns_query_start_at_desc() {
        let platform_version = PlatformVersion::latest();
        // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
        let (drive, contract) = setup_dpns_tests_with_batches(10, None, 11456, platform_version);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            235, 23, 161, 209, 153, 68, 160, 57, 151, 170, 19, 99, 64, 48, 5, 114, 233, 154, 77,
            65, 104, 102, 128, 181, 159, 124, 54, 108, 229, 88, 185, 134,
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

        let anna_id =
            hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_dpns_query_start_after_desc() {
        let platform_version = PlatformVersion::latest();
        // The point of this test is to test the situation where we have a start at a certain value for the DPNS query.
        let (drive, contract) = setup_dpns_tests_with_batches(10, None, 11456, platform_version);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            235, 23, 161, 209, 153, 68, 160, 57, 151, 170, 19, 99, 64, 48, 5, 114, 233, 154, 77,
            65, 104, 102, 128, 181, 159, 124, 54, 108, 229, 88, 185, 134,
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

        let anna_id =
            hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
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
        let document0 = document_from_legacy_value(value0);

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
                None,
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
        let document1 =
            Document::from_cbor(document_cbor1.as_slice(), None, None, platform_version)
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
                None,
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
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            233, 90, 110, 8, 43, 137, 139, 242, 8, 152, 175, 246, 177, 73, 49, 137, 61, 142, 2, 49,
            158, 134, 13, 222, 60, 223, 139, 41, 66, 131, 135, 38,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
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
        let document0 =
            Document::from_cbor(document_cbor0.as_slice(), None, None, platform_version)
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
                None,
            )
            .expect("document should be inserted");

        let domain1_id = Identifier::random_with_rng(&mut rng);

        assert!(domain0_id > domain1_id);

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
        let document1 =
            Document::from_cbor(document_cbor1.as_slice(), None, None, platform_version)
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
                None,
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
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            233, 90, 110, 8, 43, 137, 139, 242, 8, 152, 175, 246, 177, 73, 49, 137, 61, 142, 2, 49,
            158, 134, 13, 222, 60, 223, 139, 41, 66, 131, 135, 38,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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
                    .cloned()
                    .unwrap_or(Value::Null);
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

    #[cfg(feature = "server")]
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
        let document0 =
            Document::from_cbor(document_cbor0.as_slice(), None, None, platform_version)
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
                None,
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
        let document1 =
            Document::from_cbor(document_cbor1.as_slice(), None, None, platform_version)
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
                None,
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
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            233, 90, 110, 8, 43, 137, 139, 242, 8, 152, 175, 246, 177, 73, 49, 137, 61, 142, 2, 49,
            158, 134, 13, 222, 60, 223, 139, 41, 66, 131, 135, 38,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

        let anna_id =
            hex::decode("0e97eb86ceca4309751616089336a127a5d48282712473b2d0fc5663afb1a080")
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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_withdrawals_query_by_owner_id() {
        // We create 10 withdrawals owned by 2 identities
        let (drive, contract) = setup_withdrawal_tests(10, Some(2), 11456);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            237, 198, 157, 236, 20, 182, 87, 85, 216, 64, 84, 25, 163, 231, 107, 173, 155, 152, 34,
            64, 34, 142, 234, 16, 99, 134, 153, 156, 24, 208, 150, 115,
        ];

        assert_eq!(root_hash.as_slice(), expected_app_hash);

        // Document Ids are
        // document v0 : id:2kTB6gW4wCCnySj3UFUJQM3aUYBd6qDfLCY74BnWmFKu owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:09 updated_at:2024-11-21 12:31:09 amount:(i64)646767 coreFeePerByte:(i64)0 outputScript:bytes 00952c808390e575c8dd29fc07ccfed7b428e1ec2ffcb23e pooling:(i64)0 status:(i64)1 transactionIndex:(i64)4 transactionSignHeight:(i64)303186
        // document v0 : id:3T4aKmidGKA4ETnWYSedm6ETzrcdkfPL2r3D6eg6CSib owner_id:CH1EHBkN5FUuQ7z8ep1abroLPzzYjagvM5XV2NYR3DEh created_at:2024-11-21 12:31:01 updated_at:2024-11-21 12:31:01 amount:(i64)971045 coreFeePerByte:(i64)0 outputScript:bytes 525dfc160c160a7a52ef3301a7e55fccf41d73857f50a55a4d pooling:(i64)0 status:(i64)1 transactionIndex:(i64)2 transactionSignHeight:(i64)248787
        // document v0 : id:3X2QfUfR8EeVZQAKmEjcue5xDv3CZXrfPTgXkZ5vQo13 owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:11 updated_at:2024-11-21 12:31:11 amount:(i64)122155 coreFeePerByte:(i64)0 outputScript:bytes f76eb8b953ff41040d906c25a4ae42884bedb41a07fc3a pooling:(i64)0 status:(i64)3 transactionIndex:(i64)7 transactionSignHeight:(i64)310881
        // document v0 : id:5ikeRNwvFekr6ex32B4dLEcCaSsgXXHJBx5rJ2rwuhEV owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:30:59 updated_at:2024-11-21 12:30:59 amount:(i64)725014 coreFeePerByte:(i64)0 outputScript:bytes 51f203a755a7ff25ba8645841f80403ee98134690b2c0dd5e2 pooling:(i64)0 status:(i64)3 transactionIndex:(i64)1 transactionSignHeight:(i64)4072
        // document v0 : id:74giZJn9fNczYRsxxh3wVnktJS1vzTiRWYinKK1rRcyj owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:11 updated_at:2024-11-21 12:31:11 amount:(i64)151943 coreFeePerByte:(i64)0 outputScript:bytes 9db03f4c8a51e4e9855e008aae6121911b4831699c53ed pooling:(i64)0 status:(i64)1 transactionIndex:(i64)5 transactionSignHeight:(i64)343099
        // document v0 : id:8iqDAFxTzHYcmUWtcNnCRoj9Fss4HE1G3GP3HhVAZJhn owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:13 updated_at:2024-11-21 12:31:13 amount:(i64)409642 coreFeePerByte:(i64)0 outputScript:bytes 19fe0a2458a47e1726191f4dc94d11bcfacf821d024043 pooling:(i64)0 status:(i64)4 transactionIndex:(i64)8 transactionSignHeight:(i64)304397
        // document v0 : id:BdH274iP17nhquQVY4KMCAM6nwyPRc8AFJkUT91vxhbc owner_id:CH1EHBkN5FUuQ7z8ep1abroLPzzYjagvM5XV2NYR3DEh created_at:2024-11-21 12:31:03 updated_at:2024-11-21 12:31:03 amount:(i64)81005 coreFeePerByte:(i64)0 outputScript:bytes 2666e87b6cc7ddf2b63e7e52c348818c05e5562efa48f5 pooling:(i64)0 status:(i64)0
        // document v0 : id:CCjaU67Pe79Vt51oXvQ5SkyNiypofNX9DS9PYydN9tpD owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:01 updated_at:2024-11-21 12:31:01 amount:(i64)455074 coreFeePerByte:(i64)0 outputScript:bytes acde2e1652771b50a2c68fd330ee1d4b8e115631ce72375432 pooling:(i64)0 status:(i64)3 transactionIndex:(i64)3 transactionSignHeight:(i64)261103
        // document v0 : id:DxFzXvkb2mNQHmeVknsv3gWsc6rMtLk9AsS5zMpy6hou owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:05 updated_at:2024-11-21 12:31:05 amount:(i64)271303 coreFeePerByte:(i64)0 outputScript:bytes 0b845e8c3a4679f1913172f7fd939cc153f458519de8ed3d pooling:(i64)0 status:(i64)0
        // document v0 : id:FDnvFN7e72LcZEojTWNmJTP7uzok3BtvbKnaa5gjqCpW owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:11 updated_at:2024-11-21 12:31:11 amount:(i64)123433 coreFeePerByte:(i64)0 outputScript:bytes 82712473b2d0fc5663afb1a08006913ccccbf38e091a8cc7 pooling:(i64)0 status:(i64)4 transactionIndex:(i64)6 transactionSignHeight:(i64)319518

        let query_value = json!({
            "where": [
                ["$ownerId", "==", "A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ"]
            ],
            "limit": 2
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let domain_document_type = contract
            .document_type_for_name("withdrawal")
            .expect("contract should have a domain document type");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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
                document.id().to_string(Encoding::Base58)
            })
            .collect();

        let a_names = [
            "5ikeRNwvFekr6ex32B4dLEcCaSsgXXHJBx5rJ2rwuhEV".to_string(),
            "CCjaU67Pe79Vt51oXvQ5SkyNiypofNX9DS9PYydN9tpD".to_string(),
        ];

        assert_eq!(names, a_names);

        let (proof_root_hash, proof_results, _) = query
            .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
            .expect("we should be able to a proof");
        assert_eq!(root_hash, proof_root_hash);
        assert_eq!(results, proof_results);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_withdrawals_query_start_after_query_by_owner_id() {
        // We create 10 withdrawals owned by 2 identities
        let (drive, contract) = setup_withdrawal_tests(10, Some(2), 11456);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            237, 198, 157, 236, 20, 182, 87, 85, 216, 64, 84, 25, 163, 231, 107, 173, 155, 152, 34,
            64, 34, 142, 234, 16, 99, 134, 153, 156, 24, 208, 150, 115,
        ];

        assert_eq!(root_hash.as_slice(), expected_app_hash);

        // Document Ids are
        // document v0 : id:2kTB6gW4wCCnySj3UFUJQM3aUYBd6qDfLCY74BnWmFKu owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:09 updated_at:2024-11-21 12:31:09 amount:(i64)646767 coreFeePerByte:(i64)0 outputScript:bytes 00952c808390e575c8dd29fc07ccfed7b428e1ec2ffcb23e pooling:(i64)0 status:(i64)1 transactionIndex:(i64)4 transactionSignHeight:(i64)303186
        // document v0 : id:3T4aKmidGKA4ETnWYSedm6ETzrcdkfPL2r3D6eg6CSib owner_id:CH1EHBkN5FUuQ7z8ep1abroLPzzYjagvM5XV2NYR3DEh created_at:2024-11-21 12:31:01 updated_at:2024-11-21 12:31:01 amount:(i64)971045 coreFeePerByte:(i64)0 outputScript:bytes 525dfc160c160a7a52ef3301a7e55fccf41d73857f50a55a4d pooling:(i64)0 status:(i64)1 transactionIndex:(i64)2 transactionSignHeight:(i64)248787
        // document v0 : id:3X2QfUfR8EeVZQAKmEjcue5xDv3CZXrfPTgXkZ5vQo13 owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:11 updated_at:2024-11-21 12:31:11 amount:(i64)122155 coreFeePerByte:(i64)0 outputScript:bytes f76eb8b953ff41040d906c25a4ae42884bedb41a07fc3a pooling:(i64)0 status:(i64)3 transactionIndex:(i64)7 transactionSignHeight:(i64)310881
        // document v0 : id:5ikeRNwvFekr6ex32B4dLEcCaSsgXXHJBx5rJ2rwuhEV owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:30:59 updated_at:2024-11-21 12:30:59 amount:(i64)725014 coreFeePerByte:(i64)0 outputScript:bytes 51f203a755a7ff25ba8645841f80403ee98134690b2c0dd5e2 pooling:(i64)0 status:(i64)3 transactionIndex:(i64)1 transactionSignHeight:(i64)4072
        // document v0 : id:74giZJn9fNczYRsxxh3wVnktJS1vzTiRWYinKK1rRcyj owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:11 updated_at:2024-11-21 12:31:11 amount:(i64)151943 coreFeePerByte:(i64)0 outputScript:bytes 9db03f4c8a51e4e9855e008aae6121911b4831699c53ed pooling:(i64)0 status:(i64)1 transactionIndex:(i64)5 transactionSignHeight:(i64)343099
        // document v0 : id:8iqDAFxTzHYcmUWtcNnCRoj9Fss4HE1G3GP3HhVAZJhn owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:13 updated_at:2024-11-21 12:31:13 amount:(i64)409642 coreFeePerByte:(i64)0 outputScript:bytes 19fe0a2458a47e1726191f4dc94d11bcfacf821d024043 pooling:(i64)0 status:(i64)4 transactionIndex:(i64)8 transactionSignHeight:(i64)304397
        // document v0 : id:BdH274iP17nhquQVY4KMCAM6nwyPRc8AFJkUT91vxhbc owner_id:CH1EHBkN5FUuQ7z8ep1abroLPzzYjagvM5XV2NYR3DEh created_at:2024-11-21 12:31:03 updated_at:2024-11-21 12:31:03 amount:(i64)81005 coreFeePerByte:(i64)0 outputScript:bytes 2666e87b6cc7ddf2b63e7e52c348818c05e5562efa48f5 pooling:(i64)0 status:(i64)0
        // document v0 : id:CCjaU67Pe79Vt51oXvQ5SkyNiypofNX9DS9PYydN9tpD owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:01 updated_at:2024-11-21 12:31:01 amount:(i64)455074 coreFeePerByte:(i64)0 outputScript:bytes acde2e1652771b50a2c68fd330ee1d4b8e115631ce72375432 pooling:(i64)0 status:(i64)3 transactionIndex:(i64)3 transactionSignHeight:(i64)261103
        // document v0 : id:DxFzXvkb2mNQHmeVknsv3gWsc6rMtLk9AsS5zMpy6hou owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:05 updated_at:2024-11-21 12:31:05 amount:(i64)271303 coreFeePerByte:(i64)0 outputScript:bytes 0b845e8c3a4679f1913172f7fd939cc153f458519de8ed3d pooling:(i64)0 status:(i64)0
        // document v0 : id:FDnvFN7e72LcZEojTWNmJTP7uzok3BtvbKnaa5gjqCpW owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:11 updated_at:2024-11-21 12:31:11 amount:(i64)123433 coreFeePerByte:(i64)0 outputScript:bytes 82712473b2d0fc5663afb1a08006913ccccbf38e091a8cc7 pooling:(i64)0 status:(i64)4 transactionIndex:(i64)6 transactionSignHeight:(i64)319518

        let query_value = json!({
            "where": [
                ["$ownerId", "==", "A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ"]
            ],
            "startAfter":  "CCjaU67Pe79Vt51oXvQ5SkyNiypofNX9DS9PYydN9tpD",
            "limit": 3,
        });

        // This will use the identity recent index
        // {
        //     "name": "identityRecent",
        //     "properties": [
        //     {
        //         "$ownerId": "asc"
        //     },
        //     {
        //         "$updatedAt": "asc"
        //     },
        //     {
        //         "status": "asc"
        //     }
        //     ],
        //     "unique": false
        // },

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let domain_document_type = contract
            .document_type_for_name("withdrawal")
            .expect("contract should have a domain document type");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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
                document.id().to_string(Encoding::Base58)
            })
            .collect();

        // We only get back 2 values, even though we put limit 3 because the time with status 0 is an
        // empty tree and consumes a limit
        let a_names = [
            "DxFzXvkb2mNQHmeVknsv3gWsc6rMtLk9AsS5zMpy6hou".to_string(),
            "2kTB6gW4wCCnySj3UFUJQM3aUYBd6qDfLCY74BnWmFKu".to_string(),
        ];

        assert_eq!(names, a_names);

        let (proof_root_hash, proof_results, _) = query
            .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
            .expect("we should be able to a proof");
        assert_eq!(root_hash, proof_root_hash);
        assert_eq!(results, proof_results);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_withdrawals_query_start_after_query_by_owner_id_desc() {
        // We create 10 withdrawals owned by 2 identities
        let (drive, contract) = setup_withdrawal_tests(10, Some(2), 11456);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        let expected_app_hash = vec![
            237, 198, 157, 236, 20, 182, 87, 85, 216, 64, 84, 25, 163, 231, 107, 173, 155, 152, 34,
            64, 34, 142, 234, 16, 99, 134, 153, 156, 24, 208, 150, 115,
        ];

        assert_eq!(root_hash.as_slice(), expected_app_hash);

        // Document Ids are
        // document v0 : id:2kTB6gW4wCCnySj3UFUJQM3aUYBd6qDfLCY74BnWmFKu owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:09 updated_at:2024-11-21 12:31:09 amount:(i64)646767 coreFeePerByte:(i64)0 outputScript:bytes 00952c808390e575c8dd29fc07ccfed7b428e1ec2ffcb23e pooling:(i64)0 status:(i64)1 transactionIndex:(i64)4 transactionSignHeight:(i64)303186
        // document v0 : id:3T4aKmidGKA4ETnWYSedm6ETzrcdkfPL2r3D6eg6CSib owner_id:CH1EHBkN5FUuQ7z8ep1abroLPzzYjagvM5XV2NYR3DEh created_at:2024-11-21 12:31:01 updated_at:2024-11-21 12:31:01 amount:(i64)971045 coreFeePerByte:(i64)0 outputScript:bytes 525dfc160c160a7a52ef3301a7e55fccf41d73857f50a55a4d pooling:(i64)0 status:(i64)1 transactionIndex:(i64)2 transactionSignHeight:(i64)248787
        // document v0 : id:3X2QfUfR8EeVZQAKmEjcue5xDv3CZXrfPTgXkZ5vQo13 owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:11 updated_at:2024-11-21 12:31:11 amount:(i64)122155 coreFeePerByte:(i64)0 outputScript:bytes f76eb8b953ff41040d906c25a4ae42884bedb41a07fc3a pooling:(i64)0 status:(i64)3 transactionIndex:(i64)7 transactionSignHeight:(i64)310881
        // document v0 : id:5ikeRNwvFekr6ex32B4dLEcCaSsgXXHJBx5rJ2rwuhEV owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:30:59 updated_at:2024-11-21 12:30:59 amount:(i64)725014 coreFeePerByte:(i64)0 outputScript:bytes 51f203a755a7ff25ba8645841f80403ee98134690b2c0dd5e2 pooling:(i64)0 status:(i64)3 transactionIndex:(i64)1 transactionSignHeight:(i64)4072
        // document v0 : id:74giZJn9fNczYRsxxh3wVnktJS1vzTiRWYinKK1rRcyj owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:11 updated_at:2024-11-21 12:31:11 amount:(i64)151943 coreFeePerByte:(i64)0 outputScript:bytes 9db03f4c8a51e4e9855e008aae6121911b4831699c53ed pooling:(i64)0 status:(i64)1 transactionIndex:(i64)5 transactionSignHeight:(i64)343099
        // document v0 : id:8iqDAFxTzHYcmUWtcNnCRoj9Fss4HE1G3GP3HhVAZJhn owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:13 updated_at:2024-11-21 12:31:13 amount:(i64)409642 coreFeePerByte:(i64)0 outputScript:bytes 19fe0a2458a47e1726191f4dc94d11bcfacf821d024043 pooling:(i64)0 status:(i64)4 transactionIndex:(i64)8 transactionSignHeight:(i64)304397
        // document v0 : id:BdH274iP17nhquQVY4KMCAM6nwyPRc8AFJkUT91vxhbc owner_id:CH1EHBkN5FUuQ7z8ep1abroLPzzYjagvM5XV2NYR3DEh created_at:2024-11-21 12:31:03 updated_at:2024-11-21 12:31:03 amount:(i64)81005 coreFeePerByte:(i64)0 outputScript:bytes 2666e87b6cc7ddf2b63e7e52c348818c05e5562efa48f5 pooling:(i64)0 status:(i64)0
        // document v0 : id:CCjaU67Pe79Vt51oXvQ5SkyNiypofNX9DS9PYydN9tpD owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:01 updated_at:2024-11-21 12:31:01 amount:(i64)455074 coreFeePerByte:(i64)0 outputScript:bytes acde2e1652771b50a2c68fd330ee1d4b8e115631ce72375432 pooling:(i64)0 status:(i64)3 transactionIndex:(i64)3 transactionSignHeight:(i64)261103
        // document v0 : id:DxFzXvkb2mNQHmeVknsv3gWsc6rMtLk9AsS5zMpy6hou owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:05 updated_at:2024-11-21 12:31:05 amount:(i64)271303 coreFeePerByte:(i64)0 outputScript:bytes 0b845e8c3a4679f1913172f7fd939cc153f458519de8ed3d pooling:(i64)0 status:(i64)0
        // document v0 : id:FDnvFN7e72LcZEojTWNmJTP7uzok3BtvbKnaa5gjqCpW owner_id:A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ created_at:2024-11-21 12:31:11 updated_at:2024-11-21 12:31:11 amount:(i64)123433 coreFeePerByte:(i64)0 outputScript:bytes 82712473b2d0fc5663afb1a08006913ccccbf38e091a8cc7 pooling:(i64)0 status:(i64)4 transactionIndex:(i64)6 transactionSignHeight:(i64)319518

        let query_value = json!({
            "where": [
                ["$ownerId", "==", "A8GdKdMT7eDvtjnmMXe1Z3YaTtJzZdxNDRkeLb8goFrZ"]
            ],
            "startAfter":  "2kTB6gW4wCCnySj3UFUJQM3aUYBd6qDfLCY74BnWmFKu",
            "limit": 3,
            "orderBy": [
                ["$updatedAt", "desc"]
            ]
        });

        // This will use the identity recent index
        // {
        //     "name": "identityRecent",
        //     "properties": [
        //     {
        //         "$ownerId": "asc"
        //     },
        //     {
        //         "$updatedAt": "asc"
        //     },
        //     {
        //         "status": "asc"
        //     }
        //     ],
        //     "unique": false
        // },

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let domain_document_type = contract
            .document_type_for_name("withdrawal")
            .expect("contract should have a domain document type");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            domain_document_type,
            &drive.config,
            platform_version,
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
                document.id().to_string(Encoding::Base58)
            })
            .collect();

        // We only get back 2 values, even though we put limit 3 because the time with status 0 is an
        // empty tree and consumes a limit
        let a_names = [
            "DxFzXvkb2mNQHmeVknsv3gWsc6rMtLk9AsS5zMpy6hou".to_string(),
            "CCjaU67Pe79Vt51oXvQ5SkyNiypofNX9DS9PYydN9tpD".to_string(),
        ];

        assert_eq!(names, a_names);

        let (proof_root_hash, proof_results, _) = query
            .execute_with_proof_only_get_elements(&drive, None, None, platform_version)
            .expect("we should be able to a proof");
        assert_eq!(root_hash, proof_root_hash);
        assert_eq!(results, proof_results);
    }

    /// Drive-level mirror of the SDK reproducer in
    /// `packages/rs-sdk/tests/fetch/withdrawals_orderby.rs` (issue #2409).
    ///
    /// Populates **15** withdrawals (each with a unique owner) into a fresh
    /// in-memory Drive, then runs the same query twice — `orderBy $ownerId asc`
    /// and `orderBy $ownerId desc` — with `limit = 10` each. Because 15 > 10,
    /// correct behavior is:
    ///
    /// * each direction returns exactly 10 docs,
    /// * their **union** covers all 15 inserted withdrawals,
    /// * their **intersection** has exactly `10 + 10 − 15 = 5` docs (the
    ///   "middle" five by ownerId),
    /// * `asc` is returned in non-decreasing ownerId order and `desc` in
    ///   non-increasing ownerId order.
    ///
    /// Any deviation — asc and desc returning the same set, empty results,
    /// wrong ordering — indicates the Drive-level orderBy asymmetry that the
    /// SDK test reproduces against mainnet.
    #[cfg(feature = "server")]
    #[test]
    fn test_withdrawals_query_orderby_asc_vs_desc_owner_id_limit_10_of_15() {
        use std::collections::BTreeSet;

        // 15 withdrawals, each with a unique owner (total_owners = None).
        let (drive, contract) = setup_withdrawal_tests(15, None, 11456);

        let platform_version = PlatformVersion::latest();
        let db_transaction = drive.grove.start_transaction();

        let withdrawal_document_type = contract
            .document_type_for_name("withdrawal")
            .expect("contract should have a withdrawal document type");

        // Runs the orderBy query and returns (ordered doc-ids, ordered owner-ids).
        let run = |direction: &str, limit: u32| -> (Vec<String>, Vec<Identifier>) {
            let query_value = json!({
                "where": [],
                "limit": limit,
                "orderBy": [["$ownerId", direction]],
            });
            let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
                .expect("expected to serialize to cbor");
            let query = DriveDocumentQuery::from_cbor(
                where_cbor.as_slice(),
                &contract,
                withdrawal_document_type,
                &drive.config,
                platform_version,
            )
            .expect("query should be built");
            let (results, _, _) = query
                .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
                .expect("query should execute");
            let docs: Vec<Document> = results
                .iter()
                .map(|bytes| {
                    Document::from_bytes(
                        bytes.as_slice(),
                        withdrawal_document_type,
                        platform_version,
                    )
                    .expect("should deserialize withdrawal document")
                })
                .collect();
            let ids = docs
                .iter()
                .map(|d| d.id().to_string(Encoding::Base58))
                .collect();
            let owners = docs.iter().map(|d| d.owner_id()).collect();
            (ids, owners)
        };

        // First, fetch all 15 inserted withdrawals sorted ascending by ownerId
        // so the full dataset is visible before the two limit=10 queries run.
        let (all_ids, all_owners) = run("asc", 15);
        let short = |id: &Identifier| hex::encode(&id.as_bytes()[..4]);
        let all_owner_prefixes: Vec<String> = all_owners.iter().map(short).collect();
        println!("all (asc, limit=15) count       = {}", all_ids.len());
        println!("all (asc, limit=15) ids         = {:?}", all_ids);
        println!("all (asc, limit=15) owner[0..4] = {:?}", all_owner_prefixes);

        let (asc_ids, asc_owners) = run("asc", 10);
        let (desc_ids, desc_owners) = run("desc", 10);

        // Dump both limit=10 result sets up front so any subsequent assertion
        // failure includes a side-by-side view of what each direction returned.
        let asc_owner_prefixes: Vec<String> = asc_owners.iter().map(short).collect();
        let desc_owner_prefixes: Vec<String> = desc_owners.iter().map(short).collect();
        println!("asc  ids   = {:?}", asc_ids);
        println!("desc ids   = {:?}", desc_ids);
        println!("asc  owner[0..4] = {:?}", asc_owner_prefixes);
        println!("desc owner[0..4] = {:?}", desc_owner_prefixes);

        assert_eq!(
            asc_ids.len(),
            10,
            "asc should return limit=10 documents, got {} (ids={:?})",
            asc_ids.len(),
            asc_ids,
        );
        assert_eq!(
            desc_ids.len(),
            10,
            "desc should return limit=10 documents, got {} (ids={:?})",
            desc_ids.len(),
            desc_ids,
        );

        // Monotonicity: asc must be non-decreasing by ownerId,
        // desc must be non-increasing by ownerId.
        for window in asc_owners.windows(2) {
            assert!(
                window[0] <= window[1],
                "asc result not sorted ascending by ownerId: {:?} then {:?}\nfull asc owners: {:?}",
                window[0],
                window[1],
                asc_owners,
            );
        }
        for window in desc_owners.windows(2) {
            assert!(
                window[0] >= window[1],
                "desc result not sorted descending by ownerId: {:?} then {:?}\nfull desc owners: {:?}",
                window[0],
                window[1],
                desc_owners,
            );
        }

        let asc_set: BTreeSet<_> = asc_ids.iter().cloned().collect();
        let desc_set: BTreeSet<_> = desc_ids.iter().cloned().collect();
        let union: BTreeSet<_> = asc_set.union(&desc_set).cloned().collect();
        let intersection: BTreeSet<_> = asc_set.intersection(&desc_set).cloned().collect();
        let only_asc: Vec<_> = asc_set.difference(&desc_set).cloned().collect();
        let only_desc: Vec<_> = desc_set.difference(&asc_set).cloned().collect();

        assert_eq!(
            union.len(),
            15,
            "union(asc, desc) must cover all 15 inserted withdrawals, got {}\n\
             asc_ids={:?}\ndesc_ids={:?}\nonly_in_asc={:?}\nonly_in_desc={:?}",
            union.len(),
            asc_ids,
            desc_ids,
            only_asc,
            only_desc,
        );
        assert_eq!(
            intersection.len(),
            5,
            "asc ∩ desc must contain exactly 5 withdrawals (the middle by ownerId), got {}\n\
             asc_ids={:?}\ndesc_ids={:?}\nonly_in_asc={:?}\nonly_in_desc={:?}",
            intersection.len(),
            asc_ids,
            desc_ids,
            only_asc,
            only_desc,
        );

        // Reversing the desc result should NOT equal the asc result — they are
        // different halves of the 15-doc set.
        let mut desc_rev = desc_ids.clone();
        desc_rev.reverse();
        assert_ne!(
            desc_rev, asc_ids,
            "asc and reverse(desc) should differ when total docs (15) > limit (10);\n\
             asc_ids={:?}\nreverse(desc_ids)={:?}",
            asc_ids, desc_rev,
        );
    }

    /// Drive-level reproducer for the *range + `in`* half of
    /// [issue #2409](https://github.com/dashpay/platform/issues/2409):
    ///
    /// ```text
    /// where:   [['transactionIndex', 'in', [0,1,2,3,4,5]], ['status', '>', 0]]
    /// orderBy: [['status', <dir>], ['transactionIndex', <dir>]]
    /// ```
    ///
    /// On mainnet this query returns `[]` in both directions. This test inserts
    /// a deterministic 10-withdrawal dataset with a known distribution of
    /// statuses and transaction indices (seed 11456), then runs the exact query
    /// twice (asc + desc). Expected matches (status > 0 AND transactionIndex in
    /// [0..=5]) are, per the existing dataset enumeration:
    ///
    /// * status=1, txIndex=2 — `3T4aKmidGKA4ETnWYSedm6ETzrcdkfPL2r3D6eg6CSib`
    /// * status=1, txIndex=4 — `2kTB6gW4wCCnySj3UFUJQM3aUYBd6qDfLCY74BnWmFKu`
    /// * status=1, txIndex=5 — `74giZJn9fNczYRsxxh3wVnktJS1vzTiRWYinKK1rRcyj`
    /// * status=3, txIndex=1 — `5ikeRNwvFekr6ex32B4dLEcCaSsgXXHJBx5rJ2rwuhEV`
    /// * status=3, txIndex=3 — `CCjaU67Pe79Vt51oXvQ5SkyNiypofNX9DS9PYydN9tpD`
    ///
    /// Under correct behavior the query must use the `transaction` secondary
    /// index ([status asc, transactionIndex asc]) and return all 5 matches in
    /// `[status, transactionIndex]`-sorted order for asc, and the exact reverse
    /// for desc. Asserts this.
    #[cfg(feature = "server")]
    #[test]
    fn test_withdrawals_query_range_plus_in_issue_2409() {
        let (drive, contract) = setup_withdrawal_tests(10, Some(2), 11456);

        let platform_version = PlatformVersion::latest();
        let db_transaction = drive.grove.start_transaction();

        let withdrawal_document_type = contract
            .document_type_for_name("withdrawal")
            .expect("contract should have a withdrawal document type");

        let run = |direction: &str| -> Vec<(String, i64, i64)> {
            let query_value = json!({
                "where": [
                    ["transactionIndex", "in", [0, 1, 2, 3, 4, 5]],
                    ["status", ">", 0],
                ],
                "limit": 100,
                "orderBy": [
                    ["status", direction],
                    ["transactionIndex", direction],
                ],
            });
            let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
                .expect("expected to serialize to cbor");
            let query = DriveDocumentQuery::from_cbor(
                where_cbor.as_slice(),
                &contract,
                withdrawal_document_type,
                &drive.config,
                platform_version,
            )
            .expect("query should be built");
            let (results, _, _) = query
                .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
                .expect("query should execute");
            results
                .iter()
                .map(|bytes| {
                    let doc = Document::from_bytes(
                        bytes.as_slice(),
                        withdrawal_document_type,
                        platform_version,
                    )
                    .expect("should deserialize withdrawal document");
                    let status = doc
                        .get("status")
                        .expect("withdrawal has status")
                        .to_integer::<i64>()
                        .expect("status is an integer");
                    let tx_index = doc
                        .get("transactionIndex")
                        .expect("filtered docs have transactionIndex")
                        .to_integer::<i64>()
                        .expect("transactionIndex is an integer");
                    (doc.id().to_string(Encoding::Base58), status, tx_index)
                })
                .collect()
        };

        let asc = run("asc");
        let desc = run("desc");

        println!("asc  ({} docs) = {:?}", asc.len(), asc);
        println!("desc ({} docs) = {:?}", desc.len(), desc);

        // 1. Empty-array bug regression: each direction must return the 5
        //    matching docs.
        assert!(
            !asc.is_empty(),
            "asc returned no documents — reproduces the 'empty array' half of #2409 \
             (status > 0 AND transactionIndex in [0..5] should match 5 docs)",
        );
        assert!(
            !desc.is_empty(),
            "desc returned no documents — reproduces the 'empty array' half of #2409 \
             (status > 0 AND transactionIndex in [0..5] should match 5 docs)",
        );
        assert_eq!(asc.len(), 5, "asc should return 5 matches, got {:?}", asc);
        assert_eq!(
            desc.len(),
            5,
            "desc should return 5 matches, got {:?}",
            desc
        );

        // 2. All returned docs must satisfy the filters.
        for (id, status, tx_index) in asc.iter().chain(desc.iter()) {
            assert!(
                *status > 0,
                "returned doc {} violates status > 0 (status={})",
                id,
                status,
            );
            assert!(
                (0..=5).contains(tx_index),
                "returned doc {} violates transactionIndex in [0..=5] (tx_index={})",
                id,
                tx_index,
            );
        }

        // 3. Both queries must cover the same set of documents.
        let asc_ids: Vec<&String> = asc.iter().map(|(id, _, _)| id).collect();
        let desc_ids: Vec<&String> = desc.iter().map(|(id, _, _)| id).collect();
        let asc_set: std::collections::BTreeSet<_> = asc_ids.iter().copied().collect();
        let desc_set: std::collections::BTreeSet<_> = desc_ids.iter().copied().collect();
        assert_eq!(
            asc_set, desc_set,
            "asc and desc returned different document sets — reproduces the \
             asc/desc asymmetry half of #2409\nasc={:?}\ndesc={:?}",
            asc_ids, desc_ids,
        );

        // 4. Sort orders must be respected: asc non-decreasing in (status,
        //    transactionIndex); desc non-increasing.
        for w in asc.windows(2) {
            let a = (w[0].1, w[0].2);
            let b = (w[1].1, w[1].2);
            assert!(
                a <= b,
                "asc not sorted by (status, transactionIndex) ascending: {:?} then {:?}\nfull asc={:?}",
                a, b, asc,
            );
        }
        for w in desc.windows(2) {
            let a = (w[0].1, w[0].2);
            let b = (w[1].1, w[1].2);
            assert!(
                a >= b,
                "desc not sorted by (status, transactionIndex) descending: {:?} then {:?}\nfull desc={:?}",
                a, b, desc,
            );
        }

        // 5. Perfect mirror: reverse(asc) must equal desc.
        let mut asc_rev = asc.clone();
        asc_rev.reverse();
        assert_eq!(
            asc_rev, desc,
            "reverse(asc) must equal desc for this single-index query\nreverse(asc)={:?}\ndesc={:?}",
            asc_rev, desc,
        );

        // 6. Exact expected document IDs (pin the dataset so a regression in
        //    the withdrawal generator or index structure is visible).
        let expected_asc_ids = [
            "3T4aKmidGKA4ETnWYSedm6ETzrcdkfPL2r3D6eg6CSib", // status=1 ti=2
            "2kTB6gW4wCCnySj3UFUJQM3aUYBd6qDfLCY74BnWmFKu", // status=1 ti=4
            "74giZJn9fNczYRsxxh3wVnktJS1vzTiRWYinKK1rRcyj", // status=1 ti=5
            "5ikeRNwvFekr6ex32B4dLEcCaSsgXXHJBx5rJ2rwuhEV", // status=3 ti=1
            "CCjaU67Pe79Vt51oXvQ5SkyNiypofNX9DS9PYydN9tpD", // status=3 ti=3
        ];
        assert_eq!(
            asc_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            expected_asc_ids,
            "asc ids must match the deterministic expected order for seed 11456",
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_query_a_b_c_d_e_contract() {
        let drive: Drive = setup_drive_with_initial_state_structure(None);

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

        let factory = DataContractFactory::new(platform_version.protocol_version)
            .expect("should create factory");

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

    #[cfg(feature = "server")]
    #[test]
    fn test_query_documents_by_created_at() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let contract_value = platform_value!({
            "$formatVersion": "0",
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

        let query = DriveDocumentQuery::from_cbor(
            &query_bytes,
            &contract,
            document_type,
            &DriveConfig::default(),
            platform_version,
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

    #[cfg(feature = "server")]
    #[test]
    fn test_count_regular_index() {
        let platform_version = PlatformVersion::latest();

        let (drive, contract) = setup_countable_family_tests(6, 15, platform_version);

        let db_transaction = drive.grove.start_transaction();

        let _root_hash = drive
            .grove
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");

        // A query getting all elements by age

        let query_value = platform_value!({
            "where": [
                ["age", ">=", 1]
            ],
            "orderBy": [
                ["age", "asc"]
            ]
        });

        let person_document_type = contract
            .document_type_for_name("person")
            .expect("contract should have a person document type");

        let query = DriveDocumentQuery::from_value(
            query_value,
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("query should be built");

        let (proof, _) = query
            .execute_with_proof(&drive, None, None, platform_version)
            .expect("we should be able to a proof");

        assert!(!proof.is_empty(), "proof should not be empty");
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod multi_in_tests {
    //! Multiple `In` clauses on consecutive compound-index properties
    //! (protocol version 14+). Each positive case cross-checks the
    //! index-driven results against a brute-force filter over every
    //! stored document, and round-trips the proof against the live
    //! root hash.

    use super::*;

    /// All people as (serialized bytes, document) pairs.
    fn all_people(
        drive: &Drive,
        contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Vec<(Vec<u8>, Document)> {
        let person_document_type = contract
            .document_type_for_name("person")
            .expect("contract should have a person document type");
        let query_value = json!({ "limit": 100 });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("query should be built");
        let (results, _, _) = query
            .execute_raw_results_no_proof(drive, None, None, platform_version)
            .expect("expected to fetch all people");
        results
            .into_iter()
            .map(|bytes| {
                let document =
                    Document::from_bytes(bytes.as_slice(), person_document_type, platform_version)
                        .expect("document should deserialize");
                (bytes, document)
            })
            .collect()
    }

    fn text_field(document: &Document, field: &str) -> String {
        document
            .get(field)
            .expect("field should exist")
            .as_text()
            .expect("field should be text")
            .to_string()
    }

    /// Run `query_value` both without proof and with proof, assert the
    /// proof verifies against the live root hash and returns identical
    /// results, and return the deserialized documents.
    fn run_query_with_proof_round_trip(
        drive: &Drive,
        contract: &DataContract,
        query_value: serde_json::Value,
        platform_version: &PlatformVersion,
    ) -> Vec<Document> {
        let person_document_type = contract
            .document_type_for_name("person")
            .expect("contract should have a person document type");
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("query should be built");
        let (results, _, _) = query
            .execute_raw_results_no_proof(drive, None, None, platform_version)
            .expect("query should execute");

        let root_hash = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("there is always a root hash");
        let (proof_root_hash, proof_results, _) = query
            .execute_with_proof_only_get_elements(drive, None, None, platform_version)
            .expect("proof should be generated and verified");
        assert_eq!(root_hash, proof_root_hash);
        assert_eq!(results, proof_results);

        results
            .into_iter()
            .map(|bytes| {
                Document::from_bytes(bytes.as_slice(), person_document_type, platform_version)
                    .expect("document should deserialize")
            })
            .collect()
    }

    #[test]
    fn test_two_in_clauses_on_compound_index() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);
        let people = all_people(&drive, &contract, platform_version);

        // Pick in lists that partially overlap the stored data
        let first_names: Vec<String> = {
            let mut names: Vec<String> = people
                .iter()
                .map(|(_, document)| text_field(document, "firstName"))
                .collect();
            names.sort();
            names.dedup();
            names.into_iter().take(3).collect()
        };
        let last_names: Vec<String> = {
            let mut names: Vec<String> = people
                .iter()
                .map(|(_, document)| text_field(document, "lastName"))
                .collect();
            names.sort();
            names.dedup();
            names.into_iter().take(3).collect()
        };
        assert!(first_names.len() >= 2, "fixture should have enough names");
        assert!(last_names.len() >= 2, "fixture should have enough names");

        let query_value = json!({
            "where": [
                ["firstName", "in", first_names],
                ["lastName", "in", last_names],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ]
        });
        let documents =
            run_query_with_proof_round_trip(&drive, &contract, query_value, platform_version);

        // Brute-force expectation in index traversal order
        let mut expected: Vec<(String, String)> = people
            .iter()
            .filter_map(|(_, document)| {
                let first_name = text_field(document, "firstName");
                let last_name = text_field(document, "lastName");
                (first_names.contains(&first_name) && last_names.contains(&last_name))
                    .then_some((first_name, last_name))
            })
            .collect();
        expected.sort();
        assert!(!expected.is_empty(), "fixture should produce matches");

        let returned: Vec<(String, String)> = documents
            .iter()
            .map(|document| {
                (
                    text_field(document, "firstName"),
                    text_field(document, "lastName"),
                )
            })
            .collect();
        assert_eq!(returned, expected);
    }

    #[test]
    fn test_two_in_clauses_descending_first_level() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);
        let people = all_people(&drive, &contract, platform_version);

        let first_names: Vec<String> = {
            let mut names: Vec<String> = people
                .iter()
                .map(|(_, document)| text_field(document, "firstName"))
                .collect();
            names.sort();
            names.dedup();
            names
        };
        let last_names: Vec<String> = {
            let mut names: Vec<String> = people
                .iter()
                .map(|(_, document)| text_field(document, "lastName"))
                .collect();
            names.sort();
            names.dedup();
            names
        };

        let query_value = json!({
            "where": [
                ["firstName", "in", first_names],
                ["lastName", "in", last_names],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "desc"],
                ["lastName", "asc"],
            ]
        });
        let documents =
            run_query_with_proof_round_trip(&drive, &contract, query_value, platform_version);

        let mut expected: Vec<(String, String)> = people
            .iter()
            .map(|(_, document)| {
                (
                    text_field(document, "firstName"),
                    text_field(document, "lastName"),
                )
            })
            .collect();
        // firstName descending, lastName ascending within it
        expected.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

        let returned: Vec<(String, String)> = documents
            .iter()
            .map(|document| {
                (
                    text_field(document, "firstName"),
                    text_field(document, "lastName"),
                )
            })
            .collect();
        assert_eq!(returned, expected);
    }

    #[test]
    fn test_three_in_clauses_on_compound_index() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);
        let people = all_people(&drive, &contract, platform_version);

        let names_of = |field: &str| -> Vec<String> {
            let mut names: Vec<String> = people
                .iter()
                .map(|(_, document)| text_field(document, field))
                .collect();
            names.sort();
            names.dedup();
            names
        };
        // Keep the cross product under the 100-branch cap (3 x 3 x 10 = 90)
        // while guaranteeing matches: the shortened lists keep the first
        // document's values.
        let mut first_names = names_of("firstName");
        first_names.sort_by_key(|name| *name != text_field(&people[0].1, "firstName"));
        first_names.truncate(3);
        first_names.sort();
        let mut middle_names = names_of("middleName");
        middle_names.sort_by_key(|name| *name != text_field(&people[0].1, "middleName"));
        middle_names.truncate(3);
        middle_names.sort();
        let last_names = names_of("lastName");

        let query_value = json!({
            "where": [
                ["firstName", "in", first_names],
                ["middleName", "in", middle_names],
                ["lastName", "in", last_names],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["middleName", "asc"],
                ["lastName", "asc"],
            ]
        });
        let documents =
            run_query_with_proof_round_trip(&drive, &contract, query_value, platform_version);

        let mut expected: Vec<(String, String, String)> = people
            .iter()
            .filter_map(|(_, document)| {
                let first_name = text_field(document, "firstName");
                let middle_name = text_field(document, "middleName");
                (first_names.contains(&first_name) && middle_names.contains(&middle_name))
                    .then_some((first_name, middle_name, text_field(document, "lastName")))
            })
            .collect();
        expected.sort();
        assert!(!expected.is_empty(), "fixture should produce matches");

        let returned: Vec<(String, String, String)> = documents
            .iter()
            .map(|document| {
                (
                    text_field(document, "firstName"),
                    text_field(document, "middleName"),
                    text_field(document, "lastName"),
                )
            })
            .collect();
        assert_eq!(returned, expected);
    }

    #[test]
    fn test_two_in_clauses_with_trailing_range() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);
        let people = all_people(&drive, &contract, platform_version);

        let names_of = |field: &str| -> Vec<String> {
            let mut names: Vec<String> = people
                .iter()
                .map(|(_, document)| text_field(document, field))
                .collect();
            names.sort();
            names.dedup();
            names
        };
        let first_names = names_of("firstName");
        let middle_names = names_of("middleName");

        let query_value = json!({
            "where": [
                ["firstName", "in", first_names],
                ["middleName", "in", middle_names],
                ["lastName", ">", "M"],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["middleName", "asc"],
                ["lastName", "asc"],
            ]
        });
        let documents =
            run_query_with_proof_round_trip(&drive, &contract, query_value, platform_version);

        let mut expected: Vec<(String, String, String)> = people
            .iter()
            .filter_map(|(_, document)| {
                let last_name = text_field(document, "lastName");
                (last_name.as_str() > "M").then(|| {
                    (
                        text_field(document, "firstName"),
                        text_field(document, "middleName"),
                        last_name,
                    )
                })
            })
            .collect();
        expected.sort();
        assert!(!expected.is_empty(), "fixture should produce matches");

        let returned: Vec<(String, String, String)> = documents
            .iter()
            .map(|document| {
                (
                    text_field(document, "firstName"),
                    text_field(document, "middleName"),
                    text_field(document, "lastName"),
                )
            })
            .collect();
        assert_eq!(returned, expected);
    }

    #[test]
    fn test_two_in_clauses_with_descending_left_over_property() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);
        let people = all_people(&drive, &contract, platform_version);

        let names_of = |field: &str| -> Vec<String> {
            let mut names: Vec<String> = people
                .iter()
                .map(|(_, document)| text_field(document, field))
                .collect();
            names.sort();
            names.dedup();
            names
        };
        let first_names = names_of("firstName");
        let middle_names = names_of("middleName");

        // [firstName, middleName, lastName]: lastName is left over and
        // ordered descending
        let query_value = json!({
            "where": [
                ["firstName", "in", first_names],
                ["middleName", "in", middle_names],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["middleName", "asc"],
                ["lastName", "desc"],
            ]
        });
        let documents =
            run_query_with_proof_round_trip(&drive, &contract, query_value, platform_version);

        let mut expected: Vec<(String, String, std::cmp::Reverse<String>)> = people
            .iter()
            .map(|(_, document)| {
                (
                    text_field(document, "firstName"),
                    text_field(document, "middleName"),
                    std::cmp::Reverse(text_field(document, "lastName")),
                )
            })
            .collect();
        expected.sort();

        let returned: Vec<(String, String, std::cmp::Reverse<String>)> = documents
            .iter()
            .map(|document| {
                (
                    text_field(document, "firstName"),
                    text_field(document, "middleName"),
                    std::cmp::Reverse(text_field(document, "lastName")),
                )
            })
            .collect();
        assert_eq!(returned, expected);
        assert_eq!(returned.len(), 10);
    }

    #[test]
    fn test_equality_prefix_with_two_in_clauses() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);
        let people = all_people(&drive, &contract, platform_version);

        // index: [age, firstName, middleName, lastName]
        let age = people[0]
            .1
            .get("age")
            .expect("age should exist")
            .to_integer::<u8>()
            .expect("age should be an integer");
        let names_of = |field: &str| -> Vec<String> {
            let mut names: Vec<String> = people
                .iter()
                .map(|(_, document)| text_field(document, field))
                .collect();
            names.sort();
            names.dedup();
            names
        };
        let first_names = names_of("firstName");
        let middle_names = names_of("middleName");

        let query_value = json!({
            "where": [
                ["age", "==", age],
                ["firstName", "in", first_names],
                ["middleName", "in", middle_names],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["middleName", "asc"],
            ]
        });
        let documents =
            run_query_with_proof_round_trip(&drive, &contract, query_value, platform_version);

        let mut expected: Vec<(String, String)> = people
            .iter()
            .filter_map(|(_, document)| {
                let document_age = document
                    .get("age")
                    .expect("age should exist")
                    .to_integer::<u8>()
                    .expect("age should be an integer");
                (document_age == age).then(|| {
                    (
                        text_field(document, "firstName"),
                        text_field(document, "middleName"),
                    )
                })
            })
            .collect();
        expected.sort();
        assert!(!expected.is_empty(), "fixture should produce matches");

        let returned: Vec<(String, String)> = documents
            .iter()
            .map(|document| {
                (
                    text_field(document, "firstName"),
                    text_field(document, "middleName"),
                )
            })
            .collect();
        assert_eq!(returned, expected);
    }

    #[test]
    fn test_multiple_in_clauses_rejected_before_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);
        let platform_version_13 =
            PlatformVersion::get(13).expect("protocol version 13 should exist");

        let person_document_type = contract
            .document_type_for_name("person")
            .expect("contract should have a person document type");
        let query_value = json!({
            "where": [
                ["firstName", "in", ["Adey", "Briney"]],
                ["lastName", "in", ["Kriskov", "Randolf"]],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        // The grammar groups multiple in clauses structurally
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("query should be built");

        // ... but the v0 (protocol version <= 13) lowering rejects them
        let error = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version_13)
            .expect_err("multiple in clauses must be rejected before protocol version 14");
        assert!(
            matches!(error, Error::Query(QuerySyntaxError::MultipleInClauses(_))),
            "expected MultipleInClauses, got {error:?}"
        );

        let error = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version_13)
            .expect_err("multiple in clauses must be rejected on the proof path too");
        assert!(
            matches!(error, Error::Query(QuerySyntaxError::MultipleInClauses(_))),
            "expected MultipleInClauses, got {error:?}"
        );

        // ... and protocol version 14 accepts the very same query
        query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("the same query should execute at protocol version 14");
    }

    #[test]
    fn test_multiple_in_clauses_reject_cursor_pagination() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);
        let people = all_people(&drive, &contract, platform_version);
        let start_after = people[0].1.id().to_string(Encoding::Base58);

        let person_document_type = contract
            .document_type_for_name("person")
            .expect("contract should have a person document type");
        let query_value = json!({
            "where": [
                ["firstName", "in", ["Adey", "Briney"]],
                ["lastName", "in", ["Kriskov", "Randolf"]],
            ],
            "startAfter": start_after,
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("query should be built");
        let error = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect_err("cursor pagination with multiple in clauses must be rejected");
        assert!(
            matches!(error, Error::Query(QuerySyntaxError::Unsupported(_))),
            "expected Unsupported, got {error:?}"
        );
    }

    #[test]
    fn test_multiple_in_clauses_cursor_rejection_precedes_cursor_lookup() {
        // The shape preflight must fire before the startAfter document is
        // fetched from storage: a nonexistent cursor may not surface as
        // StartDocumentNotFound when the shape itself is unsupported (v14)
        // or rejected (v13).
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);
        let platform_version_13 =
            PlatformVersion::get(13).expect("protocol version 13 should exist");

        let person_document_type = contract
            .document_type_for_name("person")
            .expect("contract should have a person document type");
        let nonexistent = Identifier::from([77u8; 32]).to_string(Encoding::Base58);
        let query_value = json!({
            "where": [
                ["firstName", "in", ["Adey", "Briney"]],
                ["lastName", "in", ["Kriskov", "Randolf"]],
            ],
            "startAfter": nonexistent,
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("query should be built");

        let error = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect_err("multi-in with a cursor must be rejected at protocol version 14");
        assert!(
            matches!(error, Error::Query(QuerySyntaxError::Unsupported(_))),
            "expected Unsupported before the cursor lookup, got {error:?}"
        );

        let error = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version_13)
            .expect_err("multi-in must be rejected at protocol version 13");
        assert!(
            matches!(error, Error::Query(QuerySyntaxError::MultipleInClauses(_))),
            "expected MultipleInClauses before the cursor lookup, got {error:?}"
        );
    }

    #[test]
    fn test_multiple_in_clauses_cross_product_cap() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);

        let person_document_type = contract
            .document_type_for_name("person")
            .expect("contract should have a person document type");
        let first_names: Vec<String> = (0..20).map(|i| format!("First{i:02}")).collect();
        let last_names: Vec<String> = (0..6).map(|i| format!("Last{i}")).collect();
        let query_value = json!({
            "where": [
                ["firstName", "in", first_names],
                ["lastName", "in", last_names],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("query should be built");
        let error = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect_err("a 120-branch cross product must be rejected");
        assert!(
            matches!(error, Error::Query(QuerySyntaxError::InvalidInClause(_))),
            "expected InvalidInClause, got {error:?}"
        );
    }

    #[test]
    fn test_multiple_in_clauses_must_be_consecutive_index_properties() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);

        let person_document_type = contract
            .document_type_for_name("person")
            .expect("contract should have a person document type");
        // No index has middleName and lastName as a leading consecutive
        // run: [firstName, middleName, lastName] holds them at positions
        // 1 and 2 with no equality on firstName.
        let query_value = json!({
            "where": [
                ["middleName", "in", ["Ivanna", "Evangeline"]],
                ["lastName", "in", ["Kriskov", "Randolf"]],
            ],
            "limit": 100,
            "orderBy": [
                ["middleName", "asc"],
                ["lastName", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("query should be built");
        let error = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect_err("non-consecutive in clauses must be rejected");
        assert!(
            matches!(
                error,
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(_))
            ),
            "expected WhereClauseOnNonIndexedProperty, got {error:?}"
        );
    }

    #[test]
    fn test_multiple_in_clauses_require_order_by_on_each_in_field() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_family_tests(10, 73509, platform_version);

        let person_document_type = contract
            .document_type_for_name("person")
            .expect("contract should have a person document type");
        let query_value = json!({
            "where": [
                ["firstName", "in", ["Adey", "Briney"]],
                ["lastName", "in", ["Kriskov", "Randolf"]],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            person_document_type,
            &drive.config,
            platform_version,
        )
        .expect("query should be built");
        let error = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect_err("missing order by on an in field must be rejected");
        assert!(
            matches!(error, Error::Query(_)),
            "expected a query error, got {error:?}"
        );
    }
}
