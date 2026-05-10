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
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected query to succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].count, 0, "expected count of 0 for unmatched In");
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

/// `execute_document_count_per_in_value_no_proof` runs one GroveDB walk
/// per `In` value, so its iteration cost is proportional to the array's
/// length rather than the configured `max_query_limit`. That makes the
/// In-array length the actual amplification factor — capping the
/// *output* `limit` after the loop is cosmetic. We delegate the cap to
/// `WhereClause::in_values()` (the same 100-element validator other In
/// consumers use); this test pins that delegation at the executor's
/// entry point so a regression here surfaces as a query-rejection
/// rather than as a quietly amplified backend scan.
#[test]
fn test_count_query_in_operator_rejects_oversized_array() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // 101 distinct `age` values triggers the 100-cap in `in_values()`.
    let oversized: Vec<Value> = (0u64..101).map(Value::U64).collect();
    let in_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(oversized),
    };

    let err = drive
        .execute_document_count_per_in_value_no_proof(
            data_contract.id().to_buffer(),
            document_type,
            "person".to_string(),
            vec![in_clause],
            super::RangeCountOptions {
                distinct: false,
                limit: Some(50),
                start_after_split_key: None,
                order_by_ascending: true,
            },
            None,
            platform_version,
        )
        .expect_err("expected 101-element In array to be rejected");

    let msg = err.to_string();
    assert!(
        msg.contains("at most 100"),
        "expected 100-cap rejection, got: {}",
        msg
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

/// On a unique index, the `countable` flag only affects storage for
/// **null-bearing** index entries: when a document has any null value among
/// the indexed properties, insertion goes through the count-tree branch
/// (the same one non-unique indexes use). For all-non-null docs on a
/// unique index, the terminal is a bare Reference at key `[0]` and the
/// flag is a no-op — the count *value* still works correctly because
/// grovedb's `Element::count_value_or_default()` returns 1 for non-CountTree
/// elements (Reference falls into the `_ => 1` arm).
///
/// This test exercises the all-non-null path on a unique countable index
/// and verifies the count comes back as 1 — confirming the no-op fallback.
#[test]
fn test_count_query_unique_countable_index_returns_correct_count() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    // 3 distinct (firstName, middleName, lastName) tuples — the unique
    // countable index `(firstName, middleName, lastName)` stores a
    // Reference at key [0] under the final value level (no count tree
    // is created because all indexed fields are non-null).
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
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected query to succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].count, 1,
        "exact match on a unique countable index should be 1, not 0 \
         (Reference at [0] returns count_value_or_default = 1)"
    );
}

#[cfg(test)]
mod range_countable_picker_tests {
    //! Coverage for [`DriveDocumentCountQuery::find_range_countable_index_for_where_clauses`].
    //!
    //! Builds a small in-memory `BTreeMap<String, Index>` rather than going
    //! through a full DataContract, since we're only testing the picker
    //! rule (prefix match + range terminator + range_countable=true) and
    //! the contract-level wiring is exercised by the e2e tests under
    //! `drive::contract::insert::insert_contract::v0::range_countable_index_e2e_tests`.

    use super::*;
    use dpp::data_contract::document_type::{Index, IndexCountability, IndexProperty};

    fn make_index(
        name: &str,
        properties: &[&str],
        countable: IndexCountability,
        range_countable: bool,
    ) -> Index {
        Index {
            name: name.to_string(),
            properties: properties
                .iter()
                .map(|p| IndexProperty {
                    name: p.to_string(),
                    ascending: true,
                })
                .collect(),
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable,
            range_countable,
        }
    }

    fn make_indexes(indexes: Vec<Index>) -> std::collections::BTreeMap<String, Index> {
        indexes.into_iter().map(|i| (i.name.clone(), i)).collect()
    }

