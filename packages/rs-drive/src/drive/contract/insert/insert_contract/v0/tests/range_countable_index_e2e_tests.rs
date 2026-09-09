//! End-to-end coverage for an *indexed* `rangeCountable` property.
//!
//! Where `countable_e2e_tests` only checks the document-type-level flag
//! (`documentsCountable` / `rangeCountable` on the document type, which
//! drives the primary-key tree variant), this module builds a contract
//! whose `indices` section contains a `rangeCountable: true` index over
//! a property and verifies the *index storage tree shape*:
//!
//!   - `[contract_doc, doctype, "color"]` is a `ProvableCountTree`
//!     (created at contract setup).
//!   - `[..., "color", <c1>]` is a `CountTree` (created on document
//!     insert by the index walker), whose count tracks how many docs
//!     have that color value.
//!   - Sibling continuations under that `CountTree` (compound index
//!     suffixes) are wrapped with `Element::NonCounted` so they
//!     contribute 0 to the parent count.

use crate::drive::Drive;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::DataContractFactory;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::DataContract;
use dpp::tests::utils::generate_random_identifier_struct;
use dpp::version::PlatformVersion;
use grovedb::Element;

const PROTOCOL_VERSION_V12: u32 = 12;

/// Build a v12 contract whose `widget` document type has a
/// `rangeCountable: true` single-property index over `color`. The
/// optional `compound_index` adds a non-range-countable compound
/// `[color, size]` index so we can verify NonCounted-wrapping of the
/// sibling continuation.
fn build_widget_with_color_index(compound_index: bool) -> DataContract {
    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");

    let mut indices = vec![platform_value!({
        "name": "byColor",
        "properties": [{"color": "asc"}],
        "countable": "countable",
        "rangeCountable": true,
    })];
    if compound_index {
        indices.push(platform_value!({
            "name": "byColorSize",
            "properties": [{"color": "asc"}, {"size": "asc"}],
        }));
    }

    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "color": {
                "type": "string",
                "position": 0,
                "maxLength": 32,
            },
            "size": {
                "type": "string",
                "position": 1,
                "maxLength": 32,
            },
        },
        "indices": Value::Array(indices),
        "additionalProperties": false,
    });

    let schemas = platform_value!({ "widget": document_schema });
    let owner_id = generate_random_identifier_struct();

    factory
        .create_with_value_config(owner_id, 0, schemas, None, None)
        .expect("expected to create data contract")
        .data_contract_owned()
}

fn property_name_tree_path(
    contract: &DataContract,
    document_type_name: &str,
    property_name: &str,
) -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        document_type_name.as_bytes().to_vec(),
        property_name.as_bytes().to_vec(),
    ]
}

fn read_grove_element(drive: &Drive, path: &[Vec<u8>], key: &[u8]) -> Option<Element> {
    let pv = PlatformVersion::latest();
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    drive
        .grove_get_raw(
            path_refs.as_slice().into(),
            key,
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &pv.drive,
        )
        .expect("grove_get_raw should succeed")
}

fn build_widget_doc(contract: &DataContract, color: &str, size: &str, seed: u64) -> Document {
    let pv = PlatformVersion::latest();
    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");
    let mut doc = document_type
        .random_document(Some(seed), pv)
        .expect("random document");
    let mut props = std::collections::BTreeMap::new();
    props.insert("color".to_string(), Value::Text(color.to_string()));
    props.insert("size".to_string(), Value::Text(size.to_string()));
    doc.set_properties(props);
    doc
}

/// The top-level property-name tree at `[contract_doc, doctype, "color"]`
/// must be a `ProvableCountTree` for a contract with a `rangeCountable`
/// single-property index over `color`. This is the layer that
/// `AggregateCountOnRange` walks for O(log n) range counts.
#[test]
fn property_name_tree_for_range_countable_index_is_provable_count_tree() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(false);

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

    let path = property_name_tree_path(&contract, "widget", "color");
    let parent_path: Vec<Vec<u8>> = path[..path.len() - 1].to_vec();
    let key = path.last().unwrap().clone();
    let elem = read_grove_element(&drive, &parent_path, &key)
        .expect("color property-name tree must exist");
    match elem {
        Element::ProvableCountTree(_, count, _) => {
            assert_eq!(
                count, 0,
                "freshly created property-name ProvableCountTree should have aggregate 0"
            );
        }
        other => panic!(
            "rangeCountable index property-name tree should be ProvableCountTree, got {:?}",
            other
        ),
    }
}

/// Inserting a document whose indexed property has value `c1` creates
/// the value tree at `[contract_doc, doctype, "color", "c1"]`. With
/// `rangeCountable: true` the walker must lay this down as a
/// `CountTree` so the parent property-name `ProvableCountTree`'s
/// aggregate sums per-value counts cleanly.
#[test]
fn value_tree_for_range_countable_index_is_count_tree_after_insert() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(false);

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
    let doc = build_widget_doc(&contract, "red", "small", 1);

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&doc, None)),
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

    // Property-name aggregate should now reflect the inserted doc.
    let property_path = property_name_tree_path(&contract, "widget", "color");
    let prop_parent: Vec<Vec<u8>> = property_path[..property_path.len() - 1].to_vec();
    let prop_key = property_path.last().unwrap().clone();
    let prop_elem = read_grove_element(&drive, &prop_parent, &prop_key)
        .expect("color property-name tree must exist");
    match prop_elem {
        Element::ProvableCountTree(_, count, _) => {
            assert_eq!(
                count, 1,
                "ProvableCountTree aggregate should be 1 after inserting one doc"
            );
        }
        other => panic!("expected ProvableCountTree, got {:?}", other),
    }

    // Value tree at <c1> should be a CountTree counting the docs with
    // color="red".
    let value_elem = read_grove_element(&drive, &property_path, b"red")
        .expect("value tree for color=red must exist");
    match value_elem {
        Element::CountTree(_, count, _) => {
            assert_eq!(count, 1, "value-tree CountTree should count 1 doc");
        }
        other => panic!(
            "rangeCountable value tree should be a CountTree, got {:?}",
            other
        ),
    }
}

/// Walking the same property's IndexLevel for a *compound* sibling
/// index `[color, size]` requires the walker to insert a continuation
/// property-name tree under the `CountTree` value tree. That
/// continuation must be wrapped with `Element::NonCounted` so it
/// contributes 0 to the value tree's count — otherwise the count
/// would be `1 (reference) + 1 (continuation NormalTree) = 2` per
/// inserted doc instead of the correct `1`.
#[test]
fn count_tree_value_count_excludes_compound_continuation_via_non_counted() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(true);

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
    let doc = build_widget_doc(&contract, "red", "small", 1);

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&doc, None)),
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

    // CountTree count must be exactly 1 (the doc reference), even
    // though there's a compound continuation tree inserted as a
    // sibling. If NonCounted-wrapping is broken, count will be 2 (or
    // more, depending on how the [0] tree contributes).
    let property_path = property_name_tree_path(&contract, "widget", "color");
    let value_elem = read_grove_element(&drive, &property_path, b"red")
        .expect("value tree for color=red must exist");
    match value_elem {
        Element::CountTree(_, count, _) => {
            assert_eq!(
                count, 1,
                "CountTree count should equal exactly the number of docs with color=red, \
                 not including the compound-index continuation tree (NonCounted wrapping \
                 check)"
            );
        }
        other => panic!("expected CountTree, got {:?}", other),
    }

    // The compound continuation property-name tree at [..., "color",
    // "red", "size"] should exist and be wrapped with NonCounted.
    let mut size_path = property_path.clone();
    size_path.push(b"red".to_vec());
    let size_elem = read_grove_element(&drive, &size_path, b"size")
        .expect("compound continuation tree at 'size' must exist");
    match size_elem {
        Element::NonCounted(inner) => match inner.as_ref() {
            Element::Tree(_, _) => {} // expected: NonCounted<NormalTree>
            other => panic!(
                "expected NonCounted<NormalTree>, got NonCounted<{:?}>",
                other
            ),
        },
        other => panic!(
            "compound continuation under a CountTree must be NonCounted-wrapped, got {:?}",
            other
        ),
    }
}

