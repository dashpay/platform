use super::*;
use crate::drive::Drive;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::tests::json_document::json_document_to_contract_with_ids;
use dpp::version::PlatformVersion;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::borrow::Cow;
use std::collections::BTreeMap as StdBTreeMap;

fn setup_drive_and_contract() -> (Drive, dpp::prelude::DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();

    let data_contract = json_document_to_contract_with_ids(
        "tests/supporting_files/contract/family/family-contract-countable.json",
        None,
        None,
        false,
        platform_version,
    )
    .expect("expected to get json based contract");

    drive
        .apply_contract(
            &data_contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to apply contract successfully");

    (drive, data_contract)
}

fn insert_random_documents(
    drive: &Drive,
    data_contract: &dpp::prelude::DataContract,
    document_type_name: &str,
    count: usize,
    seed: u64,
) {
    let platform_version = PlatformVersion::latest();
    let document_type = data_contract
        .document_type_for_name(document_type_name)
        .expect("expected document type");

    let mut std_rng = StdRng::seed_from_u64(seed);
    for _ in 0..count {
        let random_document = document_type
            .random_document_with_rng(&mut std_rng, platform_version)
            .expect("expected to get random document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&random_document, storage_flags)),
                        owner_id: None,
                    },
                    contract: data_contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert document");
    }
}

/// Inserts a person document with a controlled set of property values,
/// so tests can drive the count fast path with known firstName / age
/// values rather than relying on the random-document generator.
fn insert_person_doc(
    drive: &Drive,
    data_contract: &dpp::prelude::DataContract,
    id: [u8; 32],
    first_name: &str,
    middle_name: &str,
    last_name: &str,
    age: u64,
) {
    let platform_version = PlatformVersion::latest();
    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let mut properties = StdBTreeMap::new();
    properties.insert("firstName".to_string(), Value::Text(first_name.to_string()));
    properties.insert(
        "middleName".to_string(),
        Value::Text(middle_name.to_string()),
    );
    properties.insert("lastName".to_string(), Value::Text(last_name.to_string()));
    properties.insert("age".to_string(), Value::U64(age));

    let document: Document = DocumentV0 {
        id: Identifier::from(id),
        owner_id: Identifier::from([0u8; 32]),
        properties,
        revision: None,
        created_at: None,
        updated_at: None,
        transferred_at: None,
        created_at_block_height: None,
        updated_at_block_height: None,
        transferred_at_block_height: None,
        created_at_core_block_height: None,
        updated_at_core_block_height: None,
        transferred_at_core_block_height: None,
        creator_id: None,
    }
    .into();

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, storage_flags)),
                    owner_id: None,
                },
                contract: data_contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("expected to insert document");
}

#[test]
fn test_count_query_total_count_with_documents() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    insert_random_documents(&drive, &data_contract, "person", 5, 500);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &[],
    )
    .expect("expected to find countable index");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![],
        split_by_property: None,
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected query to succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].count, 5, "expected count of 5 documents");
    assert!(
        results[0].key.is_empty(),
        "expected empty key for total count"
    );

    // Also verify proof generation works
    let proof = query
        .execute_with_proof(&drive, None, platform_version)
        .expect("expected proof generation to succeed");
    assert!(!proof.is_empty(), "expected non-empty proof");
}

#[test]
fn test_count_query_total_count_empty() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &[],
    )
    .expect("expected to find countable index");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![],
        split_by_property: None,
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected query to succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].count, 0, "expected count of 0 documents");

    // Also verify proof generation works on empty index
    let proof = query
        .execute_with_proof(&drive, None, platform_version)
        .expect("expected proof generation to succeed");
    assert!(!proof.is_empty(), "expected non-empty proof");
}

#[test]
fn test_count_query_split_by_property() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    insert_random_documents(&drive, &data_contract, "person", 5, 600);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let index = DriveDocumentCountQuery::find_countable_index_for_split(
        document_type.indexes(),
        &[],
        "firstName",
    )
    .expect("expected to find countable index for split");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![],
        split_by_property: Some("firstName".to_string()),
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected query to succeed");

    let total: u64 = results.iter().map(|e| e.count).sum();
    assert_eq!(total, 5, "expected total split count of 5 documents");

    for entry in &results {
        assert!(!entry.key.is_empty(), "expected non-empty split key");
        assert!(entry.count > 0, "expected positive count per split");
    }

    // Also verify proof generation works for split query
    let proof = query
        .execute_with_proof(&drive, None, platform_version)
        .expect("expected proof generation to succeed");
    assert!(!proof.is_empty(), "expected non-empty proof");
}

