//! The activation conversion of document sums: trees stored with a 64-bit sum under protocol
//! version 14 end up as the trees a chain born at protocol version 15 holds for the same
//! writes, and take sums past 64 bits from then on.

use crate::drive::document::migration::big_sums::DocumentSumConversionStats;
use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::QueryResultType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use crate::util::test_helpers::setup_contract;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::DataContract;
use dpp::document::document_event::DocumentEvent;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::prelude::IdentityNonce;
use dpp::system_data_contracts::SystemDataContract;
use dpp::tests::json_document::json_document_to_json_value;
use dpp::version::PlatformVersion;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

const SOURCE_CONTRACT: [u8; 32] = [9; 32];
const TIP_JAR: &str = "tests/supporting_files/contract/tip-jar/tip-jar-contract.json";

fn version_14() -> &'static PlatformVersion {
    PlatformVersion::get(14).expect("expected protocol version 14")
}

fn version_15() -> &'static PlatformVersion {
    PlatformVersion::get(15).expect("expected protocol version 15")
}

/// A drive holding the document history contract, as a chain's genesis stores it.
fn drive_with_document_history(platform_version: &PlatformVersion) -> Drive {
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = drive
        .cache
        .system_data_contracts
        .load_document_history(platform_version)
        .expect("expected the document history contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            None,
            platform_version,
        )
        .expect("expected to store the document history contract");
    drive
}

/// Records `event` for a document of the source contract, at its own block time.
fn record(
    drive: &Drive,
    event: DocumentEvent,
    nonce: IdentityNonce,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let operations = drive.add_document_history_operations(
        Identifier::new(SOURCE_CONTRACT),
        "card",
        Identifier::new([nonce as u8; 32]),
        Identifier::new([1; 32]),
        nonce,
        event,
        &BlockInfo::default_with_time(1_000 * nonce),
        &mut None,
        None,
        platform_version,
    )?;
    drive.apply_batch_low_level_drive_operations(
        None,
        None,
        operations,
        &mut vec![],
        &platform_version.drive,
    )
}

/// Two listings and two purchases whose prices fill what 64 bits hold.
fn record_prices(drive: &Drive, platform_version: &PlatformVersion) {
    for (nonce, price) in [(1, 5), (2, i64::MAX as u64 - 5)] {
        record(
            drive,
            DocumentEvent::PriceUpdate { price },
            nonce,
            platform_version,
        )
        .expect("expected to record the listing");
        record(
            drive,
            DocumentEvent::Purchase {
                seller_id: Identifier::new([2; 32]),
                price,
            },
            nonce + 10,
            platform_version,
        )
        .expect("expected to record the purchase");
    }
}