/// Deleting a document under a `range_countable` index must decrement
/// the value tree's `CountTree` and the parent property-name tree's
/// `ProvableCountTree` aggregate. If the delete walker doesn't see
/// the right tree variants in cost estimation, removals can leave
/// stale references or over-bill the operation; this test pins the
/// observable outcome (counts after delete).
#[test]
fn delete_decrements_count_tree_and_provable_count_aggregate() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(false);

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

    // Insert two docs at color="red" so we can delete one and watch
    // the count drop from 2 → 1 (instead of 1 → 0, which is also
    // correct but doesn't distinguish "decrement" from "tree
    // collapsed").
    let doc1 = build_widget_doc(&contract, "red", "small", 1);
    let doc2 = build_widget_doc(&contract, "red", "large", 2);
    for doc in [&doc1, &doc2] {
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((doc, None)),
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

    let property_path = property_name_tree_path(&contract, "widget", "color");

    // Sanity: 2 docs, both red.
    let value_elem = read_grove_element(&drive, &property_path, b"red").expect("value tree exists");
    match value_elem {
        Element::CountTree(_, count, _) => assert_eq!(count, 2),
        other => panic!("expected CountTree, got {:?}", other),
    }

    drive
        .delete_document_for_contract(
            doc1.id(),
            &contract,
            "widget",
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete document");

    let prop_parent: Vec<Vec<u8>> = property_path[..property_path.len() - 1].to_vec();
    let prop_key = property_path.last().unwrap().clone();
    let prop_elem =
        read_grove_element(&drive, &prop_parent, &prop_key).expect("property-name tree exists");
    match prop_elem {
        Element::ProvableCountTree(_, count, _) => assert_eq!(
            count, 1,
            "ProvableCountTree aggregate should drop to 1 after one delete"
        ),
        other => panic!("expected ProvableCountTree, got {:?}", other),
    }
    let value_elem = read_grove_element(&drive, &property_path, b"red").expect("value tree exists");
    match value_elem {
        Element::CountTree(_, count, _) => assert_eq!(
            count, 1,
            "CountTree count should drop to 1 after one delete"
        ),
        other => panic!("expected CountTree, got {:?}", other),
    }
}

/// Inserting multiple docs at the same color value increments the
/// CountTree, and the aggregate at the property-name
/// `ProvableCountTree` reflects the total across all values.
#[test]
fn aggregate_count_grows_across_distinct_values() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(false);

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

    for (i, color) in ["red", "red", "blue", "green", "green", "green"]
        .iter()
        .enumerate()
    {
        let doc = build_widget_doc(&contract, color, "small", (i + 1) as u64);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
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

    let property_path = property_name_tree_path(&contract, "widget", "color");

    // 6 inserts total → ProvableCountTree aggregate = 6
    let prop_parent: Vec<Vec<u8>> = property_path[..property_path.len() - 1].to_vec();
    let prop_key = property_path.last().unwrap().clone();
    let prop_elem =
        read_grove_element(&drive, &prop_parent, &prop_key).expect("property-name tree exists");
    match prop_elem {
        Element::ProvableCountTree(_, count, _) => assert_eq!(count, 6),
        other => panic!("expected ProvableCountTree, got {:?}", other),
    }

    // Per-value counts: red=2, blue=1, green=3
    for (color, expected) in [("red", 2u64), ("blue", 1), ("green", 3)] {
        let value_elem = read_grove_element(&drive, &property_path, color.as_bytes())
            .unwrap_or_else(|| panic!("value tree for color={} must exist", color));
        match value_elem {
            Element::CountTree(_, count, _) => {
                assert_eq!(count, expected, "color={} CountTree count mismatch", color)
            }
            other => panic!("expected CountTree at color={}, got {:?}", color, other),
        }
    }
}

/// End-to-end exercise of the range count executor:
/// `DriveDocumentCountQuery::execute_range_count_no_proof`. With six
/// docs at three distinct color values, a `> "blue"` range
/// should hit `green` (3 docs) and `red` (2 docs) for a total of 5,
/// and `distinct = true` returns one entry per matching value.
#[test]
fn range_count_executor_sums_and_splits_correctly() {
    use crate::query::{DriveDocumentCountQuery, RangeCountOptions, WhereClause, WhereOperator};

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(false);

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

    for (i, color) in ["red", "red", "blue", "green", "green", "green"]
        .iter()
        .enumerate()
    {
        let doc = build_widget_doc(&contract, color, "small", (i + 1) as u64);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
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

    // Find the range_countable index via the picker so the test
    // doesn't depend on any particular index name.
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::GreaterThan,
        value: dpp::platform_value::Value::Text("blue".to_string()),
    }];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("range_countable index should be picked");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses: where_clauses.clone(),
    };

    // distinct=false: single summed entry. green(3) + red(2) = 5.
    let summed = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: false,
                limit: None,
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("range count should succeed");
    assert_eq!(summed.len(), 1);
    assert!(summed[0].key.is_empty(), "summed entry has empty key");
    assert_eq!(
        summed[0].count,
        Some(5),
        "color > 'blue' should sum to 3 (green) + 2 (red) = 5"
    );

    // distinct=true: per-value entries, ascending. Should be
    // [(green, 3), (red, 2)] — `blue` is excluded by the
    // exclusive lower bound.
    let split = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: true,
                limit: None,
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("range count should succeed");
    assert_eq!(split.len(), 2);
    assert_eq!(split[0].key, b"green".to_vec());
    assert_eq!(split[0].count, Some(3));
    assert_eq!(split[1].key, b"red".to_vec());
    assert_eq!(split[1].count, Some(2));

    // distinct=true with limit=1: only the first entry.
    let limited = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: true,
                limit: Some(1),
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("range count should succeed");
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].key, b"green".to_vec());

    // Pagination via range adjustment: `color > "green"` (rather
    // than `color > "blue"` + a cursor field) yields the same
    // "everything past green" page, which here is just red.
    let after_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::GreaterThan,
        value: dpp::platform_value::Value::Text("green".to_string()),
    }];
    let after_index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &after_clauses,
        &[],
    )
    .expect("range_countable index should be picked");
    let after_query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index: after_index,
        where_clauses: after_clauses,
    };
    let after = after_query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: true,
                limit: None,
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("range count should succeed");
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].key, b"red".to_vec());

    // distinct=true descending: [(red, 2), (green, 3)].
    let desc = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: true,
                limit: None,
                order_by_ascending: false,
            },
            None,
            pv,
        )
        .expect("range count should succeed");
    assert_eq!(desc.len(), 2);
    assert_eq!(desc[0].key, b"red".to_vec());
    assert_eq!(desc[1].key, b"green".to_vec());
}

/// `Between [a, b]` is inclusive on both ends — a value at
/// exactly the lower or upper bound must be counted.
#[test]
fn range_count_executor_between_is_inclusive_on_both_bounds() {
    use crate::query::{DriveDocumentCountQuery, RangeCountOptions, WhereClause, WhereOperator};

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(false);

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

    for (i, color) in ["aaa", "bbb", "ccc", "ddd"].iter().enumerate() {
        let doc = build_widget_doc(&contract, color, "small", (i + 1) as u64);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
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

    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::Between,
        value: dpp::platform_value::Value::Array(vec![
            dpp::platform_value::Value::Text("bbb".to_string()),
            dpp::platform_value::Value::Text("ccc".to_string()),
        ]),
    }];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("range_countable index should be picked");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses,
    };

    let split = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: true,
                limit: None,
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("range count should succeed");
    assert_eq!(split.len(), 2);
    assert_eq!(split[0].key, b"bbb".to_vec());
    assert_eq!(split[0].count, Some(1));
    assert_eq!(split[1].key, b"ccc".to_vec());
    assert_eq!(split[1].count, Some(1));
}

/// `execute_aggregate_count_with_proof` should produce a grovedb
/// `AggregateCountOnRange` proof that verifies to the same total
/// count as the no-proof range walk. This is the prove-path
/// counterpart of [`range_count_executor_sums_and_splits_correctly`].
///
/// The verification step uses
/// `GroveDb::verify_aggregate_count_query` directly — proves the
/// returned bytes are a real proof, not just any blob — and asserts
/// the recovered count matches the no-proof sum.
#[test]
fn aggregate_count_proof_verifies_and_returns_correct_count() {
    use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
    use grovedb::{GroveDb, PathQuery};

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(false);

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

    // Same six-doc fixture as the no-proof test.
    for (i, color) in ["red", "red", "blue", "green", "green", "green"]
        .iter()
        .enumerate()
    {
        let doc = build_widget_doc(&contract, color, "small", (i + 1) as u64);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
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

    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::GreaterThan,
        value: dpp::platform_value::Value::Text("blue".to_string()),
    }];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("range_countable index should be picked");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses: where_clauses.clone(),
    };

    let proof_bytes = query
        .execute_aggregate_count_with_proof(&drive, None, pv)
        .expect("should generate aggregate count proof");
    assert!(!proof_bytes.is_empty(), "proof must not be empty");

    // Reconstruct the same path query the prover used, verify the
    // proof against it, and check the recovered count.
    let path = vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1u8],
        b"widget".to_vec(),
        b"color".to_vec(),
    ];
    let query_item = grovedb::QueryItem::RangeAfter(b"blue".to_vec()..);
    let path_query = PathQuery::new_aggregate_count_on_range(path, query_item);

    let (root_hash, count) =
        GroveDb::verify_aggregate_count_query(&proof_bytes, &path_query, &pv.drive.grove_version)
            .expect("aggregate-count proof should verify");
    assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
    assert_eq!(
        count, 5,
        "verified count should match no-proof sum: 3 (green) + 2 (red) = 5"
    );
}