    /// Single-property range_countable index — straightforward range
    /// query over `color`.
    #[test]
    fn picks_single_property_range_countable_index() {
        let indexes = make_indexes(vec![make_index(
            "byColor",
            &["color"],
            IndexCountability::Countable,
            true,
        )]);
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("a".to_string()),
        }];
        let picked = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            &indexes,
            &where_clauses,
        );
        assert!(picked.is_some());
        assert_eq!(picked.unwrap().name, "byColor");
    }

    /// Compound range_countable `[brand, color]`: Equal on `brand` (the
    /// prefix), range on `color` (the terminator).
    #[test]
    fn picks_compound_range_countable_index_with_equal_prefix() {
        let indexes = make_indexes(vec![make_index(
            "byBrandColor",
            &["brand", "color"],
            IndexCountability::Countable,
            true,
        )]);
        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("acme".to_string()),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::Between,
                value: Value::Array(vec![
                    Value::Text("a".to_string()),
                    Value::Text("z".to_string()),
                ]),
            },
        ];
        let picked = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            &indexes,
            &where_clauses,
        );
        assert!(picked.is_some());
        assert_eq!(picked.unwrap().name, "byBrandColor");
    }

    /// Range on a non-terminator property must not match. For
    /// `[brand, color]`, a range on `brand` (with no clause on `color`)
    /// would not be answerable via the index walker model — there's no
    /// CountTree at the brand value level.
    #[test]
    fn rejects_range_on_non_terminator_property() {
        let indexes = make_indexes(vec![make_index(
            "byBrandColor",
            &["brand", "color"],
            IndexCountability::Countable,
            true,
        )]);
        let where_clauses = vec![WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("a".to_string()),
        }];
        assert!(
            DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                &indexes,
                &where_clauses,
            )
            .is_none(),
            "a range on a non-terminator property must not match — the storage \
             layout doesn't put a ProvableCountTree at that level"
        );
    }

    /// An index without `range_countable: true` must not match even if
    /// the property structure aligns. The storage layout for these is
    /// plain NormalTree — no CountTree counts to walk.
    #[test]
    fn rejects_non_range_countable_index() {
        let indexes = make_indexes(vec![make_index(
            "byColor",
            &["color"],
            IndexCountability::Countable,
            false, // <-- NOT range_countable
        )]);
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("a".to_string()),
        }];
        assert!(
            DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                &indexes,
                &where_clauses,
            )
            .is_none()
        );
    }

    /// Two range operators should never resolve to a single index — the
    /// PathQuery model can express only one range at a time.
    #[test]
    fn rejects_multiple_range_operators() {
        let indexes = make_indexes(vec![make_index(
            "byColor",
            &["color"],
            IndexCountability::Countable,
            true,
        )]);
        let where_clauses = vec![
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Text("a".to_string()),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::LessThan,
                value: Value::Text("z".to_string()),
            },
        ];
        assert!(
            DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                &indexes,
                &where_clauses,
            )
            .is_none(),
            "two separate range operators must be rejected (use Between to express a bounded range)"
        );
    }

    /// Pure point-lookup queries should NOT match the range picker —
    /// they belong on `find_countable_index_for_where_clauses` instead.
    #[test]
    fn rejects_pure_point_lookup_queries() {
        let indexes = make_indexes(vec![make_index(
            "byColor",
            &["color"],
            IndexCountability::Countable,
            true,
        )]);
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("red".to_string()),
        }];
        assert!(
            DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                &indexes,
                &where_clauses,
            )
            .is_none(),
            "no range operator → not the range picker's job"
        );
    }
}

#[cfg(test)]
mod detect_mode_tests {
    //! Coverage for [`DriveDocumentCountQuery::detect_mode`].
    //!
    //! Pure validation/dispatch decisions — no Drive instance, no
    //! contract, no platform_version needed. Tests the full truth
    //! table of (range × In × distinct × prove).

    use super::*;