#[test]
fn test_find_countable_index_for_where_clauses_no_match() {
    let platform_version = PlatformVersion::latest();

    let data_contract = json_document_to_contract_with_ids(
        "tests/supporting_files/contract/family/family-contract-countable.json",
        None,
        None,
        false,
        platform_version,
    )
    .expect("expected to get json based contract");

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // Create a where clause for a field that doesn't appear as a prefix of any index
    let where_clause = WhereClause {
        field: "nonExistentField".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text("test".to_string()),
    };

    let result = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &[where_clause],
    );

    assert!(
        result.is_none(),
        "expected no countable index for non-existent field"
    );
}

#[test]
fn test_find_countable_index_for_split_no_match() {
    let platform_version = PlatformVersion::latest();

    let data_contract = json_document_to_contract_with_ids(
        "tests/supporting_files/contract/family/family-contract-countable.json",
        None,
        None,
        false,
        platform_version,
    )
    .expect("expected to get json based contract");

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let result = DriveDocumentCountQuery::find_countable_index_for_split(
        document_type.indexes(),
        &[],
        "nonExistentField",
    );

    assert!(
        result.is_none(),
        "expected no countable index for non-existent split field"
    );
}

#[test]
fn test_has_unsupported_operator() {
    let eq_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::Equal,
        value: Value::U64(30),
    };
    let in_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![Value::U64(30), Value::U64(40)]),
    };
    let gt_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::U64(20),
    };

    assert!(!DriveDocumentCountQuery::has_unsupported_operator(&[]));
    assert!(!DriveDocumentCountQuery::has_unsupported_operator(
        std::slice::from_ref(&eq_clause)
    ));
    assert!(!DriveDocumentCountQuery::has_unsupported_operator(
        std::slice::from_ref(&in_clause)
    ));
    assert!(!DriveDocumentCountQuery::has_unsupported_operator(&[
        eq_clause.clone(),
        in_clause.clone(),
    ]));
    assert!(DriveDocumentCountQuery::has_unsupported_operator(
        std::slice::from_ref(&gt_clause)
    ));
    assert!(DriveDocumentCountQuery::has_unsupported_operator(&[
        eq_clause, gt_clause,
    ]));
}

#[test]
fn test_find_countable_index_rejects_unsupported_operator() {
    let platform_version = PlatformVersion::latest();

    let data_contract = json_document_to_contract_with_ids(
        "tests/supporting_files/contract/family/family-contract-countable.json",
        None,
        None,
        false,
        platform_version,
    )
    .expect("expected to get json based contract");

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let gt_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::U64(20),
    };

    // Even though `age` exists as a countable index, GreaterThan disqualifies it
    // for the count fast path; the picker must report this as "no usable index"
    // so the handler turns it into a clear error.
    assert!(
        DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            std::slice::from_ref(&gt_clause),
        )
        .is_none()
    );
    assert!(DriveDocumentCountQuery::find_countable_index_for_split(
        document_type.indexes(),
        std::slice::from_ref(&gt_clause),
        "firstName",
    )
    .is_none());
}

#[test]
fn test_count_query_total_count_with_in_operator() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    // Three docs with age=30, two with age=40, one with age=50.
    insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [2u8; 32], "Bob", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [3u8; 32], "Carol", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [4u8; 32], "Dave", "M", "Smith", 40);
    insert_person_doc(&drive, &data_contract, [5u8; 32], "Eve", "M", "Smith", 40);
    insert_person_doc(&drive, &data_contract, [6u8; 32], "Frank", "M", "Smith", 50);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // age IN [30, 40] should count 5 documents (3 + 2).
    let in_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![Value::U64(30), Value::U64(40)]),
    };

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        std::slice::from_ref(&in_clause),
    )
    .expect("expected to find countable index for In on age");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![in_clause],
        split_by_property: None,
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected query to succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].count, 5,
        "expected count of 5 (age=30 has 3, age=40 has 2)"
    );
}

#[test]
fn test_count_query_total_count_with_in_operator_no_matches() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [2u8; 32], "Bob", "M", "Smith", 30);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // age IN [99, 100] - no matches.
    let in_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![Value::U64(99), Value::U64(100)]),
    };

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        std::slice::from_ref(&in_clause),
    )
    .expect("expected to find countable index for In on age");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![in_clause],
        split_by_property: None,
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected query to succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].count, 0, "expected count of 0 for unmatched In");
}