/// Range count with an `In` clause on the prefix forks the walk
/// into one path per prefix value. Each emitted entry carries
/// the `in_key` (the brand) alongside `key` (the color) — the
/// server does NOT merge across forks, because limit applied
/// pre-merge could undercount cross-fork sums (the entries the
/// limit drops on one fork might be the ones whose key collides
/// with another fork's surviving entries). Callers reduce by
/// `key` client-side via `DocumentSplitCounts::into_flat_map` if
/// they want the flat histogram view.
#[test]
fn range_count_with_in_on_prefix_returns_per_brand_color_entries() {
    use crate::query::{DriveDocumentCountQuery, RangeCountOptions, WhereClause, WhereOperator};
    use dpp::platform_value::Value;

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();

    // Build a contract with `[brand, color]` range_countable.
    let factory = dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12)
        .expect("expected to create factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "brand": { "type": "string", "position": 0, "maxLength": 32 },
            "color": { "type": "string", "position": 1, "maxLength": 32 },
        },
        "indices": [{
            "name": "byBrandColor",
            "properties": [{"brand": "asc"}, {"color": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "widget": document_schema });
    let contract = factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned();

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply contract");

    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");

    // 3 acme + red, 2 acme + blue, 2 contoso + red, 1 contoso + green.
    let docs: Vec<(&str, &str)> = vec![
        ("acme", "red"),
        ("acme", "red"),
        ("acme", "red"),
        ("acme", "blue"),
        ("acme", "blue"),
        ("contoso", "red"),
        ("contoso", "red"),
        ("contoso", "green"),
    ];
    for (i, (brand, color)) in docs.iter().enumerate() {
        let mut doc = document_type
            .random_document(Some((i + 1) as u64), pv)
            .expect("random doc");
        let mut props = std::collections::BTreeMap::new();
        props.insert("brand".to_string(), Value::Text(brand.to_string()));
        props.insert("color".to_string(), Value::Text(color.to_string()));
        doc.set_properties(props);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
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
            .expect("insert");
    }

    // brand IN (acme, contoso) AND color > "blue"
    // Match: acme+red(3), contoso+red(2), contoso+green(1) = 6
    // (Excluded: acme+blue, contoso+blue — but there's no
    //  contoso+blue, just acme+blue which doesn't match.)
    let where_clauses = vec![
        WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("acme".to_string()),
                Value::Text("contoso".to_string()),
            ]),
        },
        WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        },
    ];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("range_countable index should be picked");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses,
    };

    // Distinct mode: per-(brand, color) entries, unmerged.
    // brand=acme + color > "blue" matches red(3).
    // brand=contoso + color > "blue" matches red(2), green(1).
    // Expected order: ascending (in_key, key) tuple →
    //   (acme, red)     count=3
    //   (contoso, green) count=1
    //   (contoso, red)   count=2
    let split = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: true,
                limit: None,
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("range count should succeed");
    assert_eq!(
        split.len(),
        3,
        "expected unmerged per-(brand, color) entries, not a cross-fork sum"
    );
    assert_eq!(split[0].in_key.as_deref(), Some(b"acme".as_slice()));
    assert_eq!(split[0].key, b"red".to_vec());
    assert_eq!(split[0].count, Some(3));
    assert_eq!(split[1].in_key.as_deref(), Some(b"contoso".as_slice()));
    assert_eq!(split[1].key, b"green".to_vec());
    assert_eq!(split[1].count, Some(1));
    assert_eq!(split[2].in_key.as_deref(), Some(b"contoso".as_slice()));
    assert_eq!(split[2].key, b"red".to_vec());
    assert_eq!(split[2].count, Some(2));

    // Client-side merge over `key` recovers the flat histogram:
    //   green: 1
    //   red:   3 + 2 = 5
    let merged: std::collections::BTreeMap<Vec<u8>, u64> =
        split
            .iter()
            .fold(std::collections::BTreeMap::new(), |mut m, e| {
                *m.entry(e.key.clone()).or_insert(0) += e.count.unwrap_or(0);
                m
            });
    assert_eq!(merged.get(b"green".as_slice()), Some(&1));
    assert_eq!(merged.get(b"red".as_slice()), Some(&5));

    // Summed mode: 6 docs total across all forks.
    let summed = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: false,
                limit: None,
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("range count should succeed");
    assert_eq!(summed.len(), 1);
    assert!(
        summed[0].in_key.is_none(),
        "summed mode always emits a single in_key=None, key=empty entry"
    );
    assert!(summed[0].key.is_empty());
    assert_eq!(summed[0].count, Some(6));
}

/// `StartsWith "r"` is encoded as `Range(serialize("r")..
/// serialize("r") with last byte +1)` — the same half-open
/// byte-incremented encoding `conditions.rs:1129`'s `StartsWith`
/// arm uses for the normal docs path. On the count fast path this
/// becomes a `QueryItem::Range(..)` no different in structure from
/// `betweenExcludeRight`, so all four executor modes (no-proof
/// aggregate, no-proof distinct, prove aggregate, prove distinct)
/// serve it via the same code paths that already cover `>` / `<`
/// / `between*`. This test pins acceptance across all four.
#[test]
fn range_count_executor_accepts_starts_with_in_all_four_modes() {
    use crate::query::{DriveDocumentCountQuery, RangeCountOptions, WhereClause, WhereOperator};
    use grovedb::GroveDb;

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(false);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply contract");

    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");

    // Three colors share the `r` prefix (red, rose, ruby) and
    // one doesn't (blue). The half-open range `[r, s)` should
    // hit the three `r*` colors and miss `blue` entirely.
    //   red ×2, rose ×3, ruby ×1, blue ×4 → 6 in-range docs
    // across 3 distinct values.
    for (i, color) in [
        "red", "red", "rose", "rose", "rose", "ruby", "blue", "blue", "blue", "blue",
    ]
    .iter()
    .enumerate()
    {
        let doc = build_widget_doc(&contract, color, "small", (i + 1) as u64);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
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
            .expect("insert document");
    }

    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::StartsWith,
        value: dpp::platform_value::Value::Text("r".to_string()),
    }];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("picker accepts StartsWith");
    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses,
    };

    // Mode 1: no-proof aggregate. red(2) + rose(3) + ruby(1) = 6.
    let summed = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: false,
                limit: None,
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("no-proof aggregate over StartsWith");
    assert_eq!(summed.len(), 1, "summed mode → one entry");
    assert!(summed[0].key.is_empty(), "summed entry has empty key");
    assert_eq!(
        summed[0].count,
        Some(6),
        "color startsWith 'r' should sum to 2 (red) + 3 (rose) + 1 (ruby) = 6"
    );

    // Mode 2: no-proof distinct. Per-distinct-value entries,
    // ascending. red < rose < ruby alphabetically.
    let split = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: true,
                limit: None,
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("no-proof distinct over StartsWith");
    assert_eq!(
        split.len(),
        3,
        "distinct mode → one entry per matching color"
    );
    assert_eq!(split[0].key, b"red".to_vec());
    assert_eq!(split[0].count, Some(2));
    assert_eq!(split[1].key, b"rose".to_vec());
    assert_eq!(split[1].count, Some(3));
    assert_eq!(split[2].key, b"ruby".to_vec());
    assert_eq!(split[2].count, Some(1));

    // Mode 3: prove aggregate. Verifies via
    // `GroveDb::verify_aggregate_count_query` against the path
    // query the SDK would rebuild — same shape the existing `>`
    // prove tests use, just with a half-open `[r, s)` range
    // instead of `(b, ∞)`.
    let proof_bytes = query
        .execute_aggregate_count_with_proof(&drive, None, pv)
        .expect("aggregate count proof over StartsWith");
    let path_query = query
        .aggregate_count_path_query(pv)
        .expect("aggregate path query builds for StartsWith");
    let (root_hash, count) =
        GroveDb::verify_aggregate_count_query(&proof_bytes, &path_query, &pv.drive.grove_version)
            .expect("aggregate-count proof should verify");
    assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
    assert_eq!(
        count, 6,
        "verified aggregate count should match no-proof sum"
    );

    // Mode 4: prove distinct. The KVCount ops in the leaf merk
    // proof carry per-key counts bound to the merk root via
    // `node_hash_with_count`. Verify with standard `verify_query`
    // (matching the docs handler / distinct verifier pattern).
    const TEST_LIMIT: u16 = crate::config::DEFAULT_QUERY_LIMIT;
    let proof_bytes = query
        .execute_distinct_count_with_proof(&drive, TEST_LIMIT, true, None, pv)
        .expect("distinct count proof over StartsWith");
    assert!(
        !proof_bytes.is_empty(),
        "distinct count proof must not be empty"
    );
    let path_query = query
        .distinct_count_path_query(Some(TEST_LIMIT), true, pv)
        .expect("distinct path query builds for StartsWith");
    let (root_hash, _elements) =
        GroveDb::verify_query(&proof_bytes, &path_query, &pv.drive.grove_version)
            .expect("distinct-count proof should verify");
    assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
}

/// Empty `startsWith` prefix: `encode_value_for_tree_keys` maps
/// `Value::Text("")` to `[0]` (the explicit empty-string
/// sentinel — see `DocumentPropertyType::String`'s arm in
/// `packages/rs-dpp/src/data_contract/document_type/property/mod.rs`,
/// "we don't want to collide with the definition of an empty
/// string"). The half-open range becomes `[[0], [1])`, which
/// matches the empty-string sentinel value itself but nothing
/// else. Since no widget in this fixture has `color = ""` the
/// result is a successful sum of `0` — verifying the executor
/// reaches the count walk rather than panicking on the
/// `last_mut()` branch.
///
/// The `last_mut().ok_or(InvalidStartsWithClause)` branch in
/// `range_clause_to_query_item` is unreachable in practice
/// through this entry point because the empty-string sentinel
/// produces a non-empty serialized buffer; the check is purely
/// defense-in-depth against future encoding changes.
#[test]
fn range_count_executor_accepts_empty_starts_with_prefix_via_sentinel() {
    use crate::query::{DriveDocumentCountQuery, RangeCountOptions, WhereClause, WhereOperator};

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(false);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply contract");

    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::StartsWith,
        value: dpp::platform_value::Value::Text(String::new()),
    }];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("picker accepts StartsWith with any value");
    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses,
    };

    let result = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: false,
                limit: None,
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("empty startsWith prefix should succeed (matches empty-string sentinel only)");
    assert_eq!(result.len(), 1, "summed mode → one entry");
    assert_eq!(
        result[0].count,
        Some(0),
        "no docs have color = empty-string sentinel"
    );
}

// -------- Aggregate-count prove-path coverage helpers ----------
//
// The existing `aggregate_count_proof_verifies_and_returns_correct_count`
// tests exactly one operator (`>` → grovedb's `RangeAfter`). The
// remaining 7 mapped operator shapes
// (`>=`/`<`/`<=`/`between`/`betweenExcludeBounds`/
// `betweenExcludeLeft`/`betweenExcludeRight`) all generate
// structurally different `QueryItem` variants and exercise
// different `Disjoint`/`Contained`/`Boundary` classifications in
// grovedb's `prove_aggregate_count_on_range` walk. Each is its own
// potential regression site even though all share the same
// platform-side path-builder. The helpers + per-operator tests
// below close that gap.

/// Single-byColor fixture with 5 distinct color values
/// (`a`..`e`, two docs each — 10 docs total) so range tests can
/// land Disjoint, Contained, and Boundary classifications across
/// the AVL tree without carrying contract setup duplication.
fn setup_widget_with_5_colors_2_docs_each() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_widget_with_color_index(false);

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply contract");

    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");

    let mut seed = 1u64;
    for color in ["a", "b", "c", "d", "e"] {
        for _ in 0..2 {
            let doc = build_widget_doc(&contract, color, "small", seed);
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
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
            seed += 1;
        }
    }

    (drive, contract)
}

