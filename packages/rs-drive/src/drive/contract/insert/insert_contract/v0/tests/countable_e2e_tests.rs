//! End-to-end coverage for `documentsCountable` / `rangeCountable`.
//!
//! These tests exercise the full feature path:
//!   - Build a v12 contract with the flag set in the schema.
//!   - Apply it to a real Drive (grovedb).
//!   - Read the primary-key tree element back from grove and assert the
//!     concrete tree variant (NormalTree / CountTree / ProvableCountTree)
//!     matches what the schema requested.
//!   - For the count variants, insert and delete documents and assert the
//!     tree's internal count moves accordingly.

use crate::drive::Drive;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::DataContractFactory;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::DocumentV0Getters;
use dpp::platform_value::{platform_value, Value};
use dpp::tests::utils::generate_random_identifier_struct;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb, PathTrunkChunkQuery};

const PROTOCOL_VERSION_V12: u32 = 12;

/// Builds a v12 `DataContract` whose single `widget` document type has
/// `documentsCountable` / `rangeCountable` set to the requested values.
fn build_widget_contract(
    documents_countable: bool,
    range_countable: bool,
) -> dpp::prelude::DataContract {
    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");

    let mut document_schema = platform_value!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "position": 0,
                "maxLength": 64,
            }
        },
        "additionalProperties": false,
    });
    if documents_countable {
        document_schema.as_map_mut().unwrap().push((
            Value::Text("documentsCountable".to_string()),
            Value::Bool(true),
        ));
    }
    if range_countable {
        document_schema
            .as_map_mut()
            .unwrap()
            .push((Value::Text("rangeCountable".to_string()), Value::Bool(true)));
    }

    let schemas = platform_value!({ "widget": document_schema });
    let owner_id = generate_random_identifier_struct();

    factory
        .create_with_value_config(owner_id, 0, schemas, None, None)
        .expect("expected to create data contract")
        .data_contract_owned()
}

/// Reads the primary-key tree element directly from grove and returns it.
fn read_primary_key_tree(
    drive: &Drive,
    contract: &dpp::prelude::DataContract,
    document_type_name: &str,
) -> Element {
    let pv = PlatformVersion::latest();
    let contract_id = contract.id();
    let path: [&[u8]; 4] = [
        &[crate::drive::RootTree::DataContractDocuments as u8],
        contract_id.as_bytes(),
        &[1],
        document_type_name.as_bytes(),
    ];
    drive
        .grove_get_raw(
            (&path).into(),
            &[0],
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &pv.drive,
        )
        .expect("expected grove_get_raw to succeed")
        .expect("primary key tree element should exist")
}

fn primary_key_tree_path(
    contract: &dpp::prelude::DataContract,
    document_type_name: &str,
) -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        document_type_name.as_bytes().to_vec(),
        vec![0],
    ]
}

#[test]
fn default_contract_creates_normal_tree_for_primary_key() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_contract(false, false);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply contract");

    let elem = read_primary_key_tree(&drive, &contract, "widget");
    assert!(
        matches!(elem, Element::Tree(..)),
        "default (non-countable) contract should use a NormalTree primary key tree, got {:?}",
        elem
    );
}

#[test]
fn documents_countable_contract_creates_count_tree_for_primary_key() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_contract(true, false);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply contract");

    let elem = read_primary_key_tree(&drive, &contract, "widget");
    match &elem {
        Element::CountTree(_, count, _) => {
            assert_eq!(*count, 0, "freshly inserted CountTree should have count 0");
        }
        other => panic!(
            "documentsCountable contract should use a CountTree primary key tree, got {:?}",
            other
        ),
    }

    // Sanity: the parsed DocumentTypeV2 also reports the flag.
    let dt = contract
        .document_type_for_name("widget")
        .expect("widget exists");
    let dt_owned = dt.to_owned_document_type();
    match dt_owned {
        dpp::data_contract::document_type::DocumentType::V2(v2) => {
            assert!(v2.documents_countable());
            assert!(!v2.range_countable());
        }
        other => panic!("expected DocumentType::V2 on protocol v12, got {:?}", other),
    }
}