/// Every element at or below `path`, with the path it is stored under.
fn subtree_rows(
    drive: &Drive,
    path: Vec<Vec<u8>>,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Vec<(Vec<Vec<u8>>, Vec<u8>, Element)> {
    let mut query = Query::new();
    query.insert_all();
    let (children, _) = drive
        .grove_get_raw_path_query(
            &PathQuery::new(path.clone(), SizedQuery::new(query, None, None)),
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to read the tree");
    let mut rows = vec![];
    for (key, element) in children.to_key_elements() {
        let is_tree = element.underlying().is_any_tree();
        rows.push((path.clone(), key.clone(), element));
        if is_tree {
            let mut child_path = path.clone();
            child_path.push(key);
            rows.extend(subtree_rows(
                drive,
                child_path,
                transaction,
                platform_version,
            ));
        }
    }
    rows
}

/// The conversion as the first block of protocol version 15 runs it: in one transaction,
/// over contracts read under the new version.
fn convert(drive: &Drive) -> DocumentSumConversionStats {
    drive.cache.data_contracts.clear();
    let transaction = drive.grove.start_transaction();
    let stats = drive
        .convert_document_sums_to_big_sums(&transaction, version_15())
        .expect("expected the conversion");
    drive
        .grove
        .commit_transaction(transaction)
        .value
        .expect("expected to commit the conversion");
    stats
}

fn assert_consistent(drive: &Drive) {
    let issues = drive
        .grove
        .verify_grovedb(None, true, false, &version_15().drive.grove_version)
        .expect("expected the integrity check to run");
    assert!(issues.is_empty(), "integrity issues: {issues:?}");
}

fn history_type_path(document_type_name: &str) -> Vec<Vec<u8>> {
    contract_document_type_path_vec(
        SystemDataContract::DocumentHistory.id().as_bytes(),
        document_type_name,
    )
}

#[test]
fn should_convert_stored_price_sums_into_the_trees_a_chain_born_at_protocol_version_15_holds() {
    let upgraded = drive_with_document_history(version_14());
    record_prices(&upgraded, version_14());
    let stats = convert(&upgraded);

    let born = drive_with_document_history(version_15());
    record_prices(&born, version_15());

    for document_type_name in ["priceUpdate", "purchase", "transfer"] {
        assert_eq!(
            subtree_rows(
                &upgraded,
                history_type_path(document_type_name),
                None,
                version_15()
            ),
            subtree_rows(
                &born,
                history_type_path(document_type_name),
                None,
                version_15()
            ),
            "{document_type_name}"
        );
    }
    assert_eq!(stats.contracts, 1);
    // `purchase` and `priceUpdate` sum prices; `transfer` sums nothing.
    assert_eq!(stats.types, 2);
    assert!(stats.converted_trees > 0);
    assert_eq!(stats.trees_left_as_stored, 0);
    assert_consistent(&upgraded);

    // Nothing is left to convert.
    assert_eq!(
        convert(&upgraded),
        DocumentSumConversionStats {
            converted_trees: 0,
            ..stats
        }
    );
}

#[test]
fn should_record_prices_past_64_bits_after_the_conversion() {
    let drive = drive_with_document_history(version_14());
    record_prices(&drive, version_14());
    // The total of the listings is the most 64 bits hold: protocol version 14 takes no more.
    let error = record(
        &drive,
        DocumentEvent::PriceUpdate { price: 1 },
        3,
        version_14(),
    )
    .expect_err("expected the listing to be refused before the conversion");
    assert!(error.to_string().contains("overflow"), "{error}");

    convert(&drive);
    for nonce in 3..=4 {
        record(
            &drive,
            DocumentEvent::PriceUpdate {
                price: i64::MAX as u64,
            },
            nonce,
            version_15(),
        )
        .expect("expected to record the listing after the conversion");
    }

    let mut path = history_type_path("priceUpdate");
    path.push(b"dataContractId".to_vec());
    path.push(SOURCE_CONTRACT.to_vec());
    let path: Vec<&[u8]> = path.iter().map(Vec::as_slice).collect();
    let Element::ProvableCountProvableBigSumTree(_, count, sum, _) = drive
        .grove
        .get_raw(
            path.as_slice().into(),
            b"$createdAt",
            None,
            &version_15().drive.grove_version,
        )
        .value
        .expect("expected the prices of the contract's listings")
    else {
        panic!("expected the prices of the contract's listings in a 128-bit tree");
    };
    assert_eq!((count, sum), (4, 3 * i128::from(i64::MAX)));
    assert_consistent(&drive);
}

/// The tip jar's `tip` type sums `amount`, which may pass 2^27, through three indexes: one
/// keyed by recipient, one by time with range sums, and one by recipient and time, whose
/// second level sits wrapped in the value trees of the first.
fn tips(contract: &DataContract, platform_version: &PlatformVersion) -> Vec<Document> {
    let document_type = contract
        .document_type_for_name("tip")
        .expect("expected the tip document type");
    let mut tips: Vec<Document> = (1..=4u64)
        .map(|seed| {
            document_type
                .random_document(Some(seed), platform_version)
                .expect("expected a tip")
        })
        .collect();
    // Two tips to one recipient, so a value tree holds more than one entry
    let recipient = tips[0]
        .properties()
        .get("recipient")
        .cloned()
        .expect("expected a recipient");
    tips[1].set("recipient", recipient);
    for (tip, amount) in tips.iter_mut().zip([1u32, 70_000, u32::MAX, 12]) {
        tip.set("amount", Value::U32(amount));
    }
    tips
}

fn add_tips(
    drive: &Drive,
    contract: &DataContract,
    tips: &[Document],
    platform_version: &PlatformVersion,
) {
    let document_type = contract
        .document_type_for_name("tip")
        .expect("expected the tip document type");
    for tip in tips {
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((tip, None)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                false,
                BlockInfo::default_with_time(1_000),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the tip");
    }
}

#[test]
fn should_convert_a_stored_type_with_wrapped_levels_into_the_trees_a_chain_born_at_protocol_version_15_holds(
) {
    let upgraded = setup_drive_with_initial_state_structure(Some(version_14()));
    let contract_14 = setup_contract(
        &upgraded,
        TIP_JAR,
        None,
        None,
        None::<fn(&mut DataContract)>,
        None,
        Some(version_14()),
    );
    let tips = tips(&contract_14, version_14());
    add_tips(&upgraded, &contract_14, &tips, version_14());
    let stats = convert(&upgraded);

    let born = setup_drive_with_initial_state_structure(Some(version_15()));
    let contract_15 = setup_contract(
        &born,
        TIP_JAR,
        None,
        None,
        None::<fn(&mut DataContract)>,
        None,
        Some(version_15()),
    );
    add_tips(&born, &contract_15, &tips, version_15());

    let type_path = contract_document_type_path_vec(contract_14.id().as_bytes(), "tip");
    assert_eq!(
        subtree_rows(&upgraded, type_path.clone(), None, version_15()),
        subtree_rows(&born, type_path, None, version_15()),
    );
    assert_eq!((stats.contracts, stats.types), (1, 1));
    assert_eq!(stats.trees_left_as_stored, 0);
    assert_consistent(&upgraded);

    // The converted type takes a tip as a chain born at the version does
    let contract = upgraded
        .fetch_contract(contract_14.id().to_buffer(), None, None, None, version_15())
        .value
        .expect("expected to fetch the contract")
        .expect("expected the contract");
    let mut another = tips[0].clone();
    another.set_id(Identifier::new([77; 32]));
    add_tips(
        &upgraded,
        &contract.contract,
        std::slice::from_ref(&another),
        version_15(),
    );
    add_tips(
        &born,
        &contract_15,
        std::slice::from_ref(&another),
        version_15(),
    );
    let type_path = contract_document_type_path_vec(contract_14.id().as_bytes(), "tip");
    assert_eq!(
        subtree_rows(&upgraded, type_path.clone(), None, version_15()),
        subtree_rows(&born, type_path, None, version_15()),
    );
    assert_consistent(&upgraded);
}

#[test]
fn should_refuse_to_convert_under_a_protocol_version_that_keeps_no_big_sums() {
    let drive = setup_drive_with_initial_state_structure(Some(version_14()));
    let transaction = drive.grove.start_transaction();
    assert!(drive
        .convert_document_sums_to_big_sums(&transaction, version_14())
        .is_err());
}

/// A tip jar whose `byRecipient` index also counts with offsets: the entries tree of each
/// recipient is then a `ProvableCountSumTree`, the one tree with a 64-bit sum that has no
/// counterpart of its own shape. It becomes a `ProvableCountProvableBigSumTree`, the tree a
/// chain born at protocol version 15 creates for those entries.
fn tip_jar_counting_with_offsets(platform_version: &PlatformVersion) -> DataContract {
    let mut contract = json_document_to_json_value(TIP_JAR).expect("expected the tip jar");
    contract["documentSchemas"]["tip"]["indices"][0]["countable"] =
        serde_json::Value::String("countableAllowingOffset".to_string());
    DataContract::from_json(contract, false, platform_version)
        .expect("expected the contract to parse")
}

fn drive_with_contract(contract: &DataContract, platform_version: &PlatformVersion) -> Drive {
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    drive
        .apply_contract(
            contract,
            BlockInfo::default(),
            true,
            None,
            None,
            platform_version,
        )
        .expect("expected to store the contract");
    drive
}

#[test]
fn should_convert_a_tree_with_a_provable_count_and_a_plain_sum_into_the_tree_a_chain_born_at_protocol_version_15_holds(
) {
    let contract_14 = tip_jar_counting_with_offsets(version_14());
    let upgraded = drive_with_contract(&contract_14, version_14());
    let tips = tips(&contract_14, version_14());
    add_tips(&upgraded, &contract_14, &tips, version_14());

    let type_path = contract_document_type_path_vec(contract_14.id().as_bytes(), "tip");
    let entries_trees = |drive: &Drive, matches: fn(&Element) -> bool| {
        subtree_rows(drive, type_path.clone(), None, version_15())
            .iter()
            .filter(|(_, _, element)| matches(element.underlying()))
            .count()
    };
    // One entries tree per recipient: three recipients hold the four tips
    assert_eq!(
        entries_trees(&upgraded, |element| matches!(
            element,
            Element::ProvableCountSumTree(..)
        )),
        3
    );

    let stats = convert(&upgraded);
    assert_eq!(stats.trees_left_as_stored, 0);
    assert_eq!(
        entries_trees(&upgraded, |element| matches!(
            element,
            Element::SumTree(..)
                | Element::CountSumTree(..)
                | Element::ProvableSumTree(..)
                | Element::ProvableCountSumTree(..)
                | Element::ProvableCountProvableSumTree(..)
        )),
        0,
        "no tree of the type keeps a 64-bit sum"
    );

    let contract_15 = tip_jar_counting_with_offsets(version_15());
    let born = drive_with_contract(&contract_15, version_15());
    add_tips(&born, &contract_15, &tips, version_15());
    assert_eq!(
        subtree_rows(&upgraded, type_path.clone(), None, version_15()),
        subtree_rows(&born, type_path.clone(), None, version_15()),
    );
    assert_consistent(&upgraded);

    // The converted type takes a tip as a chain born at the version does
    let contract = upgraded
        .fetch_contract(contract_14.id().to_buffer(), None, None, None, version_15())
        .value
        .expect("expected to fetch the contract")
        .expect("expected the contract");
    let mut another = tips[0].clone();
    another.set_id(Identifier::new([77; 32]));
    add_tips(
        &upgraded,
        &contract.contract,
        std::slice::from_ref(&another),
        version_15(),
    );
    add_tips(
        &born,
        &contract_15,
        std::slice::from_ref(&another),
        version_15(),
    );
    assert_eq!(
        subtree_rows(&upgraded, type_path.clone(), None, version_15()),
        subtree_rows(&born, type_path.clone(), None, version_15()),
    );
    assert_consistent(&upgraded);
}