/// Prove-path roundtrip helper: builds the path query via the
/// shared `aggregate_count_path_query` (the same path the prover
/// internally uses), generates the proof, verifies it via
/// grovedb's `verify_aggregate_count_query`, and asserts the
/// recovered count equals `expected_count`. Reusing the
/// path-builder rather than hand-coding the path matches the SDK's
/// runtime flow — a divergence between prover and verifier
/// path-construction would surface here as a verification failure.
fn assert_aggregate_count_proof_returns(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    where_clauses: Vec<crate::query::WhereClause>,
    expected_count: u64,
) {
    use crate::query::DriveDocumentCountQuery;
    use grovedb::GroveDb;

    let pv = PlatformVersion::latest();
    let document_type = contract
        .document_type_for_name(document_type_name)
        .expect("document type exists");
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("range_countable index should be picked");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: document_type_name.to_string(),
        index,
        where_clauses,
    };

    let proof_bytes = query
        .execute_aggregate_count_with_proof(drive, None, pv)
        .expect("should generate aggregate count proof");
    assert!(!proof_bytes.is_empty(), "proof must not be empty");

    let path_query = query
        .aggregate_count_path_query(pv)
        .expect("aggregate_count_path_query should build");

    let (root_hash, count) =
        GroveDb::verify_aggregate_count_query(&proof_bytes, &path_query, &pv.drive.grove_version)
            .expect("aggregate-count proof should verify");
    assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
    assert_eq!(
        count, expected_count,
        "verified count should equal expected count"
    );
}

/// `>=` → grovedb `RangeFrom`. Lower bound inclusive, no upper
/// bound. Differs from `>` (RangeAfter) in whether the bound key
/// itself contributes — both share the same one-sided-from-below
/// AVL walk shape so this also serves as the regression for the
/// inclusivity bit.
#[test]
fn aggregate_count_proof_verifies_lower_bound_inclusive_ge() {
    use crate::query::{WhereClause, WhereOperator};

    let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::GreaterThanOrEquals,
        value: dpp::platform_value::Value::Text("c".to_string()),
    }];
    // c, d, e each have 2 docs; a, b excluded → 6.
    assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 6);
}

/// `<` → grovedb `RangeTo`. Upper bound strict, no lower bound.
/// Pins the one-sided-from-above walk shape; without this we'd
/// only ever exercise the symmetric `RangeAfter` half.
#[test]
fn aggregate_count_proof_verifies_upper_bound_strict_lt() {
    use crate::query::{WhereClause, WhereOperator};

    let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::LessThan,
        value: dpp::platform_value::Value::Text("c".to_string()),
    }];
    // a, b each have 2 docs; c, d, e excluded → 4.
    assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 4);
}

/// `<=` → grovedb `RangeToInclusive`. Pins the upper-bound
/// inclusivity bit on the from-above shape.
#[test]
fn aggregate_count_proof_verifies_upper_bound_inclusive_le() {
    use crate::query::{WhereClause, WhereOperator};

    let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::LessThanOrEquals,
        value: dpp::platform_value::Value::Text("c".to_string()),
    }];
    // a, b, c each have 2 docs; d, e excluded → 6.
    assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 6);
}

/// `between` → grovedb `RangeInclusive` (closed-closed). The most
/// common two-sided range shape; both bounds are matched.
#[test]
fn aggregate_count_proof_verifies_between_closed_closed() {
    use crate::query::{WhereClause, WhereOperator};

    let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::Between,
        value: dpp::platform_value::Value::Array(vec![
            dpp::platform_value::Value::Text("b".to_string()),
            dpp::platform_value::Value::Text("d".to_string()),
        ]),
    }];
    // b, c, d each have 2 docs → 6.
    assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 6);
}

/// `betweenExcludeBounds` → grovedb `RangeAfterTo` (open-open).
/// Both bounds are excluded — the only `between*` variant where
/// neither bound key contributes.
#[test]
fn aggregate_count_proof_verifies_between_open_open() {
    use crate::query::{WhereClause, WhereOperator};

    let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::BetweenExcludeBounds,
        value: dpp::platform_value::Value::Array(vec![
            dpp::platform_value::Value::Text("a".to_string()),
            dpp::platform_value::Value::Text("d".to_string()),
        ]),
    }];
    // b, c each have 2 docs (a excluded as lower, d excluded as
    // upper) → 4.
    assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 4);
}

/// `betweenExcludeLeft` → grovedb `RangeAfterToInclusive`
/// (open-closed). Lower excluded, upper included.
#[test]
fn aggregate_count_proof_verifies_between_open_closed() {
    use crate::query::{WhereClause, WhereOperator};

    let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::BetweenExcludeLeft,
        value: dpp::platform_value::Value::Array(vec![
            dpp::platform_value::Value::Text("a".to_string()),
            dpp::platform_value::Value::Text("c".to_string()),
        ]),
    }];
    // b, c each have 2 docs (a excluded as lower) → 4.
    assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 4);
}

/// `betweenExcludeRight` → grovedb `Range` (closed-open). Lower
/// included, upper excluded — the conventional half-open range.
#[test]
fn aggregate_count_proof_verifies_between_closed_open() {
    use crate::query::{WhereClause, WhereOperator};

    let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::BetweenExcludeRight,
        value: dpp::platform_value::Value::Array(vec![
            dpp::platform_value::Value::Text("b".to_string()),
            dpp::platform_value::Value::Text("d".to_string()),
        ]),
    }];
    // b, c each have 2 docs (d excluded as upper) → 4.
    assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 4);
}

/// Empty range: zero matching keys must still produce a valid
/// proof with count = 0. This is the boundary case where every
/// subtree is `Disjoint` from the inner range — grovedb's prover
/// short-circuits at every link without descending. The verifier
/// must accept this proof shape and recover count = 0 (not error
/// "no items in range"). Without this test a regression that made
/// empty proofs fail would only surface at customer time.
#[test]
fn aggregate_count_proof_verifies_empty_range_returns_zero() {
    use crate::query::{WhereClause, WhereOperator};

    let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::GreaterThan,
        value: dpp::platform_value::Value::Text("z".to_string()),
    }];
    // No colors > "z" — count = 0.
    assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 0);
}

/// Compound `[brand, color]` range_countable index, prove path:
/// the `Equal`-on-brand prefix becomes path bytes (not a query
/// shape), and only the terminator `color > X` becomes the merk
/// `AggregateCountOnRange` walk. This exercises grovedb's multi-
/// layer aggregate-count proof envelope: the verifier walks
/// through one non-leaf layer (the `brand=acme` value tree's
/// existence proof) before reaching the leaf merk's count proof.
/// The single-property tests above all run at the top property-
/// name layer directly so they don't reach this code path.
#[test]
fn aggregate_count_proof_verifies_on_compound_index_with_equal_prefix() {
    use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
    use dpp::platform_value::Value;
    use grovedb::GroveDb;

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();

    // Build a contract with `[brand, color]` range_countable.
    // Same shape as `range_count_with_in_on_prefix_forks_and_merges`
    // uses, but here we exercise the prove path instead of the
    // no-proof executor.
    let factory = dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12)
        .expect("expected to create factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "brand": { "type": "string", "position": 0, "maxLength": 32 },
            "color": { "type": "string", "position": 1, "maxLength": 32 },
        },
        "indices": [{
            "name": "byBrandColor",
            "properties": [{"brand": "asc"}, {"color": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "widget": document_schema });
    let contract = factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned();

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply contract");

    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");

    // acme: red×3, blue×2; contoso: red×2, green×1, blue×1.
    // Query: brand = acme AND color > "blue" → 3 (acme reds).
    let docs: &[(&str, &str)] = &[
        ("acme", "red"),
        ("acme", "red"),
        ("acme", "red"),
        ("acme", "blue"),
        ("acme", "blue"),
        ("contoso", "red"),
        ("contoso", "red"),
        ("contoso", "green"),
        ("contoso", "blue"),
    ];
    for (i, (brand, color)) in docs.iter().enumerate() {
        let mut doc = document_type
            .random_document(Some((i + 1) as u64), pv)
            .expect("random document");
        let mut props = std::collections::BTreeMap::new();
        props.insert("brand".to_string(), Value::Text(brand.to_string()));
        props.insert("color".to_string(), Value::Text(color.to_string()));
        doc.set_properties(props);

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
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

    let where_clauses = vec![
        WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("acme".to_string()),
        },
        WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        },
    ];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("compound range_countable index should be picked");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses,
    };

    let proof_bytes = query
        .execute_aggregate_count_with_proof(&drive, None, pv)
        .expect("should generate aggregate count proof");
    assert!(!proof_bytes.is_empty(), "proof must not be empty");

    let path_query = query
        .aggregate_count_path_query(pv)
        .expect("compound aggregate_count_path_query should build");

    let (root_hash, count) =
        GroveDb::verify_aggregate_count_query(&proof_bytes, &path_query, &pv.drive.grove_version)
            .expect(
                "compound aggregate-count proof should verify (multi-layer \
         envelope walk through brand=acme to color leaf merk)",
            );
    assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
    assert_eq!(
        count, 3,
        "verified count should be 3 (acme reds; acme blues excluded by `> blue`)"
    );
}