#[test]
fn range_countable_contract_creates_provable_count_tree_for_primary_key() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_contract(false, true);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply contract");

    let elem = read_primary_key_tree(&drive, &contract, "widget");
    assert!(
        matches!(elem, Element::ProvableCountTree(..)),
        "rangeCountable contract should use a ProvableCountTree primary key tree, got {:?}",
        elem
    );

    // rangeCountable implies documents_countable in the parser.
    let dt = contract
        .document_type_for_name("widget")
        .expect("widget exists");
    let dt_owned = dt.to_owned_document_type();
    match dt_owned {
        dpp::data_contract::document_type::DocumentType::V2(v2) => {
            assert!(v2.range_countable());
            assert!(v2.documents_countable());
        }
        other => panic!("expected DocumentType::V2 on protocol v12, got {:?}", other),
    }
}

#[test]
fn count_tree_count_grows_and_shrinks_with_document_inserts_and_deletes() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_contract(true, false);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply contract");

    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");

    // Insert 3 documents.
    let mut doc_ids = vec![];
    for seed in 1u64..=3 {
        let document = document_type
            .random_document(Some(seed), pv)
            .expect("random document");
        doc_ids.push(document.id());

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to insert document");
    }

    let elem_after_inserts = read_primary_key_tree(&drive, &contract, "widget");
    match elem_after_inserts {
        Element::CountTree(_, count, _) => {
            assert_eq!(count, 3, "count tree should track 3 inserted documents");
        }
        other => panic!("expected CountTree, got {:?}", other),
    }

    // Delete one.
    drive
        .delete_document_for_contract(
            doc_ids[0],
            &contract,
            "widget",
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete document");

    let elem_after_delete = read_primary_key_tree(&drive, &contract, "widget");
    match elem_after_delete {
        Element::CountTree(_, count, _) => {
            assert_eq!(count, 2, "count tree should drop to 2 after one delete");
        }
        other => panic!("expected CountTree, got {:?}", other),
    }
}

#[test]
fn provable_count_tree_count_grows_with_document_inserts() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_contract(false, true);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply contract");

    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");

    for seed in 1u64..=5 {
        let document = document_type
            .random_document(Some(seed), pv)
            .expect("random document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to insert document");
    }

    let elem = read_primary_key_tree(&drive, &contract, "widget");
    match elem {
        Element::ProvableCountTree(_, count, _) => {
            assert_eq!(count, 5, "provable count tree should track 5 documents");
        }
        other => panic!("expected ProvableCountTree, got {:?}", other),
    }
}

#[test]
fn range_countable_primary_key_tree_supports_trunk_proof() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_contract(false, true);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply contract");

    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");

    for seed in 1u64..=20 {
        let document = document_type
            .random_document(Some(seed), pv)
            .expect("random document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to insert document");
    }

    let elem = read_primary_key_tree(&drive, &contract, "widget");
    match elem {
        Element::ProvableCountTree(_, count, _) => {
            assert_eq!(count, 20, "provable count tree should track inserted docs");
        }
        other => panic!("expected ProvableCountTree, got {:?}", other),
    }

    let query = PathTrunkChunkQuery::new(primary_key_tree_path(&contract, "widget"), 3);
    let proof = drive
        .grove
        .prove_trunk_chunk(&query, &pv.drive.grove_version)
        .value
        .expect("expected trunk proof call to succeed");
    let (root_hash, result) =
        GroveDb::verify_trunk_chunk_proof(&proof, &query, &pv.drive.grove_version)
            .expect("expected trunk proof to verify");

    assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
    assert!(
        !result.elements.is_empty(),
        "trunk proof should return primary-key tree elements"
    );
    assert!(
        result
            .leaf_keys
            .values()
            .any(|leaf_info| leaf_info.count.is_some()),
        "rangeCountable trunk proof should expose subtree counts"
    );
}