    fn eq_clause(field: &str) -> WhereClause {
        WhereClause {
            field: field.to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("x".to_string()),
        }
    }
    fn in_clause(field: &str) -> WhereClause {
        WhereClause {
            field: field.to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![Value::Text("a".to_string())]),
        }
    }
    fn gt_clause(field: &str) -> WhereClause {
        WhereClause {
            field: field.to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("b".to_string()),
        }
    }
    fn lt_clause(field: &str) -> WhereClause {
        WhereClause {
            field: field.to_string(),
            operator: WhereOperator::LessThan,
            value: Value::Text("z".to_string()),
        }
    }

    /// No clauses, no flags → total mode.
    #[test]
    fn no_clauses_no_flags_is_total() {
        let mode = DriveDocumentCountQuery::detect_mode(&[], false, false).unwrap();
        assert_eq!(mode, DocumentCountMode::Total);
    }

    /// Equal-only clauses → still total.
    #[test]
    fn only_equal_clauses_is_total() {
        let clauses = vec![eq_clause("a"), eq_clause("b")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, false, false).unwrap(),
            DocumentCountMode::Total,
        );
    }

    /// Single In clause → per-In-value.
    #[test]
    fn single_in_is_per_in_value() {
        let clauses = vec![in_clause("a")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, false, false).unwrap(),
            DocumentCountMode::PerInValue,
        );
    }

    /// Equal + In on different fields → per-In-value.
    #[test]
    fn equal_plus_in_is_per_in_value() {
        let clauses = vec![eq_clause("a"), in_clause("b")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, false, false).unwrap(),
            DocumentCountMode::PerInValue,
        );
    }

    /// Single range + no proof → range no-proof.
    #[test]
    fn single_range_no_proof_is_range_no_proof() {
        let clauses = vec![gt_clause("color")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, false, false).unwrap(),
            DocumentCountMode::RangeNoProof,
        );
    }

    /// Single range + prove → range proof.
    #[test]
    fn single_range_with_prove_is_range_proof() {
        let clauses = vec![gt_clause("color")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, false, true).unwrap(),
            DocumentCountMode::RangeProof,
        );
    }

    /// No range + prove → point-lookup proof (materialize-and-count).
    #[test]
    fn no_range_with_prove_is_point_lookup_proof() {
        let clauses = vec![eq_clause("a")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, false, true).unwrap(),
            DocumentCountMode::PointLookupProof,
        );
    }

    /// Equal-prefix + range terminator + no proof → range no-proof.
    #[test]
    fn equal_prefix_plus_range_terminator_is_range_no_proof() {
        let clauses = vec![eq_clause("brand"), gt_clause("color")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, false, false).unwrap(),
            DocumentCountMode::RangeNoProof,
        );
    }

    /// Two range operators → rejected.
    #[test]
    fn two_range_operators_rejected() {
        let clauses = vec![gt_clause("color"), lt_clause("color")];
        let err = DriveDocumentCountQuery::detect_mode(&clauses, false, false).unwrap_err();
        assert!(matches!(
            err,
            QuerySyntaxError::InvalidWhereClauseComponents(msg) if msg.contains("at most one range")
        ));
    }

    /// Two `In` operators → rejected.
    #[test]
    fn two_in_operators_rejected() {
        let clauses = vec![in_clause("a"), in_clause("b")];
        let err = DriveDocumentCountQuery::detect_mode(&clauses, false, false).unwrap_err();
        assert!(matches!(
            err,
            QuerySyntaxError::InvalidWhereClauseComponents(msg) if msg.contains("at most one `in`")
        ));
    }

    /// Range + In together → rejected (ambiguous output shape).
    #[test]
    fn range_plus_in_rejected() {
        let clauses = vec![in_clause("a"), gt_clause("b")];
        let err = DriveDocumentCountQuery::detect_mode(&clauses, false, false).unwrap_err();
        assert!(matches!(
            err,
            QuerySyntaxError::InvalidWhereClauseComponents(msg) if msg.contains("cannot also carry an `in`")
        ));
    }

    /// `return_distinct_counts_in_range = true` without a range → rejected.
    #[test]
    fn distinct_without_range_rejected() {
        let err = DriveDocumentCountQuery::detect_mode(&[], true, false).unwrap_err();
        assert!(matches!(
            err,
            QuerySyntaxError::InvalidWhereClauseComponents(msg) if msg.contains("requires a range where-clause")
        ));
    }

    /// `return_distinct_counts_in_range = true` + `prove = true` → rejected
    /// (the proof primitive returns a single aggregate).
    #[test]
    fn distinct_on_prove_path_rejected() {
        let clauses = vec![gt_clause("color")];
        let err = DriveDocumentCountQuery::detect_mode(&clauses, true, true).unwrap_err();
        assert!(matches!(
            err,
            QuerySyntaxError::InvalidWhereClauseComponents(msg) if msg.contains("only supported on the \\\n                 no-prove path") || msg.contains("no-prove path")
        ));
    }

    /// Distinct mode in no-prove range → still RangeNoProof; the
    /// distinct flag is consumed by the executor, not the mode tag.
    #[test]
    fn distinct_no_prove_with_range_is_range_no_proof() {
        let clauses = vec![gt_clause("color")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, true, false).unwrap(),
            DocumentCountMode::RangeNoProof,
        );
    }

    /// `prove = true` + `In` routes to `PointLookupProof` (the
    /// materialize-and-count proof fallback). The SDK's
    /// `FromProof<DocumentCountQuery>` for `DocumentSplitCounts`
    /// then groups verified documents by the In field's serialized
    /// value to produce per-key count entries. No proof aggregate
    /// primitive supports per-In-value entries directly, but
    /// materialize-and-count is correct (and was the pre-refactor
    /// behavior).
    #[test]
    fn in_with_prove_routes_to_point_lookup_proof() {
        let clauses = vec![in_clause("a")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, false, true).unwrap(),
            DocumentCountMode::PointLookupProof,
        );
    }
}