/// Scale test for the `AggregateCountOnRange` proof primitive at
/// non-trivial fan-out: a parking-lot contract with one document
/// per car, each tagged with its lot letter (`a`..`z`). Lot `a`
/// has 1 car, `b` has 2, ..., `z` has 26 — total `1+2+...+26 =
/// 351` cars across 26 distinct lot values.
///
/// Question: how many cars are in parking lots > b?
/// Answer: cars in lots `c..=z` = `3+4+...+26` = 348.
///
/// Why this test earns its keep on top of the operator-shape
/// matrix above:
///
/// 1. **Wide range** — 24 of 26 distinct values are in-range, so
///    grovedb's prover walks the AVL tree end-to-end and
///    classifies most subtrees as `Contained` (one-level kv_hash
///    + grandchild-hash visit) rather than `Boundary` (recurse).
///    The narrow ranges in the operator-shape tests don't
///    exercise this regime.
/// 2. **Realistic per-key fan-out** — multi-doc lots (b=2, c=3,
///    …, z=26) mean each value tree is a non-trivial CountTree
///    with internal counts > 1. The aggregate count must sum
///    those internal counts correctly, not just count keys.
/// 3. **The proof stays O(log n)** even though the answer is 348
///    — the verifier never sees the underlying 348 documents,
///    only the merk-level count proof. That's the whole reason
///    the aggregate primitive exists vs. the materialize-and-
///    count fallback.
#[test]
fn aggregate_count_proof_counts_cars_in_parking_lots_greater_than_b() {
    use crate::query::{WhereClause, WhereOperator};
    use dpp::platform_value::Value;

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();

    // parking-lot contract: one `car` document type with a `byLot`
    // range_countable index on the `lot` property. Single-property
    // index keeps the path-builder at the top property-name layer
    // (the leaf-merk count proof is the whole envelope here).
    let factory = dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12)
        .expect("expected to create factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "lot": { "type": "string", "position": 0, "maxLength": 4 },
        },
        "indices": [{
            "name": "byLot",
            "properties": [{"lot": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "car": document_schema });
    let contract = factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .expect("create parking-lot contract")
        .data_contract_owned();

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply parking-lot contract");

    let document_type = contract
        .document_type_for_name("car")
        .expect("car document type exists");

    // Insert N cars for each lot, where N = lot's 1-based
    // position in the alphabet (a → 1, b → 2, …, z → 26).
    let mut seed = 1u64;
    for (idx, letter) in ('a'..='z').enumerate() {
        let car_count = idx + 1;
        for _ in 0..car_count {
            let mut doc = document_type
                .random_document(Some(seed), pv)
                .expect("random document");
            let mut props = std::collections::BTreeMap::new();
            props.insert("lot".to_string(), Value::Text(letter.to_string()));
            doc.set_properties(props);

            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
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
                .expect("expected to insert car document");
            seed += 1;
        }
    }

    // Quick math check on the closed-form expected count so a
    // future reader doesn't have to recompute the sum to follow
    // the assertion.
    let expected: u64 = (3..=26).sum();
    assert_eq!(
        expected, 348,
        "sanity check: cars in lots c..=z = 3 + 4 + … + 26 = 348"
    );

    // The actual scenario: how many cars are in parking lots > b?
    let where_clauses = vec![WhereClause {
        field: "lot".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("b".to_string()),
    }];

    use crate::query::DriveDocumentCountQuery;
    use grovedb::GroveDb;
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("byLot range_countable index should be picked");
    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "car".to_string(),
        index,
        where_clauses,
    };

    let proof_bytes = query
        .execute_aggregate_count_with_proof(&drive, None, pv)
        .expect("should generate aggregate count proof");

    let path_query = query
        .aggregate_count_path_query(pv)
        .expect("path query should build");

    let (root_hash, count) =
        GroveDb::verify_aggregate_count_query(&proof_bytes, &path_query, &pv.drive.grove_version)
            .expect("aggregate-count proof should verify");

    // Inline-print under `cargo test -- --nocapture`. The
    // envelope walk decodes the bincode-wrapped `GroveDBProof`,
    // then for each layer's merk proof bytes uses
    // `MerkProofDecoder` to print the per-layer Op stream. This
    // is the same decoding the verifier above performed
    // internally — surfacing it makes the O(log n) shape concrete
    // (the leaf merk proof for `lot > "b"` is ~700 bytes
    // regardless of how many of the 351 cars are in-range).
    use grovedb::operations::proof::{
        GroveDBProof, GroveDBProofV0, GroveDBProofV1, LayerProof, MerkOnlyLayerProof, ProofBytes,
    };
    use grovedb::{MerkProofDecoder, MerkProofOp};

    fn label_path_segment(key: &[u8]) -> String {
        // Path keys are mostly small ascii, but the contract-id
        // bytes and the `[1]` doctype-table marker aren't —
        // hex-encode anything non-printable.
        if key.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
            format!("\"{}\"", String::from_utf8_lossy(key))
        } else {
            format!("0x{}", hex::encode(key))
        }
    }

    fn print_ops(label: &str, depth: usize, merk_bytes: &[u8]) {
        let indent = "  ".repeat(depth);
        println!(
            "{}{} (merk_proof = {} bytes)",
            indent,
            label,
            merk_bytes.len()
        );
        for (i, op_res) in MerkProofDecoder::new(merk_bytes).enumerate() {
            match op_res {
                Ok(MerkProofOp::Push(n)) => println!("{}  [{:>2}] Push({})", indent, i, n),
                Ok(MerkProofOp::PushInverted(n)) => {
                    println!("{}  [{:>2}] PushInverted({})", indent, i, n)
                }
                Ok(MerkProofOp::Parent) => println!("{}  [{:>2}] Parent", indent, i),
                Ok(MerkProofOp::Child) => println!("{}  [{:>2}] Child", indent, i),
                Ok(MerkProofOp::ParentInverted) => {
                    println!("{}  [{:>2}] ParentInverted", indent, i)
                }
                Ok(MerkProofOp::ChildInverted) => {
                    println!("{}  [{:>2}] ChildInverted", indent, i)
                }
                Err(e) => println!("{}  [{:>2}] <decode error: {}>", indent, i, e),
            }
        }
    }

    fn walk_v0(layer: &MerkOnlyLayerProof, depth: usize, label: String) {
        print_ops(&label, depth, &layer.merk_proof);
        for (k, lower) in &layer.lower_layers {
            walk_v0(
                lower,
                depth + 1,
                format!(
                    "layer @ depth {} (path key {})",
                    depth + 1,
                    label_path_segment(k)
                ),
            );
        }
    }

    fn walk_v1(layer: &LayerProof, depth: usize, label: String) {
        let bytes = match &layer.merk_proof {
            ProofBytes::Merk(b) => b.as_slice(),
            _ => {
                println!(
                    "{}{}: <non-merk leaf bytes — unexpected for aggregate-count>",
                    "  ".repeat(depth),
                    label
                );
                return;
            }
        };
        print_ops(&label, depth, bytes);
        for (k, lower) in &layer.lower_layers {
            walk_v1(
                lower,
                depth + 1,
                format!(
                    "layer @ depth {} (path key {})",
                    depth + 1,
                    label_path_segment(k)
                ),
            );
        }
    }

    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<{ 256 * 1024 * 1024 }>();
    let (envelope, _): (GroveDBProof, _) =
        bincode::decode_from_slice(&proof_bytes, config).expect("envelope decodes");

    println!("=== parking-lot aggregate-count proof ===");
    println!("inserted docs: 351 (1 + 2 + ... + 26)");
    println!("query: lot > \"b\"");
    println!("verified count: {}", count);
    println!("verified root hash: {}", hex::encode(root_hash));
    println!("envelope size: {} bytes", proof_bytes.len());

    match envelope {
        GroveDBProof::V0(GroveDBProofV0 { root_layer, .. }) => {
            walk_v0(&root_layer, 0, "layer @ depth 0 (root)".to_string())
        }
        GroveDBProof::V1(GroveDBProofV1 { root_layer }) => {
            walk_v1(&root_layer, 0, "layer @ depth 0 (root)".to_string())
        }
    }
    println!("=== end proof ===");

    assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
    assert_eq!(
        count, expected,
        "expected {} cars in parking lots > b (sum of 3+4+...+26)",
        expected
    );
}

/// Same parking-lot fixture as the prove-path scenario, but
/// asking the no-proof distinct-mode executor for *per-lot*
/// counts in the same range. Where the aggregate-count proof
/// returns one number (348 = total cars in lots > b), distinct
/// mode walks the property-name `ProvableCountTree` and emits
/// one entry per distinct in-range value:
/// `c=3, d=4, e=5, ..., z=26`.
///
/// No-proof companion to the aggregate-count proof path: the
/// `AggregateCountOnRange` merk primitive returns a single u64,
/// so getting per-distinct-value counts requires the executor to
/// walk the children of the property-name tree directly. That
/// walk is cheaper than the materialize-and-count fallback (no
/// documents are loaded), but isn't cryptographically committed
/// by a single proof shape on the prove + non-distinct path —
/// see `book/src/drive/document-count-trees.md` for the
/// prove-vs-no-proof matrix.
///
/// The fixture is identical to
/// `aggregate_count_proof_counts_cars_in_parking_lots_greater_than_b`
/// — duplicating the setup keeps each test independently
/// runnable rather than introducing a fragile shared-fixture
/// helper.
#[test]
fn range_count_executor_returns_per_lot_counts_for_lots_greater_than_b() {
    use crate::query::{DriveDocumentCountQuery, RangeCountOptions, WhereClause, WhereOperator};
    use dpp::platform_value::Value;

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();

    let factory = dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12)
        .expect("expected to create factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "lot": { "type": "string", "position": 0, "maxLength": 4 },
        },
        "indices": [{
            "name": "byLot",
            "properties": [{"lot": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "car": document_schema });
    let contract = factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .expect("create parking-lot contract")
        .data_contract_owned();

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply parking-lot contract");

    let document_type = contract
        .document_type_for_name("car")
        .expect("car document type exists");

    let mut seed = 1u64;
    for (idx, letter) in ('a'..='z').enumerate() {
        let car_count = idx + 1;
        for _ in 0..car_count {
            let mut doc = document_type
                .random_document(Some(seed), pv)
                .expect("random document");
            let mut props = std::collections::BTreeMap::new();
            props.insert("lot".to_string(), Value::Text(letter.to_string()));
            doc.set_properties(props);

            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
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
                .expect("expected to insert car document");
            seed += 1;
        }
    }

    // Range query: `lot > "b"` (same predicate as the prove
    // test). Distinct mode → one entry per distinct in-range
    // value, each carrying that lot's car count.
    let where_clauses = vec![WhereClause {
        field: "lot".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("b".to_string()),
    }];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("byLot range_countable index should be picked");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "car".to_string(),
        index,
        where_clauses,
    };

    let entries = query
        .execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: true,
                limit: None,
                order_by_ascending: true,
            },
            None,
            pv,
        )
        .expect("distinct-range count should succeed");

    // 24 distinct lots in range (c through z).
    assert_eq!(
        entries.len(),
        24,
        "expected one entry per lot from c through z"
    );

    // Each entry: lot letter (as serialized key bytes) → its
    // alphabet-position car count. Ascending serialized-key
    // order matches alphabetical order for ASCII single chars.
    for (i, entry) in entries.iter().enumerate() {
        let expected_letter = (b'c' + i as u8) as char;
        let expected_count = (i + 3) as u64; // c → 3, d → 4, …, z → 26
        assert_eq!(
            entry.key,
            expected_letter.to_string().as_bytes().to_vec(),
            "entry {} should be lot '{}'",
            i,
            expected_letter
        );
        assert_eq!(
            entry.count,
            Some(expected_count),
            "lot '{}' should have {} cars",
            expected_letter,
            expected_count
        );
    }

    // Sum-check: per-lot counts must total the prove-path
    // aggregate (348). Different code path, same answer — the
    // distinct walk and the merk-level aggregate are obligated
    // to agree.
    let total: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
    assert_eq!(
        total, 348,
        "sum of per-lot counts must equal the aggregate (3+4+...+26 = 348)"
    );
}