#[test]
fn test_count_query_split_with_in_prefix() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    // firstName IN ["Alice", "Bob"] split by lastName
    // Expected: Smith=3 (Alice+Alice+Bob), Jones=2 (Alice+Bob), Doe=1 (Carol — excluded)
    insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [2u8; 32], "Alice", "N", "Smith", 31);
    insert_person_doc(&drive, &data_contract, [3u8; 32], "Bob", "M", "Smith", 32);
    insert_person_doc(&drive, &data_contract, [4u8; 32], "Alice", "M", "Jones", 33);
    insert_person_doc(&drive, &data_contract, [5u8; 32], "Bob", "M", "Jones", 34);
    insert_person_doc(&drive, &data_contract, [6u8; 32], "Carol", "M", "Doe", 35);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let in_clause = WhereClause {
        field: "firstName".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![
            Value::Text("Alice".to_string()),
            Value::Text("Bob".to_string()),
        ]),
    };

    let index = DriveDocumentCountQuery::find_countable_index_for_split(
        document_type.indexes(),
        std::slice::from_ref(&in_clause),
        "lastName",
    )
    .expect("expected to find countable index for In + split lastName");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![in_clause],
        split_by_property: Some("lastName".to_string()),
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected query to succeed");

    let total: u64 = results.iter().map(|e| e.count).sum();
    assert_eq!(
        total, 5,
        "expected total of 5 (3 Smith + 2 Jones, Carol/Doe excluded)"
    );
    assert_eq!(
        results.len(),
        2,
        "expected 2 split entries (Smith and Jones)"
    );
    for entry in &results {
        assert!(entry.count > 0, "filtered split entries should be > 0");
    }
}

/// Codex review finding #3: an `In` clause with duplicate values used to
/// double-count by recursing once per array element. The fix dedupes
/// branches by serialized key before summing.
#[test]
fn test_count_query_in_operator_dedupes_duplicate_values() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [2u8; 32], "Bob", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [3u8; 32], "Carol", "M", "Smith", 40);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // age IN [30, 30, 30] — set semantics: should count age=30 once = 2 docs.
    let in_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![Value::U64(30), Value::U64(30), Value::U64(30)]),
    };

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        std::slice::from_ref(&in_clause),
    )
    .expect("expected to find countable index for In on age");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![in_clause],
        split_by_property: None,
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected query to succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].count, 2,
        "expected count of 2 (age=30, set semantics — duplicates collapsed)"
    );
}

/// Documents the grovedb semantic the count fast path relies on:
/// what does a child element contribute to a parent `CountTree`'s count?
///
/// Per [`merk/src/element/tree_type.rs`](https://github.com/dashpay/grovedb/blob/8f25b20/merk/src/element/tree_type.rs)
/// `get_feature_type`, a child inserted under a `TreeType::CountTree` parent
/// is wrapped as `CountedMerkNode(self.count_value_or_default())`, and per
/// [`grovedb-element/src/element/helpers.rs`](https://github.com/dashpay/grovedb/blob/8f25b20/grovedb-element/src/element/helpers.rs)
/// `count_value_or_default` returns the child's stored count value for
/// `CountTree` / `ProvableCountTree` / `CountSumTree` / `ProvableCountSumTree`
/// variants and **`1` for everything else** — including an empty `Tree`.
///
/// So an empty `Tree` child counts as **1**, not 0. The CountTree's
/// aggregated count is `local_value + left_subtree_count + right_subtree_count`
/// (recursively).
#[test]
fn test_count_tree_aggregation_with_empty_child_subtrees() {
    use grovedb::Element;
    use grovedb_path::SubtreePath;

    let drive = crate::util::test_helpers::setup::setup_drive(None);
    let platform_version = PlatformVersion::latest();
    let drive_version = &platform_version.drive;
    let grove_version = &drive_version.grove_version;

    let root: &[&[u8]] = &[];

    // 1. Insert an empty CountTree at the root, key "ct".
    drive
        .grove
        .insert(
            SubtreePath::from(root),
            b"ct",
            Element::empty_count_tree(),
            None,
            None,
            grove_version,
        )
        .unwrap()
        .expect("insert empty count tree");

    let read_count = |drive: &Drive| -> u64 {
        let elem = drive
            .grove
            .get(SubtreePath::from(root), b"ct", None, grove_version)
            .unwrap()
            .expect("read count tree");
        elem.count_value_or_default()
    };

    // Empty CountTree → count = 0 (no children at all).
    assert_eq!(
        read_count(&drive),
        0,
        "freshly created empty count tree should have count 0"
    );

    // 2. Insert an empty Tree (NormalTree) as a child of the CountTree.
    let count_tree_path: &[&[u8]] = &[b"ct"];
    drive
        .grove
        .insert(
            SubtreePath::from(count_tree_path),
            b"empty_subtree_a",
            Element::empty_tree(),
            None,
            None,
            grove_version,
        )
        .unwrap()
        .expect("insert empty subtree under count tree");

    // The empty Tree contributes 1 to the parent CountTree's count.
    assert_eq!(
        read_count(&drive),
        1,
        "an empty Tree child counts as 1 inside a CountTree"
    );

    // 3. Insert a second empty Tree → count = 2.
    drive
        .grove
        .insert(
            SubtreePath::from(count_tree_path),
            b"empty_subtree_b",
            Element::empty_tree(),
            None,
            None,
            grove_version,
        )
        .unwrap()
        .expect("insert second empty subtree");
    assert_eq!(
        read_count(&drive),
        2,
        "two empty Tree children count as 2 inside a CountTree"
    );

    // 4. Insert a non-tree Item child → count = 3 (each non-CountTree element
    //    contributes 1, regardless of whether it's a Tree, an Item, or a
    //    Reference).
    drive
        .grove
        .insert(
            SubtreePath::from(count_tree_path),
            b"item",
            Element::new_item(b"hello".to_vec()),
            None,
            None,
            grove_version,
        )
        .unwrap()
        .expect("insert item child");
    assert_eq!(
        read_count(&drive),
        3,
        "an Item child also contributes 1, same as an empty Tree"
    );
}