/// Sanity: existing document fetch + count APIs still work for a CountTree
/// contract — i.e. switching the underlying primary-key tree variant
/// does not break document iteration.
#[test]
fn count_tree_contract_supports_document_fetch() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_contract(true, false);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply contract");

    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");

    let document = document_type
        .random_document(Some(42), pv)
        .expect("random document");
    let inserted_id = document.id();

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to insert document");

    let query = crate::query::DriveDocumentQuery::all_items_query(&contract, document_type, None);
    let (docs, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, pv)
        .expect("expected query to succeed");
    assert_eq!(docs.len(), 1, "should fetch exactly the inserted document");
    let decoded = dpp::document::Document::from_bytes(&docs[0], document_type, pv)
        .expect("expected to decode document");
    assert_eq!(decoded.id(), inserted_id);
}

/// Apply a contract with the given countable flags and return the fees
/// reported by `insert_contract`. Used to compare fee profiles across
/// the three primary-key tree variants.
fn fees_for_contract_with(
    documents_countable: bool,
    range_countable: bool,
) -> dpp::fee::fee_result::FeeResult {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_contract(documents_countable, range_countable);
    drive
        .insert_contract(&contract, BlockInfo::default(), true, None, pv)
        .expect("expected insert_contract to succeed and return fees")
}

/// Switching the primary-key tree variant from NormalTree to CountTree
/// changes the underlying grovedb element shape (CountTree carries an
/// extra count value). The reported fees must therefore differ — if they
/// don't, the contract insert path silently degraded back to the
/// NormalTree branch and the documentsCountable feature is dead.
#[test]
fn count_tree_contract_apply_produces_different_fees_than_normal_tree() {
    let normal_fees = fees_for_contract_with(false, false);
    let count_fees = fees_for_contract_with(true, false);

    assert!(normal_fees.storage_fee > 0, "normal tree storage fee");
    assert!(normal_fees.processing_fee > 0, "normal tree processing fee");
    assert!(count_fees.storage_fee > 0, "count tree storage fee");
    assert!(count_fees.processing_fee > 0, "count tree processing fee");

    assert_ne!(
        (normal_fees.storage_fee, normal_fees.processing_fee),
        (count_fees.storage_fee, count_fees.processing_fee),
        "documentsCountable: true must produce a different fee profile than the default \
         NormalTree contract — equal fees mean the count-tree branch was never exercised"
    );
}

/// Same invariant for the rangeCountable / ProvableCountTree branch:
/// switching from CountTree to ProvableCountTree changes both the grove
/// element type and the proof shape, so fees must differ.
#[test]
fn provable_count_tree_contract_apply_produces_different_fees_than_count_tree() {
    let count_fees = fees_for_contract_with(true, false);
    let provable_fees = fees_for_contract_with(false, true);

    assert!(provable_fees.storage_fee > 0, "provable count storage fee");
    assert!(
        provable_fees.processing_fee > 0,
        "provable count processing fee"
    );

    assert_ne!(
        (count_fees.storage_fee, count_fees.processing_fee),
        (provable_fees.storage_fee, provable_fees.processing_fee,),
        "rangeCountable: true must produce a different fee profile than documentsCountable: \
         true alone — equal fees mean the provable-count-tree branch was never exercised"
    );
}

/// Document insert into a CountTree contract should produce positive fees
/// without error. This exercises the document-insert code paths
/// (add_document_for_contract_operations, primary-key-tree dispatch in
/// add_document_to_primary_storage) under the count-tree branch.
#[test]
fn document_insert_into_count_tree_produces_positive_fees() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_contract(true, false);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply contract");

    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");
    let document = document_type
        .random_document(Some(7), pv)
        .expect("random document");

    let fee = drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to insert document into count tree");

    assert!(
        fee.storage_fee > 0,
        "document insert into a CountTree contract must produce a positive storage fee"
    );
    assert!(
        fee.processing_fee > 0,
        "document insert into a CountTree contract must produce a positive processing fee"
    );
}