/// The trustless companion to the no-proof distinct test above:
/// same parking-lot fixture, same `lot > "b"` predicate, asking
/// for *per-lot* counts but this time via the prove path. Returns
/// a regular grovedb range proof against the property-name
/// `ProvableCountTree` — no `AggregateCountOnRange` wrapper.
/// merk's `to_kv_count_node` emits one `Node::KVCount(key, value,
/// count)` per matched in-range key, each `count` bound to the
/// merk root via `node_hash_with_count`, and we recover the
/// per-key map by walking the proof's op stream after the
/// standard hash-chain check passes.
///
/// Pinned guarantees:
/// 1. Per-lot counts match the no-proof distinct walk exactly
///    (cross-checked against the `range_count_executor_returns
///    _per_lot_counts_for_lots_greater_than_b` expectations).
/// 2. The recovered counts sum to 348 — same answer the
///    aggregate prove path produces, just decomposed per-key.
///    All three code paths (no-proof distinct, prove aggregate,
///    prove distinct) are obligated to agree.
/// 3. The proof never materializes the underlying 348 documents.
///    Total proof bytes scale with O(distinct lots in range)
///    rather than O(matched docs), proving the
///    "doesn't-materialize-docs" win that distinguishes this
///    from the materialize-and-count fallback.
#[test]
fn distinct_count_proof_returns_per_lot_counts_for_lots_greater_than_b() {
    use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
    use dpp::platform_value::Value;
    use grovedb::GroveDb;

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();

    let factory = dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12)
        .expect("expected to create factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "lot": { "type": "string", "position": 0, "maxLength": 4 },
        },
        "indices": [{
            "name": "byLot",
            "properties": [{"lot": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "car": document_schema });
    let contract = factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .expect("create parking-lot contract")
        .data_contract_owned();

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply parking-lot contract");

    let document_type = contract
        .document_type_for_name("car")
        .expect("car document type exists");

    let mut seed = 1u64;
    for (idx, letter) in ('a'..='z').enumerate() {
        let car_count = idx + 1;
        for _ in 0..car_count {
            let mut doc = document_type
                .random_document(Some(seed), pv)
                .expect("random document");
            let mut props = std::collections::BTreeMap::new();
            props.insert("lot".to_string(), Value::Text(letter.to_string()));
            doc.set_properties(props);

            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
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
                .expect("expected to insert car document");
            seed += 1;
        }
    }

    let where_clauses = vec![WhereClause {
        field: "lot".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("b".to_string()),
    }];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("byLot range_countable index should be picked");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "car".to_string(),
        index,
        where_clauses,
    };

    // Prove side: no `AggregateCountOnRange` wrapper. Use the
    // shared `DEFAULT_QUERY_LIMIT` so the test exercises the
    // same default the dispatcher would apply when a client
    // omits `limit`. 24 distinct lots fit comfortably under
    // 100 so all entries land in the proof.
    const TEST_LIMIT: u16 = crate::config::DEFAULT_QUERY_LIMIT;
    let proof_bytes = query
        .execute_distinct_count_with_proof(&drive, TEST_LIMIT, true, None, pv)
        .expect("should generate distinct count proof");
    assert!(!proof_bytes.is_empty(), "proof must not be empty");

    // Verify side: standard verify_query gives us the integrity
    // check + root_hash. The per-lot counts inside the proof are
    // bound to root_hash via node_hash_with_count, so once this
    // returns we just read each element's count.
    let path_query = query
        .distinct_count_path_query(Some(TEST_LIMIT), true, pv)
        .expect("path query should build");

    // Mirror the normal docs query's verify pattern: `verify_query`
    // (strict succinctness, no absence-proof requirement) — see
    // `DriveDocumentQuery::verify_proof_keep_serialized_v0`. The
    // `verify_query_with_options` default has
    // `absence_proofs_for_non_existing_searched_keys: true` which
    // can't handle unbounded ranges like `lot > "b"`; this helper
    // doesn't.
    let (root_hash, _elements) =
        GroveDb::verify_query(&proof_bytes, &path_query, &pv.drive.grove_version)
            .expect("standard verify_query must succeed for the regular range proof shape");
    assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");

    // Walk the envelope down to the leaf merk and pluck per-lot
    // counts. Mirrors verify_distinct_count_proof's extraction.
    // At the property-name `ProvableCountTree` layer each child's
    // value is a serialized `Element::CountTree(_, lot_count, _)`
    // pointing to that lot's value-CountTree; we deserialize the
    // value bytes and read `count_value_or_default()` for the per-
    // lot count. The AVL-aggregate count carried by the
    // `ProvableCountedMerkNode(_)` feature type is the *wrong*
    // number — it includes left/right AVL-subtree contributions,
    // not just this lot.
    use grovedb::operations::proof::{GroveDBProof, GroveDBProofV0, GroveDBProofV1, ProofBytes};
    use grovedb::{Element, MerkProofDecoder, MerkProofNode, MerkProofOp};
    use std::collections::BTreeMap;

    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<{ 256 * 1024 * 1024 }>();
    let (envelope, _): (GroveDBProof, _) =
        bincode::decode_from_slice(&proof_bytes, config).expect("envelope decodes");

    let mut counts: BTreeMap<Vec<u8>, u64> = BTreeMap::new();
    let target_depth = path_query.path.len();

    let extract_per_lot = |merk_bytes: &[u8], counts: &mut BTreeMap<Vec<u8>, u64>| {
        for op in MerkProofDecoder::new(merk_bytes) {
            let (key, value) = match op {
                Ok(MerkProofOp::Push(MerkProofNode::KVValueHashFeatureType(key, value, _, _))) => {
                    (key, value)
                }
                Ok(MerkProofOp::Push(MerkProofNode::KVValueHashFeatureTypeWithChildHash(
                    key,
                    value,
                    _,
                    _,
                    _,
                ))) => (key, value),
                Ok(MerkProofOp::Push(MerkProofNode::KVCount(key, value, _))) => (key, value),
                _ => continue,
            };
            let elem = Element::deserialize(&value, &pv.drive.grove_version)
                .expect("element value should deserialize");
            counts.insert(key, elem.count_value_or_default());
        }
    };

    match envelope {
        GroveDBProof::V0(GroveDBProofV0 { root_layer, .. }) => {
            let mut layer = &root_layer;
            let mut depth = 0;
            while depth < target_depth {
                let next_key = &path_query.path[depth];
                layer = layer
                    .lower_layers
                    .get(next_key)
                    .expect("lower layer must exist for each path key");
                depth += 1;
            }
            extract_per_lot(&layer.merk_proof, &mut counts);
        }
        GroveDBProof::V1(GroveDBProofV1 { root_layer }) => {
            let mut layer = &root_layer;
            let mut depth = 0;
            while depth < target_depth {
                let next_key = &path_query.path[depth];
                layer = layer
                    .lower_layers
                    .get(next_key)
                    .expect("lower layer must exist for each path key");
                depth += 1;
            }
            let merk_bytes = match &layer.merk_proof {
                ProofBytes::Merk(b) => b.as_slice(),
                _ => panic!("unexpected non-merk leaf bytes for distinct-count proof"),
            };
            extract_per_lot(merk_bytes, &mut counts);
        }
    }

    // Inline-print under `cargo test -- --nocapture`. Mirrors the
    // aggregate test's print-decoded-proof block but for the
    // distinct shape: matched children show up as
    // `KVValueHashFeatureType[WithChildHash]` ops carrying the
    // encoded `Element::CountTree(_, lot_count, _)` value plus
    // the AVL-aggregate `ProvableCountedMerkNode(_)` feature
    // count. Side-by-side comparison with the aggregate proof
    // makes the size/shape trade-off visible.
    fn label_path_segment(key: &[u8]) -> String {
        if key.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
            format!("\"{}\"", String::from_utf8_lossy(key))
        } else {
            format!("0x{}", hex::encode(key))
        }
    }
    fn print_ops(label: &str, depth: usize, merk_bytes: &[u8]) {
        let indent = "  ".repeat(depth);
        println!(
            "{}{} (merk_proof = {} bytes)",
            indent,
            label,
            merk_bytes.len()
        );
        for (i, op_res) in MerkProofDecoder::new(merk_bytes).enumerate() {
            match op_res {
                Ok(MerkProofOp::Push(n)) => println!("{}  [{:>2}] Push({})", indent, i, n),
                Ok(MerkProofOp::PushInverted(n)) => {
                    println!("{}  [{:>2}] PushInverted({})", indent, i, n)
                }
                Ok(MerkProofOp::Parent) => println!("{}  [{:>2}] Parent", indent, i),
                Ok(MerkProofOp::Child) => println!("{}  [{:>2}] Child", indent, i),
                Ok(MerkProofOp::ParentInverted) => {
                    println!("{}  [{:>2}] ParentInverted", indent, i)
                }
                Ok(MerkProofOp::ChildInverted) => {
                    println!("{}  [{:>2}] ChildInverted", indent, i)
                }
                Err(e) => println!("{}  [{:>2}] <decode error: {}>", indent, i, e),
            }
        }
    }
    fn walk_v0_print(
        layer: &grovedb::operations::proof::MerkOnlyLayerProof,
        depth: usize,
        label: String,
    ) {
        print_ops(&label, depth, &layer.merk_proof);
        for (k, lower) in &layer.lower_layers {
            walk_v0_print(
                lower,
                depth + 1,
                format!(
                    "layer @ depth {} (path key {})",
                    depth + 1,
                    label_path_segment(k)
                ),
            );
        }
    }
    fn walk_v1_print(layer: &grovedb::operations::proof::LayerProof, depth: usize, label: String) {
        let bytes = match &layer.merk_proof {
            ProofBytes::Merk(b) => b.as_slice(),
            _ => {
                println!(
                    "{}{}: <non-merk leaf bytes — unexpected for distinct-count>",
                    "  ".repeat(depth),
                    label
                );
                return;
            }
        };
        print_ops(&label, depth, bytes);
        for (k, lower) in &layer.lower_layers {
            walk_v1_print(
                lower,
                depth + 1,
                format!(
                    "layer @ depth {} (path key {})",
                    depth + 1,
                    label_path_segment(k)
                ),
            );
        }
    }
    let (envelope_for_print, _): (GroveDBProof, _) =
        bincode::decode_from_slice(&proof_bytes, config).expect("envelope decodes");

    println!("=== parking-lot DISTINCT-count proof ===");
    println!("inserted docs: 351 (1 + 2 + ... + 26)");
    println!("query: lot > \"b\" (return_distinct_counts_in_range = true)");
    println!("verified per-lot count entries: {}", counts.len());
    println!("verified root hash: {}", hex::encode(root_hash));
    println!("envelope size: {} bytes", proof_bytes.len());
    match envelope_for_print {
        GroveDBProof::V0(GroveDBProofV0 { root_layer, .. }) => {
            walk_v0_print(&root_layer, 0, "layer @ depth 0 (root)".to_string())
        }
        GroveDBProof::V1(GroveDBProofV1 { root_layer }) => {
            walk_v1_print(&root_layer, 0, "layer @ depth 0 (root)".to_string())
        }
    }
    println!("=== end distinct proof ===");

    // 24 distinct lots (c..=z) each with their alphabet-position
    // count. Same expectation as the no-proof distinct test — the
    // prove path is obligated to return the same numbers, just
    // with cryptographic bounding on each.
    assert_eq!(
        counts.len(),
        24,
        "expected one entry per lot from c through z, got {}",
        counts.len()
    );
    for (i, letter) in ('c'..='z').enumerate() {
        let key = letter.to_string().into_bytes();
        let expected_count = (i + 3) as u64;
        assert_eq!(
            counts.get(&key).copied(),
            Some(expected_count),
            "lot '{}' should have {} cars",
            letter,
            expected_count
        );
    }

    // Cross-path agreement: per-lot sum equals the aggregate
    // proof's answer (348). Three code paths (no-proof distinct,
    // prove aggregate, prove distinct) all obligated to agree.
    let total: u64 = counts.values().sum();
    assert_eq!(
        total, 348,
        "sum of per-lot counts must equal aggregate (3+4+...+26 = 348)"
    );
}