/// Sanity check that a contract using the new `IndexCountability::CountableAllowingOffset`
/// variant — a string `"countableAllowingOffset"` in the JSON schema — loads,
/// is recognized as countable by the picker, and answers a total-count query.
///
/// This exercises the full path: schema → `IndexCountability` enum → index
/// picker → tree-type selection (`ProvableCountTree`) → fast-path read.
#[test]
fn test_countable_allowing_offset_variant_end_to_end() {
    use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use dpp::data_contract::document_type::IndexCountability;

    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();

    // Hand-build a contract from JSON value so we can use the string form.
    // family-contract-countable.json sits next door using "countable": true; this
    // covers the CountableAllowingOffset variant via the string form and a
    // single index for clarity.
    let contract_json = serde_json::json!({
        "$formatVersion": "0",
        "id": "94zNLp7A1ZcYG3Egqf2YmQk4DQr9P8D543GwXyCJRz4",
        "ownerId": "AcYUCSvAmUwryNsQqkqqD1o3BnFuzepGtR3Mhh2swLk6",
        "version": 1,
        "documentSchemas": {
            "person": {
                "type": "object",
                "indices": [
                    {
                        "name": "byFirstName",
                        "properties": [{ "firstName": "asc" }],
                        "countable": "countableAllowingOffset"
                    }
                ],
                "properties": {
                    "firstName": {
                        "type": "string",
                        "maxLength": 50,
                        "position": 0
                    }
                },
                "required": ["firstName"],
                "additionalProperties": false
            }
        }
    });

    let data_contract =
        dpp::data_contract::DataContract::from_json(contract_json, false, platform_version)
            .expect("expected to load contract with string-form countable");

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // Confirm the schema parsed into the right enum variant.
    let index = document_type
        .indexes()
        .values()
        .next()
        .expect("expected one index");
    assert_eq!(
        index.countable,
        IndexCountability::CountableAllowingOffset,
        "string \"countableAllowingOffset\" should parse as CountableAllowingOffset"
    );
    assert!(index.countable.allows_offset());

    drive
        .apply_contract(
            &data_contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to apply contract");

    insert_random_documents(&drive, &data_contract, "person", 4, 700);

    // The picker should still find this index — `is_countable()` covers both
    // `Countable` and `CountableAllowingOffset`.
    let picked = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &[],
    )
    .expect("expected picker to accept CountableAllowingOffset index");
    assert_eq!(picked.countable, IndexCountability::CountableAllowingOffset);

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index: picked,
        where_clauses: vec![],
        split_by_property: None,
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected count query to succeed against ProvableCountTree");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].count, 4,
        "ProvableCountTree should report total count = 4"
    );
}

/// Codex review finding #4: a `unique: true, countable: true` index used to
/// allegedly return 0 because `fetch_count_at_path` would read a Reference
/// instead of a CountTree element. We assert the correct count semantics
/// (1 per matching unique tuple, summed under partial prefixes) so a
/// regression here surfaces immediately.
#[test]
fn test_count_query_unique_countable_index_returns_correct_count() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    // 3 distinct (firstName, middleName, lastName) tuples — the unique
    // countable index `(firstName, middleName, lastName)` stores a
    // Reference at key [0] under the final value level.
    insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [2u8; 32], "Alice", "N", "Smith", 31);
    insert_person_doc(&drive, &data_contract, [3u8; 32], "Bob", "O", "Jones", 32);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // Pick the unique countable 3-property index by matching its full prefix.
    let where_clauses = vec![
        WhereClause {
            field: "firstName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("Alice".to_string()),
        },
        WhereClause {
            field: "middleName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("M".to_string()),
        },
        WhereClause {
            field: "lastName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("Smith".to_string()),
        },
    ];

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
    )
    .expect("expected to find a countable index covering all 3 properties");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses,
        split_by_property: None,
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected query to succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].count, 1,
        "exact match on a unique countable index should be 1, not 0"
    );
}