/// `RangeDistinctProof` honors the request's `limit` field — the
/// path query carries `SizedQuery::limit = Some(N)` so the
/// prover bounds the proof at `N` matched keys. With `limit = 5`
/// over the 24-distinct-lots-in-range parking-lot fixture, the
/// verified proof should cover exactly the first 5 lots in
/// ascending order: `c, d, e, f, g`.
///
/// Pins two things at once: (1) the limit is plumbed end-to-end
/// through `execute_document_count_range_distinct_proof` →
/// `execute_distinct_count_with_proof` →
/// `distinct_count_path_query`, and (2) the prover and verifier
/// build the *exact same* `PathQuery` with that limit so the
/// merk-root recomputation matches.
#[test]
fn distinct_count_proof_honors_request_limit() {
    use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
    use dpp::platform_value::Value;
    use grovedb::{Element, GroveDb};

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let factory =
        dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12).expect("factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": { "lot": { "type": "string", "position": 0, "maxLength": 4 } },
        "indices": [{
            "name": "byLot",
            "properties": [{"lot": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "car": document_schema });
    let contract = factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("car").expect("car doctype");

    // 1 car per lot a..z = 26 docs; small fixture is fine since
    // we're only testing the limit, not per-lot counts.
    let mut seed = 1u64;
    for letter in 'a'..='z' {
        let mut doc = document_type
            .random_document(Some(seed), pv)
            .expect("random doc");
        let mut props = std::collections::BTreeMap::new();
        props.insert("lot".to_string(), Value::Text(letter.to_string()));
        doc.set_properties(props);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
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
            .expect("insert");
        seed += 1;
    }

    let where_clauses = vec![WhereClause {
        field: "lot".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("b".to_string()),
    }];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("byLot picked");
    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "car".to_string(),
        index,
        where_clauses,
    };

    const LIMIT: u16 = 5;
    let proof_bytes = query
        .execute_distinct_count_with_proof(&drive, LIMIT, true, None, pv)
        .expect("proof");
    let path_query = query
        .distinct_count_path_query(Some(LIMIT), true, pv)
        .expect("path query");

    let (root_hash, elements) =
        GroveDb::verify_query(&proof_bytes, &path_query, &pv.drive.grove_version).expect("verify");
    assert_ne!(root_hash, [0u8; 32]);

    // Proof should cover exactly LIMIT entries — the first 5 in
    // ascending key order: c, d, e, f, g.
    let keys: Vec<Vec<u8>> = elements
        .iter()
        .filter_map(|(_p, k, e)| e.as_ref().map(|_| k.clone()))
        .collect();
    assert_eq!(
        keys.len(),
        LIMIT as usize,
        "proof should cover exactly {} matched keys, got {}",
        LIMIT,
        keys.len()
    );
    assert_eq!(
        keys,
        vec![
            b"c".to_vec(),
            b"d".to_vec(),
            b"e".to_vec(),
            b"f".to_vec(),
            b"g".to_vec()
        ],
        "first {} matched keys in ascending order",
        LIMIT
    );

    // Spot-check that we can still recover the per-lot count
    // (everyone is 1 in this fixture).
    for (_p, _k, elem) in elements {
        let elem = elem.expect("matched element");
        assert_eq!(
            elem.count_value_or_default(),
            1,
            "each lot has exactly 1 doc in this fixture"
        );
        // Suppress unused-import if nothing else uses Element.
        let _: Element = elem;
    }
}

/// `order_by_ascending = false` on the prove-distinct path
/// flips grovedb's `Query.left_to_right` to `false`, so the
/// proof covers the last `limit` matched keys in descending
/// order instead of the first `limit` in ascending order.
///
/// Same parking-lot fixture as
/// [`distinct_count_proof_honors_request_limit`] (one car per
/// letter `a..=z`, queried with `lot > "b"` so 24 lots are
/// in-range). With `LIMIT = 5` and descending iteration the
/// proof should cover `z, y, x, w, v` — pinning that:
/// (1) `left_to_right = false` propagates end-to-end through
/// `execute_document_count_range_distinct_proof` →
/// `execute_distinct_count_with_proof` →
/// `distinct_count_path_query`;
/// (2) the prover and verifier agree on the descending path
/// query so the merk-root recomputation matches;
/// (3) descending order under `limit` is semantically
/// correct — we get the LAST `limit` keys, not the first
/// `limit` keys reversed (which would be `c, d, e, f, g`
/// reversed, i.e. `g, f, e, d, c` — wrong).
#[test]
fn distinct_count_proof_descending_returns_last_limit_keys() {
    use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
    use dpp::platform_value::Value;
    use grovedb::{Element, GroveDb};

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let factory =
        dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12).expect("factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": { "lot": { "type": "string", "position": 0, "maxLength": 4 } },
        "indices": [{
            "name": "byLot",
            "properties": [{"lot": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "car": document_schema });
    let contract = factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply parking-lot contract");
    let document_type = contract.document_type_for_name("car").expect("car");

    // One car per letter a..=z.
    let mut seed = 1u64;
    for letter in 'a'..='z' {
        let mut doc = document_type
            .random_document(Some(seed), pv)
            .expect("random doc");
        let mut props = std::collections::BTreeMap::new();
        props.insert("lot".to_string(), Value::Text(letter.to_string()));
        doc.set_properties(props);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
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
            .expect("insert");
        seed += 1;
    }

    let where_clauses = vec![WhereClause {
        field: "lot".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("b".to_string()),
    }];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("byLot picked");
    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "car".to_string(),
        index,
        where_clauses,
    };

    const LIMIT: u16 = 5;
    // left_to_right = false → descending. Both prove and verify
    // sides MUST pass the same value or the merk-root chain
    // check below fails.
    let proof_bytes = query
        .execute_distinct_count_with_proof(&drive, LIMIT, false, None, pv)
        .expect("proof");
    let path_query = query
        .distinct_count_path_query(Some(LIMIT), false, pv)
        .expect("path query");

    let (root_hash, elements) =
        GroveDb::verify_query(&proof_bytes, &path_query, &pv.drive.grove_version)
            .expect("descending path query must verify against the prover's proof");
    assert_ne!(root_hash, [0u8; 32]);

    // Proof should cover exactly LIMIT entries — the LAST 5 in
    // descending key order: z, y, x, w, v. Critically, NOT the
    // first 5 ascending reversed (that would be g, f, e, d, c).
    let keys: Vec<Vec<u8>> = elements
        .iter()
        .filter_map(|(_p, k, e)| e.as_ref().map(|_| k.clone()))
        .collect();
    assert_eq!(
        keys.len(),
        LIMIT as usize,
        "proof should cover exactly {} matched keys, got {}",
        LIMIT,
        keys.len()
    );
    assert_eq!(
        keys,
        vec![
            b"z".to_vec(),
            b"y".to_vec(),
            b"x".to_vec(),
            b"w".to_vec(),
            b"v".to_vec()
        ],
        "last {} matched keys in descending order",
        LIMIT
    );
    for (_p, _k, elem) in elements {
        let elem = elem.expect("matched element");
        assert_eq!(elem.count_value_or_default(), 1);
        let _: Element = elem;
    }
}

/// The dispatcher rejects `RangeDistinctProof` requests where
/// the effective limit exceeds `max_query_limit` rather than
/// silently clamping. Silent clamping would invisibly break
/// client-side proof reconstruction (the SDK builds its
/// `PathQuery` from `request.limit`, not from a server-clamped
/// value the SDK never sees), so the policy is to fail loudly.
#[test]
fn distinct_count_proof_rejects_limit_above_max_query_limit() {
    use crate::query::{DocumentCountRequest, DocumentCountResponse, DriveDocumentCountQuery};
    use dpp::platform_value::Value;

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let factory =
        dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12).expect("factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": { "lot": { "type": "string", "position": 0, "maxLength": 4 } },
        "indices": [{
            "name": "byLot",
            "properties": [{"lot": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "car": document_schema });
    let contract = factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("car").expect("car doctype");

    // Single range clause `lot > "b"` as a typed `WhereClause`.
    // The dispatcher runs the same validate-and-canonicalize
    // step the CBOR-shaped path runs.
    use crate::query::{WhereClause, WhereOperator};
    let where_clauses = vec![WhereClause {
        field: "lot".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("b".to_string()),
    }];

    let drive_config = crate::config::DriveConfig::default();
    let too_large = drive_config.max_query_limit as u32 + 1;

    let request = DocumentCountRequest {
        contract: &contract,
        document_type,
        where_clauses,
        order_clauses: Vec::new(),
        mode: crate::query::CountMode::GroupByRange,
        limit: Some(too_large),
        prove: true,
        drive_config: &drive_config,
        resolved_time_ranges: vec![],
    };
    let result = drive.execute_document_count_request(request, None, pv);

    match &result {
        Err(crate::error::Error::Query(crate::error::query::QuerySyntaxError::InvalidLimit(
            msg,
        ))) => assert!(
            msg.contains("exceeds max_query_limit"),
            "expected message about exceeding max_query_limit, got: {}",
            msg
        ),
        Ok(DocumentCountResponse::Aggregate(_)) => {
            panic!("expected rejection, got Aggregate")
        }
        Ok(DocumentCountResponse::Entries(_)) => panic!("expected rejection, got Entries"),
        Ok(DocumentCountResponse::Proof(_)) => panic!("expected rejection, got Proof"),
        Err(e) => panic!("expected InvalidLimit, got different error: {:?}", e),
    }
    // Silence unused-import for `DriveDocumentCountQuery` —
    // referenced as a type for `PhantomData` only.
    let _ = std::marker::PhantomData::<DriveDocumentCountQuery>;
}

/// The prove-distinct path supports `In` on prefix via grovedb's
/// native subquery primitive: outer `Query` has one `Key(...)`
/// per In value at the In-bearing prop's property-name subtree,
/// `set_subquery_path` carries any post-In Equal pairs +
/// terminator name, `set_subquery` is the range item. The
/// resulting proof emits per-(brand, color) elements which the
/// verifier reads as-is. The server intentionally does NOT merge
/// across forks here, because `limit` pushed into the prover's
/// path query is applied per-fork: merging post-limit would let
/// one fork's surviving entries collide with another fork's
/// dropped entries on the same `key` and silently undercount.
/// Callers that want the flat-histogram view reduce by `key`
/// client-side via [`DocumentSplitCounts::into_flat_map`].
///
/// Mirrors the no-proof
/// `range_count_with_in_on_prefix_returns_per_brand_color_entries`
/// test — same fixture (3 acme+red, 2 acme+blue, 2 contoso+red,
/// 1 contoso+green), same predicate (`brand IN (acme, contoso)
/// AND color > "blue"`), same expected per-(brand, color)
/// entries. Pins that both code paths agree on the unmerged
/// compound shape.
#[test]
fn distinct_count_proof_with_in_on_prefix_returns_per_brand_color_entries() {
    use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
    use dpp::platform_value::Value;
    use grovedb::{Element, GroveDb};

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();

    let factory =
        dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12).expect("factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "brand": { "type": "string", "position": 0, "maxLength": 32 },
            "color": { "type": "string", "position": 1, "maxLength": 32 },
        },
        "indices": [{
            "name": "byBrandColor",
            "properties": [{"brand": "asc"}, {"color": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "widget": document_schema });
    let contract = factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply contract");
    let document_type = contract
        .document_type_for_name("widget")
        .expect("widget exists");

    // Same fixture as the no-proof counterpart.
    let docs: Vec<(&str, &str)> = vec![
        ("acme", "red"),
        ("acme", "red"),
        ("acme", "red"),
        ("acme", "blue"),
        ("acme", "blue"),
        ("contoso", "red"),
        ("contoso", "red"),
        ("contoso", "green"),
    ];
    for (i, (brand, color)) in docs.iter().enumerate() {
        let mut doc = document_type
            .random_document(Some((i + 1) as u64), pv)
            .expect("random doc");
        let mut props = std::collections::BTreeMap::new();
        props.insert("brand".to_string(), Value::Text(brand.to_string()));
        props.insert("color".to_string(), Value::Text(color.to_string()));
        doc.set_properties(props);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
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
            .expect("insert");
    }

    let where_clauses = vec![
        WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("acme".to_string()),
                Value::Text("contoso".to_string()),
            ]),
        },
        WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        },
    ];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("byBrandColor picked");
    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses,
    };

    const LIMIT: u16 = 100;
    let proof_bytes = query
        .execute_distinct_count_with_proof(&drive, LIMIT, true, None, pv)
        .expect("proof");
    assert!(!proof_bytes.is_empty(), "proof must not be empty");

    let path_query = query
        .distinct_count_path_query(Some(LIMIT), true, pv)
        .expect("path query");

    // `lot > "blue"` is one-sided — disable absence proofs
    // (same reason as the other distinct-prove tests).
    let verify_options = grovedb::VerifyOptions {
        absence_proofs_for_non_existing_searched_keys: false,
        ..grovedb::VerifyOptions::default()
    };
    let (root_hash, elements) = GroveDb::verify_query_with_options(
        &proof_bytes,
        &path_query,
        verify_options,
        &pv.drive.grove_version,
    )
    .expect("verify");
    assert_ne!(root_hash, [0u8; 32]);

    // Walk the verified `(path, key, element)` triples and
    // collect per-(brand, color) entries — mirrors what
    // `verify_distinct_count_proof` does. We do NOT sum across
    // brand forks here; the unmerged shape is what the verifier
    // returns.
    let base_path_len = path_query.path.len();
    let mut per_pair: std::collections::BTreeMap<(Vec<u8>, Vec<u8>), u64> =
        std::collections::BTreeMap::new();
    for (path, key, elem) in elements {
        if let Some(e) = elem {
            let _: Element = e.clone();
            let count = e.count_value_or_default();
            if count == 0 {
                continue;
            }
            let in_key = if path.len() > base_path_len {
                path[base_path_len].clone()
            } else {
                Vec::new()
            };
            *per_pair.entry((in_key, key)).or_insert(0) += count;
        }
    }

    // Expected unmerged:
    //   (acme,    red)    → 3
    //   (contoso, green)  → 1
    //   (contoso, red)    → 2
    // blue excluded by `> blue`.
    assert_eq!(
        per_pair.len(),
        3,
        "expected three (brand, color) pairs in the verified proof"
    );
    assert_eq!(per_pair.get(&(b"acme".to_vec(), b"red".to_vec())), Some(&3));
    assert_eq!(
        per_pair.get(&(b"contoso".to_vec(), b"green".to_vec())),
        Some(&1)
    );
    assert_eq!(
        per_pair.get(&(b"contoso".to_vec(), b"red".to_vec())),
        Some(&2)
    );

    // Cross-path agreement (client-side merge): sum across
    // brand forks per color matches what callers reducing by
    // `key` would see. Sum of all per-(brand, color) counts
    // matches the sum-mode no-proof answer (6 docs).
    let mut per_color: std::collections::BTreeMap<Vec<u8>, u64> = std::collections::BTreeMap::new();
    for ((_, color), count) in &per_pair {
        *per_color.entry(color.clone()).or_insert(0) += count;
    }
    assert_eq!(per_color.get(b"red".as_slice()), Some(&5));
    assert_eq!(per_color.get(b"green".as_slice()), Some(&1));
    let total: u64 = per_pair.values().sum();
    assert_eq!(total, 6);
}
