use super::*;
use crate::drive::Drive;
use crate::query::ResolvedTimeRange;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::tests::json_document::json_document_to_contract_with_ids;
use dpp::version::PlatformVersion;
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
        contract_version: None,
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

/// Exact-coverage query (`age == 30` against the single-property
/// `byAge` countable index) — the strict-picker happy path on both
/// no-proof and prove. Pins:
/// - Picker accepts a 1-property index whose property exactly matches
///   the where-clause field.
/// - No-proof executor reads the CountTree at the resolved path and
///   returns the count.
/// - Prove executor builds a CountTree-element proof returning
///   non-empty bytes.
#[test]
fn test_count_query_fully_covered_equal_succeeds_on_both_paths() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    // 3 docs at age=30, 2 at age=40 → byAge count at 30 should be 3.
    insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [2u8; 32], "Bob", "", "Jones", 30);
    insert_person_doc(&drive, &data_contract, [3u8; 32], "Carol", "", "Brown", 30);
    insert_person_doc(&drive, &data_contract, [4u8; 32], "Dave", "", "Smith", 40);
    insert_person_doc(&drive, &data_contract, [5u8; 32], "Eve", "", "Jones", 40);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let age_eq_30 = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::Equal,
        value: Value::U64(30),
    };
    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        std::slice::from_ref(&age_eq_30),
        &[],
    )
    .expect("expected picker to accept fully-covered byAge index");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![age_eq_30],
    };

    // No-proof path
    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected no-proof count to succeed");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].count,
        Some(3),
        "expected count of 3 docs at age=30"
    );
    assert!(
        results[0].key.is_empty(),
        "expected empty key for fully-covered Equal-only count"
    );

    // Prove path — emits the CountTree element proof for the resolved
    // branch. Non-empty bytes guarantee the prover walked a real merk
    // path (not a degenerate empty envelope).
    let proof = query
        .execute_point_lookup_count_with_proof(&drive, None, platform_version)
        .expect("expected prove count to succeed on fully-covered Equal query");
    assert!(
        !proof.is_empty(),
        "expected non-empty proof bytes for fully-covered Equal prove count"
    );
}

/// Strict-picker rejection contract: a where clause that doesn't
/// exactly cover any `countable: true` index returns `None` from the
/// picker. Pre-rewrite the picker would have returned a longer-prefix
/// index and downstream code would have walked partially-covered
/// trees via `count_recursive`; now the responsibility for index
/// design sits cleanly with the contract author, and queries against
/// partially-covered indexes fail loudly at the picker level.
#[test]
fn test_count_query_picker_rejects_partial_coverage() {
    let (_drive, data_contract) = setup_drive_and_contract();
    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // family-contract-countable.json has byFirstNameLastName (2 props),
    // byFirstNameMiddleLastName (3 props, unique), and byAge (1 prop).
    // Empty where doesn't exactly cover any of them.
    let no_match = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &[],
        &[],
    );
    assert!(
        no_match.is_none(),
        "strict picker must reject empty where clauses (no index has 0 properties)"
    );

    // `firstName = X` alone is a prefix of byFirstNameLastName but
    // not an exact match — there's no 1-property `[firstName]` index
    // in this contract. Strict picker rejects.
    let first_name_only = vec![WhereClause {
        field: "firstName".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text("Alice".to_string()),
    }];
    let no_match_partial = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &first_name_only,
        &[],
    );
    assert!(
        no_match_partial.is_none(),
        "`firstName = X` doesn't exactly cover any index (only as prefix of \
         2- and 3-property indexes) → picker returns None"
    );

    // `age = X` exactly covers byAge (1-prop) → picker accepts.
    // Confirms the strict contract isn't over-rejecting.
    let age_only = vec![WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::Equal,
        value: Value::U64(30),
    }];
    let picked = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &age_only,
        &[],
    )
    .expect("byAge is exactly covered");
    assert_eq!(picked.properties.len(), 1);
    assert_eq!(picked.properties[0].name, "age");
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
        &[],
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
            &[],
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
        &[],
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
        results[0].count,
        Some(5),
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
        &[],
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
        results[0].count,
        Some(0),
        "expected count of 0 for unmatched In"
    );
}

/// Pin against silent-aggregate-truncation: the PerInValue / range
/// fan-out arms used to unwrap `request.limit` to
/// `drive_config.default_query_limit`, which under tighter operator
/// tuning would truncate the per-In fan-out below |In| and produce
/// a wrong aggregate sum.
///
/// `CountMode::Aggregate` callers reject explicit `limit` upstream
/// (`validate_and_route` returns `InvalidLimit`), so the only path
/// into the dispatcher with a meaningful In fan-out cap is the
/// constant `MAX_LIMIT_AS_FAILSAFE` baked into the dispatcher. This
/// test sets `default_query_limit = 3` and asks for an Aggregate
/// over an 8-element In array: pre-fix this returned 3 (sum of
/// first 3 In branches), post-fix it returns 8.
#[test]
fn test_aggregate_count_in_fan_out_ignores_default_query_limit() {
    use crate::config::DriveConfig;
    use crate::query::drive_document_count_query::drive_dispatcher::{
        DocumentCountRequest, DocumentCountResponse,
    };

    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    // 8 distinct ages, one doc per age. Each doc gets a unique
    // (firstName, middleName, lastName) tuple to satisfy the
    // family-contract-countable's unique compound index.
    // Count > `OPERATOR_TUNED_LIMIT` (3) so truncation would be
    // detectable.
    let names = [
        "Alice", "Bob", "Carol", "Dave", "Eve", "Frank", "Grace", "Heidi",
    ];
    for (i, (age, name)) in [30u64, 40, 50, 60, 70, 80, 90, 100]
        .iter()
        .zip(names.iter())
        .enumerate()
    {
        insert_person_doc(
            &drive,
            &data_contract,
            [i as u8 + 1; 32],
            name,
            "M",
            "Smith",
            *age,
        );
    }

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // Operator-tuned tight `default_query_limit`. Pre-fix the
    // dispatcher would propagate this to the PerInValue executor
    // and truncate the fan-out to 3 of the 8 In branches.
    const OPERATOR_TUNED_LIMIT: u16 = 3;
    let drive_config = DriveConfig {
        default_query_limit: OPERATOR_TUNED_LIMIT,
        ..Default::default()
    };

    // Typed `In` clause on `age` with all 8 values. The dispatcher
    // runs the same validate-and-canonicalize step the CBOR-shaped
    // path runs (see [`validate_and_canonicalize_where_clauses`]),
    // so structurally identical to the legacy fixture.
    let where_clauses = vec![WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![
            Value::U64(30),
            Value::U64(40),
            Value::U64(50),
            Value::U64(60),
            Value::U64(70),
            Value::U64(80),
            Value::U64(90),
            Value::U64(100),
        ]),
    }];

    let request = DocumentCountRequest {
        contract: &data_contract,
        document_type,
        where_clauses,
        order_clauses: Vec::new(),
        mode: CountMode::Aggregate,
        // Aggregate rejects explicit `limit` upstream; the
        // dispatcher must not substitute `default_query_limit` for
        // the per-In fan-out cap or the aggregate is wrong.
        limit: None,
        prove: false,
        drive_config: &drive_config,
        resolved_time_ranges: vec![],
    };

    let response = drive
        .execute_document_count_request(request, None, platform_version)
        .expect("dispatcher should succeed on Aggregate + In + no-prove");

    // rs-drive's dispatcher emits `Entries` for the PerInValue
    // path; drive-abci's `dispatch_count_v1` is what sums them
    // into a single `Aggregate` response on the wire. At this
    // layer we exercise the fan-out directly: both the entry
    // count and the sum-of-counts must match the full 8 In
    // branches, regardless of `OPERATOR_TUNED_LIMIT`.
    let entries = match response {
        DocumentCountResponse::Entries(e) => e,
        other => panic!("expected Entries response from PerInValue dispatch, got {other:?}"),
    };
    assert_eq!(
        entries.len(),
        8,
        "PerInValue fan-out must emit all 8 In branches regardless of \
         operator-tuned default_query_limit ({OPERATOR_TUNED_LIMIT}); pre-fix \
         this returned {OPERATOR_TUNED_LIMIT} entries because the dispatcher \
         propagated `default_query_limit` to the executor's `RangeCountOptions::limit`"
    );
    let total: u64 = entries.iter().filter_map(|e| e.count).sum();
    assert_eq!(
        total, 8,
        "aggregate sum over per-In entries must be 8; under the pre-fix \
         truncation the sum would have been {OPERATOR_TUNED_LIMIT}"
    );
}

/// `In` clauses with duplicate values are rejected with
/// `InvalidInClause` — the system-wide canonical contract enforced
/// by [`WhereClause::in_values`]. Every In-consuming path the count
/// dispatcher reaches (the shared `point_lookup_count_path_query`
/// builder for both no-proof and prove, the `per_in_value`
/// executor's `in_values()` call, the regular document query path,
/// the contract-level where-clause validator) routes through the
/// same `in_values()` validator, so `age IN [30, 30]` is rejected
/// loudly rather than silently deduplicated.
///
/// Pre-unification the no-proof count path was the outlier — its
/// hand-rolled `expand_paths_and_count` walker bypassed
/// `in_values()` and silently deduplicated via a `BTreeSet<Vec<u8>>`
/// of serialized keys. Collapsing the no-proof executor to share
/// the path-query builder fixed that inconsistency by routing both
/// sides through the same validator.
#[test]
fn test_count_query_in_operator_rejects_duplicate_values() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // age IN [30, 30, 30] — duplicates rejected by the system-wide
    // `in_values()` validator before any subtree access.
    let in_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![Value::U64(30), Value::U64(30), Value::U64(30)]),
    };

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        std::slice::from_ref(&in_clause),
        &[],
    )
    .expect("expected to find countable index for In on age");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![in_clause],
    };

    let err = query
        .execute_no_proof(&drive, None, platform_version)
        .expect_err("expected duplicate-In-values to be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("no duplicates"),
        "expected duplicate-rejection error from in_values(), got: {}",
        msg
    );
}

/// `In` on the **before-last** index property with a trailing `Equal`
/// on the last property exercises the relaxed prove count builder
/// shape. The regular document query path's `Index::matches` allows
/// `In` on the last OR before-last property of the chosen index, and
/// the prove count builder follows the same rule (see
/// `point_lookup_count_path_query` in `path_query.rs`).
///
/// Index used: `byFirstNameLastName` (`[firstName, lastName]`).
/// Where: `firstName IN ["Alice", "Bob"] AND lastName == "Smith"`.
/// - Alice + Smith: 2 docs
/// - Bob + Smith: 1 doc
/// - Bob + Jones: 1 doc (ignored — lastName != Smith)
/// - Carol + Smith: 1 doc (ignored — firstName not in In array)
///
/// Pins:
/// - Strict picker accepts the 2-prop index when both properties are
///   covered (one by In, one by Equal).
/// - No-proof executor goes through the same
///   `point_lookup_count_path_query` builder as the prove side, runs
///   it through `grove.query`, and sums the emitted CountTree
///   elements' `count_value`s: 2 + 1 = 3.
/// - Prove executor builds a compound path query whose `base_path`
///   stops at `[..., "firstName"]`, with `outer_query` keys = the
///   sorted serialized In values and `set_subquery_path` carrying
///   `["lastName", serialize("Smith")]`; the subquery's `Key([0])`
///   then picks off the CountTree under each matched In branch.
/// - Proof verifies (round-trips through `GroveDb::verify_query` in
///   the verifier), and the verified per-branch entries' counts sum
///   to the no-proof count.
#[test]
fn test_count_query_in_on_before_last_with_trailing_equal_succeeds_on_both_paths() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    // Different middle names so the unique `byFirstNameMiddleLastName`
    // index is satisfied — the count goes through the non-unique
    // 2-prop `byFirstNameLastName` index, which doesn't care about
    // middleName.
    insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [2u8; 32], "Alice", "N", "Smith", 31);
    insert_person_doc(&drive, &data_contract, [3u8; 32], "Bob", "M", "Smith", 40);
    insert_person_doc(&drive, &data_contract, [4u8; 32], "Bob", "N", "Jones", 41);
    insert_person_doc(&drive, &data_contract, [5u8; 32], "Carol", "M", "Smith", 50);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let in_first = WhereClause {
        field: "firstName".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![
            Value::Text("Alice".to_string()),
            Value::Text("Bob".to_string()),
        ]),
    };
    let eq_last = WhereClause {
        field: "lastName".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text("Smith".to_string()),
    };
    let where_clauses = vec![in_first, eq_last];

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("expected picker to accept byFirstNameLastName for In + Equal coverage");
    // Sanity-check the picker really chose the 2-prop index, not the
    // 3-prop unique one — confirms set-equality coverage and pins the
    // covering-index expectation against future picker tweaks.
    assert_eq!(index.properties.len(), 2);

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses,
    };

    // No-proof: 2 Alice+Smith + 1 Bob+Smith = 3.
    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected no-proof count to succeed");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].count,
        Some(3),
        "expected 3 docs covered by firstName IN [Alice, Bob] AND lastName = Smith"
    );

    // Prove: builder emits the compound shape; verifier round-trips
    // and returns per-In-value entries.
    let proof = query
        .execute_point_lookup_count_with_proof(&drive, None, platform_version)
        .expect("expected prove count to succeed on In-on-before-last shape");
    assert!(
        !proof.is_empty(),
        "expected non-empty proof bytes for In-on-before-last prove count"
    );

    let (_root_hash, entries) = query
        .verify_point_lookup_count_proof(&proof, platform_version)
        .expect("expected proof verification to succeed");
    // Verifier emits one entry per In branch with a non-zero count.
    // Alice → 2, Bob → 1.
    let summed: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
    assert_eq!(
        summed, 3,
        "verified per-branch entries should sum to the no-proof total"
    );
}

/// `In` on the **first** property of a 3-property index, with two
/// trailing Equals (`firstName IN [..] AND middleName = m AND
/// lastName = ln` on the unique `byFirstNameMiddleLastName` index)
/// — exercises the most aggressive shape the relaxed prove count
/// builder accepts: In at position 0 with two trailing Equals
/// rolling through `subquery_path_extension`. Both no-proof and
/// prove paths go through the same
/// `point_lookup_count_path_query` builder (no-proof reads the
/// emitted CountTree elements via `grove.query`; prove signs them
/// via `get_proved_path_query`), so accepting this shape on one
/// side automatically accepts it on the other. The count path is
/// deliberately more permissive than the regular document query
/// path here — see the builder's docstring for the divergence
/// rationale vs. `Index::matches`.
///
/// Index used: `byFirstNameMiddleLastName` (unique, 3 props).
/// Where: `firstName IN ["Alice", "Bob"] AND middleName = "M" AND
/// lastName = "Smith"`.
/// - (Alice, M, Smith): 1 doc
/// - (Bob, M, Smith): 1 doc
/// - (Carol, M, Smith): 1 doc (excluded — firstName not in In)
/// - (Alice, N, Smith): 1 doc (excluded — middleName ≠ M)
///
/// Pins:
/// - Strict picker accepts the 3-prop covering index.
/// - No-proof executor sums per-In-value: 1 + 1 = 2.
/// - Prove executor builds a compound path query with `base_path`
///   stopping at `[..., "firstName"]`, `outer_query` keys = sorted
///   serialized In values, `set_subquery_path` =
///   `["middleName", serialize("M"), "lastName", serialize("Smith")]`,
///   subquery `Key([0])`.
/// - Proof verifies and the verified per-branch entries' counts
///   sum to the no-proof count.
#[test]
fn test_count_query_in_on_first_of_three_with_two_trailing_equals_succeeds_on_both_paths() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    // Pick distinct (firstName, middleName, lastName) tuples so the
    // unique 3-prop index doesn't reject any inserts. The picker
    // will route the count query through that same 3-prop index
    // because the where clauses cover exactly its properties.
    insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [2u8; 32], "Bob", "M", "Smith", 40);
    insert_person_doc(&drive, &data_contract, [3u8; 32], "Carol", "M", "Smith", 50);
    insert_person_doc(&drive, &data_contract, [4u8; 32], "Alice", "N", "Smith", 31);
    insert_person_doc(&drive, &data_contract, [5u8; 32], "Bob", "N", "Jones", 41);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let in_first = WhereClause {
        field: "firstName".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![
            Value::Text("Alice".to_string()),
            Value::Text("Bob".to_string()),
        ]),
    };
    let eq_middle = WhereClause {
        field: "middleName".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text("M".to_string()),
    };
    let eq_last = WhereClause {
        field: "lastName".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text("Smith".to_string()),
    };
    let where_clauses = vec![in_first, eq_middle, eq_last];

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &[],
    )
    .expect("expected picker to accept the 3-prop covering index");
    // Sanity-pin the picker actually chose the 3-prop unique
    // countable index rather than some weaker variant.
    assert_eq!(index.properties.len(), 3);

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses,
    };

    // No-proof: 1 (Alice,M,Smith) + 1 (Bob,M,Smith) = 2.
    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected no-proof count to succeed");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].count,
        Some(2),
        "expected 2 docs covered by firstName IN [Alice, Bob] AND \
         middleName = M AND lastName = Smith"
    );

    // Prove: builder emits compound shape with 2-segment
    // `subquery_path_extension`. Verifier round-trips and returns
    // per-In-value entries.
    let proof = query
        .execute_point_lookup_count_with_proof(&drive, None, platform_version)
        .expect("expected prove count to succeed on In-on-first-of-3 shape");
    assert!(
        !proof.is_empty(),
        "expected non-empty proof bytes for In-on-first-of-3 prove count"
    );

    let (_root_hash, entries) = query
        .verify_point_lookup_count_proof(&proof, platform_version)
        .expect("expected proof verification to succeed");
    let summed: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
    assert_eq!(
        summed, 2,
        "verified per-branch entries should sum to the no-proof total"
    );
}

/// Pins the **absent-In-branch ↔ missing-from-output** contract on a
/// real grovedb proof.
///
/// The `point_lookup_count_path_query` builder does NOT set
/// `absence_proofs_for_non_existing_searched_keys: true` on the outer
/// query (see `path_query.rs`), so grovedb's `verify_query` silently
/// omits absent-`Key` branches from the elements stream rather than
/// emitting `(path, key, None)` triples for them. The verifier therefore
/// emits ZERO entries for absent In values — the request's In array
/// length is the authority on what was asked, and "queried but absent"
/// is detected by the caller diffing the In array against the verified
/// output (cf. [`verify_distinct_count_proof_v0`]'s docstring at the
/// "caller can detect 'I asked for 3 In values but only got entries for
/// 2'" comment).
///
/// This contract makes the `count: Option<u64>` field's `None` variant
/// effectively unreachable on the current path-query shape — it's
/// reserved for a future variant that flips
/// `absence_proofs_for_non_existing_searched_keys: true`. The `elem.map(...)`
/// branch in `verify_point_lookup_count_proof_v0` is forward-compatible
/// code for that variant, not active behavior today.
///
/// Test setup: insert docs at age=30 (×3), age=40 (×2), age=50 (×1);
/// query `age IN [30, 40, 99, 50]` against `byAge`. age=99 has no
/// matching docs and no CountTree element materialized in the merk
/// tree, so grovedb omits that key from the verified elements stream.
///
/// Pins:
/// - **Absent branch (age=99) is silently dropped** — verified entry
///   count is 3, not 4. Caller must diff against the In array if they
///   want to surface absent branches. A regression that emitted a
///   `Some(0)` entry for the absent branch would break the "absence is
///   detected by missing entry, not by zero-count entry" contract this
///   path's docstring documents.
/// - **Present branches → `Some(N)` matching the no-proof totals** —
///   30→3, 40→2, 50→1 — pins that present-branch counts round-trip
///   through real merk proof verification correctly.
/// - **Entry-to-In-value mapping via serialized `key`** — each entry's
///   `key` equals `document_type.serialize_value_for_key("age", &v, …)`
///   for its In value, so callers can demux entries back to the In
///   array without positional assumptions (grovedb sorts by serialized
///   key, not user-input order — see `path_query.rs:391–400`).
///
/// This is the test we'd need to flip the assertion in if/when the path
/// query starts requesting absence proofs — a clear semantic anchor for
/// the future variant.
#[test]
fn test_point_lookup_proof_omits_absent_in_branches_from_entries() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    // Distinct (firstName, middleName, lastName) tuples so the unique
    // `byFirstNameMiddleLastName` index doesn't reject any insert; the
    // count query routes through `byAge` regardless.
    insert_person_doc(&drive, &data_contract, [1u8; 32], "A", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [2u8; 32], "B", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [3u8; 32], "C", "M", "Smith", 30);
    insert_person_doc(&drive, &data_contract, [4u8; 32], "D", "M", "Smith", 40);
    insert_person_doc(&drive, &data_contract, [5u8; 32], "E", "M", "Smith", 40);
    insert_person_doc(&drive, &data_contract, [6u8; 32], "F", "M", "Smith", 50);

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // age IN [30, 40, 99, 50] — 99 is the absent branch. Interleaving
    // it between present values pins that grovedb omits absent keys
    // regardless of position, not just at the array tail.
    let in_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![
            Value::U64(30),
            Value::U64(40),
            Value::U64(99),
            Value::U64(50),
        ]),
    };

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        std::slice::from_ref(&in_clause),
        &[],
    )
    .expect("expected picker to accept byAge for In on age");
    // Sanity-pin the picker chose the single-property `byAge` index —
    // a change here means a future picker rewrite reshaped what counts
    // as "fully covered" for a 1-property In.
    assert_eq!(index.properties.len(), 1);
    assert_eq!(index.properties[0].name.as_str(), "age");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![in_clause],
    };

    let proof = query
        .execute_point_lookup_count_with_proof(&drive, None, platform_version)
        .expect("expected prove count to succeed");
    assert!(!proof.is_empty(), "expected non-empty proof bytes");

    let (_root_hash, entries) = query
        .verify_point_lookup_count_proof(&proof, platform_version)
        .expect("expected proof verification to succeed");

    // The load-bearing assertion: grovedb's `verify_query` (without
    // `absence_proofs_for_non_existing_searched_keys: true`) silently
    // drops absent-Key branches from the elements stream, so the
    // verifier emits 3 entries — one per PRESENT In value — not 4.
    assert_eq!(
        entries.len(),
        3,
        "expected one entry per PRESENT In value (absent branches \
         are omitted, not emitted as Some(0) or None); got {} entries: \
         {:?}",
        entries.len(),
        entries
    );

    // Demux entries by serialized `key` (which is what the verifier
    // populates from `path[base_path_len]`, see
    // `verify_point_lookup_count_proof_v0`). Same serializer the
    // path-query builder uses for outer-Query keys, so by-construction
    // the entry's `key` matches `serialize_value_for_key("age", v)`.
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");
    let key_for = |v: u64| -> Vec<u8> {
        document_type
            .serialize_value_for_key("age", &Value::U64(v), platform_version)
            .expect("serialize age key")
    };

    let find_present = |v: u64| -> u64 {
        let k = key_for(v);
        let matching: Vec<_> = entries.iter().filter(|e| e.key == k).collect();
        assert_eq!(
            matching.len(),
            1,
            "expected exactly one entry for present age={}; got {}: {:?}",
            v,
            matching.len(),
            matching
        );
        matching[0]
            .count
            .expect("present-branch entry must be Some(_), not None")
    };

    // Present branches: real counts, round-tripped through proof bytes.
    assert_eq!(find_present(30), 3, "age=30 has 3 docs");
    assert_eq!(find_present(40), 2, "age=40 has 2 docs");
    assert_eq!(find_present(50), 1, "age=50 has 1 doc");

    // Absent branch: no entry with key=serialize(99). This is the
    // contract — absent branches are detected by the caller as "queried
    // but missing from output", not surfaced via Some(0) or None.
    let absent_key = key_for(99);
    assert!(
        !entries.iter().any(|e| e.key == absent_key),
        "expected NO entry for absent age=99; found one in: {:?}. \
         If this fires after a path-query change, the builder may now \
         request absence proofs — update this test and the verifier \
         docstrings to reflect the new contract.",
        entries
    );
}

/// Boundary-cap test: `|In| = 100` exactly. The 100-element cap on In
/// arrays lives in [`WhereClause::in_values`]; existing tests cover
/// `< 100` (the happy path) and `> 100` (the rejection case at 101,
/// see [`test_count_query_in_operator_rejects_oversized_array`]). Off-
/// by-one in the cap (`>= 100` vs. `> 100`) would silently reject all
/// max-sized queries while passing every smaller test — this pins that
/// 100 is **accepted** end-to-end through both no-proof and prove paths.
///
/// Setup: 100 distinct `age` values (single-property `byAge` countable
/// index, fully covered by an In on `age` alone), with each doc's
/// (firstName, middleName, lastName) tuple distinct so the unique
/// 3-prop index admits all inserts. Each age has exactly one matching
/// doc → total no-proof count = 100; per-branch prove count = 100
/// entries × `Some(1)`.
///
/// Pins:
/// - **`in_values()` accepts |In| = 100** — boundary not off-by-one.
/// - **No-proof per-In fan-out scales to 100 branches** — one
///   `query_aggregate_count` per In value, summed to 100.
/// - **Prove path emits 100 verified entries** — proof reconstruction
///   doesn't hit a hidden inner cap (e.g. a smaller `limit` baked into
///   the path-query builder).
/// - **All 100 entries verify with `Some(1)`** — sum equals no-proof
///   total; per-branch shape matches. Pinning per-entry rather than
///   just the sum catches a regression that would split the count
///   unevenly across branches.
#[test]
fn test_count_query_in_operator_accepts_max_sized_array() {
    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    // 100 distinct ages, each with a unique (firstName, middleName,
    // lastName) tuple so the unique 3-prop index admits all inserts.
    // Using ages 1..=100 keeps the byAge index fully covered by a
    // single In on `age`.
    for i in 0u64..100 {
        let mut id = [0u8; 32];
        id[..2].copy_from_slice(&(i as u16).to_be_bytes());
        // Unique firstName per doc keeps the unique 3-prop index happy
        // regardless of any shared middle/last names.
        let first_name = format!("P{:03}", i);
        insert_person_doc(&drive, &data_contract, id, &first_name, "M", "Smith", i + 1);
    }

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    let in_values: Vec<Value> = (1u64..=100).map(Value::U64).collect();
    assert_eq!(in_values.len(), 100, "test setup invariant: |In| = 100");

    let in_clause = WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(in_values),
    };

    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        std::slice::from_ref(&in_clause),
        &[],
    )
    .expect("expected picker to accept byAge for In on age");
    assert_eq!(index.properties.len(), 1);
    assert_eq!(index.properties[0].name.as_str(), "age");

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index,
        where_clauses: vec![in_clause],
    };

    // No-proof: per-In fan-out, summed. 100 branches × 1 doc each = 100.
    let no_proof = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected no-proof count to accept |In| = 100");
    assert_eq!(
        no_proof.len(),
        1,
        "no-proof returns single aggregated entry"
    );
    assert_eq!(
        no_proof[0].count,
        Some(100),
        "100 distinct age branches × 1 doc each = 100"
    );

    // Prove: verifier emits one entry per PRESENT branch (all 100 are
    // present here, so 100 entries — see
    // `test_point_lookup_proof_omits_absent_in_branches_from_entries`
    // for the absent-branch contract).
    let proof = query
        .execute_point_lookup_count_with_proof(&drive, None, platform_version)
        .expect("expected prove count to accept |In| = 100");
    assert!(
        !proof.is_empty(),
        "expected non-empty proof bytes for 100-element In array"
    );

    let (_root_hash, entries) = query
        .verify_point_lookup_count_proof(&proof, platform_version)
        .expect("expected proof verification to succeed for |In| = 100");

    assert_eq!(
        entries.len(),
        100,
        "verifier emits one entry per present In value at the 100-cap \
         boundary; got {} entries — a smaller count means a hidden \
         inner cap kicked in (e.g. DEFAULT_QUERY_LIMIT on the \
         path-query builder)",
        entries.len()
    );
    let summed: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
    assert_eq!(
        summed, 100,
        "verified per-branch counts should sum to the no-proof total"
    );
    // Every entry must be Some(1) — present branch with one doc.
    // Catches a regression that splits counts unevenly (e.g. a
    // verifier bug that double-counts one branch and zeros another).
    assert!(
        entries.iter().all(|e| e.count == Some(1)),
        "each of the 100 branches has exactly one doc; expected every \
         entry to be Some(1), got: {:?}",
        entries
    );
}

/// Pins the DoS-bound invariant on the compound `range + In`
/// summed no-proof path: per-In aggregate fan-out, NOT a walk-and-
/// sum over every matched `(in_key, key)` element. A regression
/// to walk-and-sum surfaces as a request-amplification on a public
/// unauthenticated endpoint (one broad range × 100 In values can
/// force a full index walk while the response stays a single
/// aggregate `u64`).
///
/// Test invariant: the per-In fan-out gives a correct sum
/// (functional check), and it uses `query_aggregate_count` rather
/// than `query_raw` (DoS-bound check). The functional check pins
/// the result against a known distribution; the DoS-bound check is
/// implicit — `query_aggregate_count` is O(log n) per call vs.
/// `query_raw`'s O(matched elements), and the test data is
/// constructed so a walk would surface a runtime regression (e.g.
/// timeout in CI). We rely on the executor's per-In loop structure
/// as the structural pin; the comment + this test together
/// document the contract.
#[test]
fn test_compound_range_in_summed_no_proof_uses_per_in_aggregate_fanout() {
    use crate::config::DriveConfig;
    use crate::query::drive_document_count_query::drive_dispatcher::{
        DocumentCountRequest, DocumentCountResponse,
    };
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::platform_value;

    const PROTOCOL_VERSION_V12: u32 = 12;

    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();

    // `[brand, color]` compound range_countable index. `brand` is
    // the prefix the test will fan-out an `In` clause across;
    // `color` is the range terminator. The aggregate primitive
    // works on the per-brand `color` subtree directly, so
    // `query_aggregate_count` can answer "how many widgets with
    // brand=X and color > 'blue'" in O(log n) per brand.
    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "brand": {"type": "string", "position": 0, "maxLength": 32},
            "color": {"type": "string", "position": 1, "maxLength": 32},
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
    let data_contract = factory
        .create_with_value_config(
            dpp::tests::utils::generate_random_identifier_struct(),
            0,
            schemas,
            None,
            None,
        )
        .expect("expected to create data contract")
        .data_contract_owned();
    drive
        .apply_contract(
            &data_contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");

    let document_type = data_contract
        .document_type_for_name("widget")
        .expect("widget doc type exists");

    // 3 brands × varying colors, mixing in-range (`color > "blue"`)
    // and out-of-range entries. Expected count for
    // `brand IN [acme, contoso] AND color > "blue"`:
    //   acme: 2 red + 1 green = 3 in-range, 1 blue out  → 3
    //   contoso: 1 red + 2 green = 3 in-range, 0 blue   → 3
    //   stark: 1 red (excluded by In)                   → 0
    // Total = 6.
    let entries = [
        ("acme", "red"),
        ("acme", "red"),
        ("acme", "green"),
        ("acme", "blue"),
        ("contoso", "red"),
        ("contoso", "green"),
        ("contoso", "green"),
        ("stark", "red"),
    ];
    for (i, (brand, color)) in entries.iter().enumerate() {
        let mut properties = StdBTreeMap::new();
        properties.insert("brand".to_string(), Value::Text(brand.to_string()));
        properties.insert("color".to_string(), Value::Text(color.to_string()));
        let document: Document = DocumentV0 {
            contract_version: None,
            id: Identifier::from([(i + 1) as u8; 32]),
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
                    contract: &data_contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert widget");
    }

    // Request: `brand IN ["acme", "contoso"] AND color > "blue"`,
    // no-proof, summed mode. Goes through
    // `execute_range_count_no_proof`'s compound-summed branch,
    // which loops over the In values and issues
    // `query_aggregate_count` per branch.
    let drive_config = DriveConfig::default();
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
    let request = DocumentCountRequest {
        contract: &data_contract,
        document_type,
        where_clauses,
        order_clauses: Vec::new(),
        mode: CountMode::Aggregate,
        limit: None,
        prove: false,
        drive_config: &drive_config,
        resolved_time_ranges: vec![],
    };

    let response = drive
        .execute_document_count_request(request, None, platform_version)
        .expect("expected dispatcher to succeed on compound summed range path");
    let count = match response {
        DocumentCountResponse::Aggregate(c) => c,
        other => panic!("expected Aggregate response, got {:?}", other),
    };
    assert_eq!(
        count, 6,
        "acme(2 red + 1 green) + contoso(1 red + 2 green) = 6 in-range widgets"
    );
}

/// `where_clauses_from_value` must run the parsed `Vec<WhereClause>`
/// through `WhereClause::group_clauses` to reject malformed shapes
/// the regular document-query path rejects.
///
/// Without `group_clauses` validation, the count endpoint silently
/// accepts duplicate/conflicting clauses and returns a count for an
/// arbitrarily reduced query:
/// - Two conflicting `Equal` clauses on the same field collapse to
///   a single clause via `find_countable_index_for_where_clauses`'s
///   `BTreeSet` over field names and `point_lookup_count_path_query`'s
///   `.find(...)` for each index property — the executor picks the
///   first clause and the second is silently dropped.
/// - Multiple `In` clauses or multiple range clauses similarly slip
///   through.
///
/// This test pins the rejection at the dispatcher seam (via the
/// `execute_document_count_request` entry point that all callers
/// reach through the abci handler), so a future change that bypasses
/// the validator gets caught.
#[test]
fn test_count_request_with_duplicate_equality_clauses_is_rejected() {
    use crate::config::DriveConfig;
    use crate::query::drive_document_count_query::drive_dispatcher::DocumentCountRequest;

    let (drive, data_contract) = setup_drive_and_contract();
    let platform_version = PlatformVersion::latest();

    let document_type = data_contract
        .document_type_for_name("person")
        .expect("expected document type");

    // Two conflicting `Equal` clauses on `firstName` — the request
    // is structurally malformed: there's no single document that
    // satisfies both `firstName = "Alice"` AND `firstName = "Bob"`,
    // so the answer should be 0, but a regression would return
    // count("firstName = Alice") or count("firstName = Bob")
    // depending on iteration order.
    let where_clauses = vec![
        WhereClause {
            field: "firstName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("Alice".to_string()),
        },
        WhereClause {
            field: "firstName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("Bob".to_string()),
        },
    ];
    let drive_config = DriveConfig::default();
    let request = DocumentCountRequest {
        contract: &data_contract,
        document_type,
        where_clauses,
        order_clauses: Vec::new(),
        mode: CountMode::Aggregate,
        limit: None,
        prove: false,
        drive_config: &drive_config,
        resolved_time_ranges: vec![],
    };

    let err = drive
        .execute_document_count_request(request, None, platform_version)
        .expect_err("expected duplicate-equality request to be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("duplicate") || msg.contains("DuplicateNonGroupableClauseSameField"),
        "expected duplicate-equality rejection from group_clauses, got: {}",
        msg
    );
}

/// Pins the consensus-sensitive limit-fallback invariant on the
/// `RangeDistinctProof` dispatch path: when the request's `limit`
/// is `None`, the dispatcher MUST fall back to the compile-time
/// `crate::config::DEFAULT_QUERY_LIMIT` constant (which the SDK
/// verifier also reads), NOT the operator-tunable
/// `drive_config.default_query_limit`. The two values are often
/// equal in practice (both default to 100), so a regression where
/// the dispatcher reads from `drive_config.default_query_limit`
/// would only manifest on operators who tuned the runtime value
/// away from the constant — exactly the silent verify-failure
/// surface the CodeRabbit review flagged.
///
/// Mechanism: we build a `DocumentCountRequest` whose
/// `drive_config.default_query_limit` is **deliberately set to 50**
/// (≠ `DEFAULT_QUERY_LIMIT` = 100). If the dispatcher uses
/// `drive_config.default_query_limit`, the proof embeds
/// `SizedQuery::limit = 50`; if it uses `DEFAULT_QUERY_LIMIT`, the
/// proof embeds `SizedQuery::limit = 100`. We then reconstruct the
/// path query with `Some(DEFAULT_QUERY_LIMIT)` — exactly what the
/// SDK verifier does — and run `GroveDb::verify_query` on the
/// proof bytes. The merk-root recomputation only succeeds if the
/// prover signed with `limit = 100`; if it signed with `limit = 50`
/// the reconstructed path query bytes differ and `verify_query`
/// returns an error.
///
/// Without the fix in `drive_dispatcher.rs`'s `RangeDistinctProof`
/// arm this test fails. The "fix" is the one-line change from
/// `request.drive_config.default_query_limit` to
/// `crate::config::DEFAULT_QUERY_LIMIT` on the prove path — see
/// the comment in that arm for the symmetric reasoning.
#[test]
fn test_range_distinct_proof_uses_compile_time_default_query_limit_not_operator_config() {
    use crate::config::{DriveConfig, DEFAULT_QUERY_LIMIT};
    use crate::query::drive_document_count_query::drive_dispatcher::{
        DocumentCountRequest, DocumentCountResponse,
    };
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::platform_value;
    use grovedb::GroveDb;

    const PROTOCOL_VERSION_V12: u32 = 12;
    // Set the operator's tuned limit to **1** — a value small
    // enough that the prover's walk would actually stop after one
    // element instead of just covering the entire result set
    // (which 50 or 100 both would, masking any limit-mismatch by
    // producing identical proof bytes). With 2+ in-range distinct
    // keys below and `OPERATOR_TUNED_LIMIT = 1`, the prover-side
    // limit choice **materially affects which elements end up in
    // the proof** and the merk-root recomputation. If the
    // dispatcher (incorrectly) used `default_query_limit = 1`,
    // the prover would emit a 1-key proof; the verifier
    // (rebuilding with `DEFAULT_QUERY_LIMIT = 100`) would expect
    // up to 100 keys and the boundary-subtree hash chain would
    // not match → `verify_query` returns Err.
    const OPERATOR_TUNED_LIMIT: u16 = 1;
    assert_ne!(
        DEFAULT_QUERY_LIMIT, OPERATOR_TUNED_LIMIT,
        "test invariant: OPERATOR_TUNED_LIMIT must differ from the \
         compile-time DEFAULT_QUERY_LIMIT for the regression check \
         to be load-bearing"
    );

    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();

    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "color": {"type": "string", "position": 0, "maxLength": 32},
        },
        "indices": [{
            "name": "byColor",
            "properties": [{"color": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "widget": document_schema });
    let data_contract = factory
        .create_with_value_config(
            dpp::tests::utils::generate_random_identifier_struct(),
            0,
            schemas,
            None,
            None,
        )
        .expect("expected to create data contract")
        .data_contract_owned();

    drive
        .apply_contract(
            &data_contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");

    let document_type = data_contract
        .document_type_for_name("widget")
        .expect("widget doc type exists");

    // Spread docs across distinct color values so the
    // RangeDistinctProof path actually carries per-key counts in
    // its proof (an empty range would still verify trivially and
    // mask the limit mismatch). 2 red + 3 green + 1 blue; the
    // `color > "blue"` clause excludes blue, leaving 2 distinct
    // in-range keys (red, green).
    for (i, color) in ["red", "red", "green", "green", "green", "blue"]
        .iter()
        .enumerate()
    {
        let mut properties = StdBTreeMap::new();
        properties.insert("color".to_string(), Value::Text(color.to_string()));
        let document: Document = DocumentV0 {
            contract_version: None,
            id: Identifier::from([(i + 1) as u8; 32]),
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
                    contract: &data_contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert widget");
    }

    // Operator-tuned DriveConfig with `default_query_limit = 50`.
    // The dispatcher MUST NOT propagate this onto the prove path's
    // path query.
    let drive_config = DriveConfig {
        default_query_limit: OPERATOR_TUNED_LIMIT,
        ..Default::default()
    };

    // Range clause `color > "blue"` as a typed WhereClause —
    // the dispatcher runs validate-and-canonicalize internally and
    // dispatches to the RangeDistinctProof path on `prove=true`.
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("blue".to_string()),
    }];
    let request = DocumentCountRequest {
        contract: &data_contract,
        document_type,
        where_clauses,
        order_clauses: Vec::new(),
        mode: CountMode::GroupByRange,
        limit: None,
        prove: true,
        drive_config: &drive_config,
        resolved_time_ranges: vec![],
    };

    let response = drive
        .execute_document_count_request(request, None, platform_version)
        .expect("expected dispatcher to succeed on RangeDistinctProof path");
    let proof_bytes = match response {
        DocumentCountResponse::Proof(p) => p,
        other => panic!("expected Proof response, got {:?}", other),
    };
    assert!(
        !proof_bytes.is_empty(),
        "expected non-empty proof bytes from RangeDistinctProof path"
    );

    // Reconstruct the path query the way the SDK verifier does:
    // anchored to the compile-time `DEFAULT_QUERY_LIMIT`, not the
    // operator's runtime value. If the dispatcher used
    // `OPERATOR_TUNED_LIMIT` instead, the reconstructed path
    // query's `SizedQuery::limit` bytes will differ from what the
    // prover signed and `verify_query` returns Err.
    let color_gt_blue = WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Text("blue".to_string()),
    };
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        std::slice::from_ref(&color_gt_blue),
        &[],
    )
    .expect("byColor range_countable index covers `color > blue`");
    let count_query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "widget".to_string(),
        index,
        where_clauses: vec![color_gt_blue],
    };
    let verifier_path_query = count_query
        .distinct_count_path_query(Some(DEFAULT_QUERY_LIMIT), true, platform_version)
        .expect("path query builder should accept the same shape the prover used");

    let (_root_hash, _elements) = GroveDb::verify_query(
        &proof_bytes,
        &verifier_path_query,
        &platform_version.drive.grove_version,
    )
    .expect(
        "expected proof to verify against a path query rebuilt with \
         DEFAULT_QUERY_LIMIT; a failure here means the dispatcher signed \
         the proof with the operator-tunable default_query_limit instead — \
         a consensus-adjacent silent-verify-failure regression",
    );
}

/// A `max_query_limit: 0` config (or an explicit `limit: 0`) must fail
/// closed on the no-proof distinct walk instead of reaching storage
/// with a zero bound, which grovedb treats as "return nothing" and
/// which would masquerade as an empty result set. Mirrors the sum-side
/// `effective_no_proof_distinct_limit` policy.
#[test]
fn test_range_distinct_no_proof_rejects_zero_effective_limit() {
    use crate::config::DriveConfig;
    use crate::error::query::QuerySyntaxError;
    use crate::error::Error;
    use crate::query::drive_document_count_query::drive_dispatcher::DocumentCountRequest;
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::platform_value;

    const PROTOCOL_VERSION_V12: u32 = 12;

    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();

    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "color": {"type": "string", "position": 0, "maxLength": 32},
        },
        "indices": [{
            "name": "byColor",
            "properties": [{"color": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "widget": document_schema });
    let data_contract = factory
        .create_with_value_config(
            dpp::tests::utils::generate_random_identifier_struct(),
            0,
            schemas,
            None,
            None,
        )
        .expect("expected to create data contract")
        .data_contract_owned();
    let document_type = data_contract
        .document_type_for_name("widget")
        .expect("widget doc type exists");

    // The rejection fires in the dispatcher before any storage walk,
    // so the contract does not need to be applied nor documents
    // inserted for this check to be load-bearing.
    let drive_config = DriveConfig {
        max_query_limit: 0,
        ..Default::default()
    };
    let request = DocumentCountRequest {
        contract: &data_contract,
        document_type,
        where_clauses: vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        }],
        order_clauses: Vec::new(),
        mode: CountMode::GroupByRange,
        limit: None,
        prove: false,
        drive_config: &drive_config,
        resolved_time_ranges: vec![],
    };

    let result = drive.execute_document_count_request(request, None, platform_version);
    match result {
        Err(Error::Query(QuerySyntaxError::InvalidLimit(msg))) => {
            assert!(
                msg.contains("greater than zero"),
                "error must state the zero-limit rejection; got: {msg}"
            );
        }
        other => panic!("expected InvalidLimit error, got {other:?}"),
    }
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
            &[],
            super::RangeCountOptions {
                distinct: false,
                limit: Some(50),
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

    // Use canonical Deserialize (no schema validation — see
    // `data_contract/conversion/serde/mod.rs` for the no-validation-by-default
    // policy). The earlier `from_json(_, false, _)` legacy method was deleted
    // when the `_versioned` family collapsed into canonical + `_validated`.
    let _ = platform_version;
    let data_contract: dpp::data_contract::DataContract = serde_json::from_value(contract_json)
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

    // 2 Alices + 2 Bobs so the byFirstName count at "Alice" is 2.
    // Using fully-covered `firstName == "Alice"` because the strict
    // picker requires exact coverage.
    insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "", "", 30);
    insert_person_doc(&drive, &data_contract, [2u8; 32], "Alice", "", "", 31);
    insert_person_doc(&drive, &data_contract, [3u8; 32], "Bob", "", "", 40);
    insert_person_doc(&drive, &data_contract, [4u8; 32], "Bob", "", "", 41);

    let first_name_eq_alice = WhereClause {
        field: "firstName".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text("Alice".to_string()),
    };
    // The picker should accept this index — `is_countable()` covers
    // both `Countable` and `CountableAllowingOffset` variants.
    let picked = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        std::slice::from_ref(&first_name_eq_alice),
        &[],
    )
    .expect("expected picker to accept CountableAllowingOffset index");
    assert_eq!(picked.countable, IndexCountability::CountableAllowingOffset);

    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: data_contract.id().to_buffer(),
        document_type_name: "person".to_string(),
        index: picked,
        where_clauses: vec![first_name_eq_alice],
    };

    let results = query
        .execute_no_proof(&drive, None, platform_version)
        .expect("expected count query to succeed against ProvableCountTree");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].count,
        Some(2),
        "ProvableCountTree should report 2 Alices"
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
        &[],
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
        results[0].count,
        Some(1),
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
            // Sum-axis: count-picker tests don't drive sum behaviour;
            // keep the index sum-disabled so the matrix collapses to
            // the count-only sub-cube. Setting `summable: None` is
            // sufficient to take the count-only path through every
            // tree-shape resolver (see `primary_key_tree_type.rs`).
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_countable_at: vec![],
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
            skip_if_absent: false,
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
            &[],
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
            &[],
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
                &[],
            )
            .is_none(),
            "a range on a non-terminator property must not match — the storage \
             layout doesn't put a ProvableCountTree at that level"
        );
    }

    /// Carrier arm (two ranges on distinct fields): an extra Equal clause
    /// on a field the index does not carry must disqualify the index —
    /// the carrier path-query builder iterates only index properties, so
    /// an admitted index would silently drop the clause and produce an
    /// over-broad per-group count that still verifies (the verifier
    /// rebuilds the same path query from the same picker). Mirrors sum's
    /// strict-coverage guard.
    #[test]
    fn carrier_arm_rejects_index_missing_an_equality_field() {
        let indexes = make_indexes(vec![make_index(
            "byBrandColor",
            &["brand", "color"],
            IndexCountability::Countable,
            true,
        )]);
        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Text("a".to_string()),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Text("f".to_string()),
            },
            WhereClause {
                field: "material".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("wood".to_string()),
            },
        ];
        assert!(
            DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                &indexes,
                &where_clauses,
                &[],
            )
            .is_none(),
            "an equality clause the index cannot cover must disqualify it"
        );
    }

    /// Positive control for the strict-coverage guard: the identical
    /// query shape against an index that carries the equality field as
    /// its intermediate property is covered and picked.
    #[test]
    fn carrier_arm_picks_index_covering_the_equality_field() {
        let indexes = make_indexes(vec![make_index(
            "byBrandMaterialColor",
            &["brand", "material", "color"],
            IndexCountability::Countable,
            true,
        )]);
        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Text("a".to_string()),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Text("f".to_string()),
            },
            WhereClause {
                field: "material".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("wood".to_string()),
            },
        ];
        let picked = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            &indexes,
            &where_clauses,
            &[],
        );
        assert_eq!(
            picked.map(|index| index.name.as_str()),
            Some("byBrandMaterialColor"),
            "with the equality field covered, the carrier index is picked"
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
                &[],
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
                &[],
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
                &[],
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
        let mode = DriveDocumentCountQuery::detect_mode(&[], CountMode::Aggregate, false).unwrap();
        assert_eq!(mode, DocumentCountMode::Total);
    }

    /// Equal-only clauses → still total.
    #[test]
    fn only_equal_clauses_is_total() {
        let clauses = vec![eq_clause("a"), eq_clause("b")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, false).unwrap(),
            DocumentCountMode::Total,
        );
    }

    /// Single In clause → per-In-value.
    #[test]
    fn single_in_is_per_in_value() {
        let clauses = vec![in_clause("a")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, false).unwrap(),
            DocumentCountMode::PerInValue,
        );
    }

    /// Equal + In on different fields → per-In-value.
    #[test]
    fn equal_plus_in_is_per_in_value() {
        let clauses = vec![eq_clause("a"), in_clause("b")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, false).unwrap(),
            DocumentCountMode::PerInValue,
        );
    }

    /// Single range + no proof → range no-proof.
    #[test]
    fn single_range_no_proof_is_range_no_proof() {
        let clauses = vec![gt_clause("color")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, false).unwrap(),
            DocumentCountMode::RangeNoProof,
        );
    }

    /// Single range + prove → range proof.
    #[test]
    fn single_range_with_prove_is_range_proof() {
        let clauses = vec![gt_clause("color")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, true).unwrap(),
            DocumentCountMode::RangeProof,
        );
    }

    /// No range + prove → point-lookup proof (materialize-and-count).
    #[test]
    fn no_range_with_prove_is_point_lookup_proof() {
        let clauses = vec![eq_clause("a")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, true).unwrap(),
            DocumentCountMode::PointLookupProof,
        );
    }

    /// Equal-prefix + range terminator + no proof → range no-proof.
    #[test]
    fn equal_prefix_plus_range_terminator_is_range_no_proof() {
        let clauses = vec![eq_clause("brand"), gt_clause("color")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, false).unwrap(),
            DocumentCountMode::RangeNoProof,
        );
    }

    /// Two range operators → rejected.
    #[test]
    fn two_range_operators_rejected() {
        let clauses = vec![gt_clause("color"), lt_clause("color")];
        let err = DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, false)
            .unwrap_err();
        assert!(matches!(
            err,
            QuerySyntaxError::InvalidWhereClauseComponents(msg) if msg.contains("at most one range")
        ));
    }

    /// Two `In` operators → rejected.
    #[test]
    fn two_in_operators_rejected() {
        let clauses = vec![in_clause("a"), in_clause("b")];
        let err = DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, false)
            .unwrap_err();
        assert!(matches!(
            err,
            QuerySyntaxError::InvalidWhereClauseComponents(msg) if msg.contains("at most one `in`")
        ));
    }

    /// Range + In together → routed by mode:
    /// - `(range, In, no-proof, _)` → `RangeNoProof` (executor uses
    ///   `distinct_count_path_query`'s compound shape with grovedb
    ///   subqueries to cartesian-fork over the In values).
    /// - `(range, In, prove, distinct=true)` → `RangeDistinctProof`
    ///   (same compound shape, just runs through the prove path).
    /// - `(range, In, prove, distinct=false)` → **rejected** because
    ///   grovedb's `AggregateCountOnRange` primitive wraps a single
    ///   inner range and can't cartesian-fork at the merk layer.
    #[test]
    fn range_plus_in_routes_by_mode() {
        let clauses = vec![in_clause("a"), gt_clause("b")];

        // No-proof — both sum and distinct route through RangeNoProof,
        // which uses the unified `distinct_count_path_query` builder
        // and applies `options.distinct` in post-processing.
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, false).unwrap(),
            DocumentCountMode::RangeNoProof,
        );
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::GroupByRange, false).unwrap(),
            DocumentCountMode::RangeNoProof,
        );

        // Prove + distinct — routes to RangeDistinctProof. The path
        // query carries In as outer `Key`s and the range as the
        // subquery; the verifier reconstructs the same shape.
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::GroupByRange, true).unwrap(),
            DocumentCountMode::RangeDistinctProof,
        );

        // Prove + !distinct (aggregate) — still rejected, the
        // AggregateCountOnRange primitive can't fork.
        let err =
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, true).unwrap_err();
        assert!(
            matches!(
                err,
                QuerySyntaxError::InvalidWhereClauseComponents(msg)
                    if msg.contains("not supported on the aggregate prove path")
            ),
            "expected aggregate-prove rejection, got: {:?}",
            err,
        );
    }

    /// `CountMode::GroupByRange` without a range clause → rejected.
    #[test]
    fn distinct_without_range_rejected() {
        let err =
            DriveDocumentCountQuery::detect_mode(&[], CountMode::GroupByRange, false).unwrap_err();
        assert!(matches!(
            err,
            QuerySyntaxError::InvalidWhereClauseComponents(msg) if msg.contains("requires a range where-clause")
        ));
    }

    /// `CountMode::GroupByRange` + `prove = true` →
    /// `RangeDistinctProof`. Per-distinct-value counts come from a
    /// regular range proof against the property-name
    /// `ProvableCountTree` (no `AggregateCountOnRange` wrapper), with
    /// `KVCount(key, value, count)` ops bound to the merk root via
    /// `node_hash_with_count`. The verifier extracts them as a
    /// `BTreeMap<Vec<u8>, u64>`.
    #[test]
    fn distinct_with_prove_is_range_distinct_proof() {
        let clauses = vec![gt_clause("color")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::GroupByRange, true).unwrap(),
            DocumentCountMode::RangeDistinctProof,
        );
    }

    /// Distinct mode in no-prove range → still RangeNoProof; the
    /// distinct flag is consumed by the executor, not the mode tag.
    #[test]
    fn distinct_no_prove_with_range_is_range_no_proof() {
        let clauses = vec![gt_clause("color")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::GroupByRange, false).unwrap(),
            DocumentCountMode::RangeNoProof,
        );
    }

    /// `prove = true` + `In` routes to `PointLookupProof` — the
    /// CountTree-element proof primitive. The
    /// `point_lookup_count_path_query` builder emits one
    /// `Element::CountTree` per matched In branch; the verifier
    /// reads `count_value_or_default()` off each verified element
    /// directly. No document materialization, no `u16::MAX` cap on
    /// matching docs. Proof size is O(|In values| × log n).
    #[test]
    fn in_with_prove_routes_to_point_lookup_proof() {
        let clauses = vec![in_clause("a")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::Aggregate, true).unwrap(),
            DocumentCountMode::PointLookupProof,
        );
    }

    /// `GroupByRange + prove + two range clauses on distinct fields`
    /// routes to `RangeAggregateCarrierProof` (the carrier-ACOR with
    /// outer Range shape — chapter 30 G8). The dispatcher applies a
    /// platform-wide max outer-walk cap via
    /// [`MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT`], with caller
    /// semantics tested at the dispatcher level.
    #[test]
    fn outer_range_plus_inner_range_with_prove_and_group_by_range_routes_to_carrier_proof() {
        let clauses = vec![gt_clause("brand"), gt_clause("color")];
        assert_eq!(
            DriveDocumentCountQuery::detect_mode(&clauses, CountMode::GroupByRange, true).unwrap(),
            DocumentCountMode::RangeAggregateCarrierProof,
        );
    }

    /// Two range clauses on the SAME field are still rejected — the
    /// "two ranges on distinct fields" carrier escape hatch requires
    /// the ranges to be on different properties (one outer, one
    /// terminator). Same-field two-sided ranges flatten through the
    /// upstream parser into `between*` and arrive here as one clause.
    #[test]
    fn two_ranges_on_same_field_with_group_by_range_prove_still_rejected() {
        let clauses = vec![gt_clause("color"), lt_clause("color")];
        let err = DriveDocumentCountQuery::detect_mode(&clauses, CountMode::GroupByRange, true)
            .unwrap_err();
        assert!(matches!(
            err,
            QuerySyntaxError::InvalidWhereClauseComponents(_)
        ));
    }

    /// No-proof path keeps the original `range_count > 1` rejection
    /// — the carrier escape hatch is gated on `prove = true` because
    /// the no-proof variant doesn't have a corresponding executor
    /// yet. (Documenting the gate so a future no-proof carrier wire-
    /// up doesn't silently slip past `detect_mode`'s exhaustiveness.)
    #[test]
    fn two_ranges_no_proof_with_group_by_range_still_rejected() {
        let clauses = vec![gt_clause("brand"), gt_clause("color")];
        let err = DriveDocumentCountQuery::detect_mode(&clauses, CountMode::GroupByRange, false)
            .unwrap_err();
        assert!(matches!(
            err,
            QuerySyntaxError::InvalidWhereClauseComponents(_)
        ));
    }
}

/// Coverage for the rangeCountable-terminator optimization on the
/// point-lookup proof path. See
/// [`DriveDocumentCountQuery::point_lookup_count_path_query`] for
/// the two-shape rationale.
///
/// These tests pin **three** axes:
///
/// 1. **Counts are unchanged** — the optimization is a proof-size
///    win, not a semantic change. Every shape's no-proof and prove
///    paths must agree on the per-branch counts before and after.
/// 2. **Path-query shape diverges between countable and
///    rangeCountable** — explicit structural assertions on
///    `PathQuery.path` / `Query.items` / `default_subquery_branch`
///    so a regression that re-introduces the `[0]` descent for
///    rangeCountable (or, worse, drops it for normal countable)
///    fails loudly here rather than only showing up as a wrong
///    proof size at runtime.
/// 3. **Non-rangeCountable shape preserved** — the byAge regression
///    test pins the unchanged `Key([0])` selector so the
///    optimization isn't accidentally applied to indexes whose
///    value trees are NormalTree (where `[0]` is load-bearing for
///    finding the count).
///
/// We assert path-query shape directly rather than relying on proof
/// size to surface regressions, because proof-size measurements
/// fluctuate with merk-tree balance and only catch the regression
/// stochastically. The shape assertion is deterministic and points
/// at the exact line that drifted.
#[cfg(all(feature = "server", feature = "verify"))]
mod range_countable_point_lookup_tests {
    use super::*;
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use dpp::data_contract::DataContract;
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::platform_value;
    use grovedb::QueryItem;

    const PROTOCOL_VERSION_V12: u32 = 12;

    /// Build a `widget` document type with a single `byBrand` index
    /// flagged `range_countable: true`. The terminator's value
    /// trees are CountTrees (rather than NormalTree + `[0]`-child
    /// CountTree), so the point-lookup proof should target them
    /// directly.
    fn build_by_brand_range_countable_contract() -> DataContract {
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "brand": {"type": "string", "position": 0, "maxLength": 32},
            },
            "indices": [{
                "name": "byBrand",
                "properties": [{"brand": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned()
    }

    /// Build a `widget` document type with a compound `byBrandColor`
    /// index flagged `range_countable: true`. The terminator is
    /// `color`; only its value trees are CountTrees. The intermediate
    /// `brand` value trees stay NormalTree (because they're not the
    /// terminator), so the optimization is only legal when the proof
    /// resolves *down to* the `color` value tree — which is exactly
    /// what `brand IN [..] AND color = X` does.
    fn build_by_brand_color_range_countable_contract() -> DataContract {
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "brand": {"type": "string", "position": 0, "maxLength": 32},
                "color": {"type": "string", "position": 1, "maxLength": 32},
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
        factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned()
    }

    /// Build a `gizmo` document type with a single `byCategory`
    /// index that is `countable: true` but **NOT** `range_countable`.
    /// Used as the regression control — its value trees stay
    /// `NormalTree` and the count lives at the `[0]` child, so the
    /// point-lookup path query must continue to use `Key([0])`.
    fn build_by_category_normal_countable_contract() -> DataContract {
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "category": {"type": "string", "position": 0, "maxLength": 32},
            },
            "indices": [{
                "name": "byCategory",
                "properties": [{"category": "asc"}],
                "countable": "countable",
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "gizmo": document_schema });
        factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned()
    }

    /// Insert a widget doc with the given `(brand, color)`. `color`
    /// may be `None` for single-property `byBrand` fixtures.
    fn insert_widget(
        drive: &Drive,
        data_contract: &DataContract,
        id: [u8; 32],
        brand: &str,
        color: Option<&str>,
    ) {
        let platform_version = PlatformVersion::latest();
        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget doc type");

        let mut properties = StdBTreeMap::new();
        properties.insert("brand".to_string(), Value::Text(brand.to_string()));
        if let Some(c) = color {
            properties.insert("color".to_string(), Value::Text(c.to_string()));
        }
        let document: Document = DocumentV0 {
            contract_version: None,
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
            .expect("insert widget");
    }

    /// Insert a gizmo doc with a `category` property. Mirror of
    /// [`insert_widget`] for the normal-countable regression fixture.
    fn insert_gizmo(drive: &Drive, data_contract: &DataContract, id: [u8; 32], category: &str) {
        let platform_version = PlatformVersion::latest();
        let document_type = data_contract
            .document_type_for_name("gizmo")
            .expect("gizmo doc type");

        let mut properties = StdBTreeMap::new();
        properties.insert("category".to_string(), Value::Text(category.to_string()));
        let document: Document = DocumentV0 {
            contract_version: None,
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
            .expect("insert gizmo");
    }

    /// **Equal-only rangeCountable**: `brand == "acme"` against
    /// single-property `byBrand` (rangeCountable). The path query
    /// must stop *one segment short* of the legacy shape — at
    /// `[..., "brand"]` with the query asking for
    /// `Key(serialize("acme"))` — so the resolved element is the
    /// terminator value tree itself (a CountTree). The legacy shape
    /// would have descended to `[..., "brand", serialize("acme")]`
    /// + `Key([0])`, which adds a redundant merk layer.
    #[test]
    fn equal_only_rangecountable_path_query_targets_value_tree_directly() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_by_brand_range_countable_contract();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        // 3 acme + 2 contoso so we have a non-trivial per-brand count
        // to verify against.
        insert_widget(&drive, &data_contract, [1u8; 32], "acme", None);
        insert_widget(&drive, &data_contract, [2u8; 32], "acme", None);
        insert_widget(&drive, &data_contract, [3u8; 32], "acme", None);
        insert_widget(&drive, &data_contract, [4u8; 32], "contoso", None);
        insert_widget(&drive, &data_contract, [5u8; 32], "contoso", None);

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let brand_eq = WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("acme".to_string()),
        };
        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            std::slice::from_ref(&brand_eq),
            &[],
        )
        .expect("byBrand covers brand==acme");
        assert!(
            index.range_countable,
            "fixture: byBrand must be rangeCountable for this test to exercise the optimization"
        );
        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses: vec![brand_eq.clone()],
        };

        // Shape assertion: path stops at `[..., "brand"]`, query
        // selects `Key(serialize("acme"))`.
        let path_query = query
            .point_lookup_count_path_query(platform_version)
            .expect("path query builds");
        // Path: [DataContractDocuments, contract_id, 1, "widget",
        //        "brand"] — 5 segments, last one is the prop name.
        assert_eq!(
            path_query.path.last().expect("non-empty path"),
            &b"brand".to_vec(),
            "rangeCountable Equal-only path must end at the property-name \
             subtree, NOT at the serialized value (which would re-introduce \
             the `[0]` descent)"
        );
        let serialized_acme = document_type
            .serialize_value_for_key("brand", &Value::Text("acme".to_string()), platform_version)
            .expect("serialize brand key");
        let items = &path_query.query.query.items;
        assert_eq!(items.len(), 1, "single Key item for Equal-only");
        assert_eq!(
            items[0],
            QueryItem::Key(serialized_acme.clone()),
            "Equal-only rangeCountable selector must be Key(serialize(value)) — \
             a regression to Key([0]) would mean the optimization was reverted"
        );
        assert_ne!(
            items[0],
            QueryItem::Key(vec![0]),
            "Key([0]) is the normal-countable selector and must NOT appear here"
        );
        let subquery_branch = &path_query.query.query.default_subquery_branch;
        assert!(
            subquery_branch.subquery.is_none() && subquery_branch.subquery_path.is_none(),
            "Equal-only rangeCountable must not set a subquery (the resolved \
             element IS the count-bearing value tree)"
        );

        // Counts match: no-proof and prove agree, both report 3.
        let no_proof = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("no-proof");
        assert_eq!(no_proof.len(), 1);
        assert_eq!(no_proof[0].count, Some(3), "acme has 3 widgets");

        let proof_bytes = query
            .execute_point_lookup_count_with_proof(&drive, None, platform_version)
            .expect("prove count");
        assert!(!proof_bytes.is_empty());
        let (_root_hash, entries) = query
            .verify_point_lookup_count_proof(&proof_bytes, platform_version)
            .expect("verify");
        let summed: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
        assert_eq!(
            summed, 3,
            "rangeCountable Equal-only verified count must equal the no-proof \
             total — different merk layer, same answer"
        );
    }

    /// **In-on-terminator rangeCountable**: `brand IN [acme, contoso,
    /// absent]` against single-property `byBrand` (rangeCountable).
    /// Outer Keys land directly on CountTree value trees; no
    /// subquery is set. The verifier picks up the In value from
    /// `grove_key` (since `path.len() == base_path_len`) rather than
    /// `path[base_path_len]` like the normal-countable shape.
    #[test]
    fn in_on_rangecountable_terminator_path_query_has_no_subquery() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_by_brand_range_countable_contract();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        insert_widget(&drive, &data_contract, [1u8; 32], "acme", None);
        insert_widget(&drive, &data_contract, [2u8; 32], "acme", None);
        insert_widget(&drive, &data_contract, [3u8; 32], "contoso", None);
        // Note: no `absent` widgets — pins the "absent branches
        // silently omitted" contract for the new shape too.

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let brand_in = WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("acme".to_string()),
                Value::Text("contoso".to_string()),
                Value::Text("absent".to_string()),
            ]),
        };
        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            std::slice::from_ref(&brand_in),
            &[],
        )
        .expect("byBrand covers brand IN [...]");
        assert!(index.range_countable);
        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses: vec![brand_in.clone()],
        };

        let path_query = query
            .point_lookup_count_path_query(platform_version)
            .expect("path query builds");
        assert_eq!(
            path_query.path.last().expect("non-empty path"),
            &b"brand".to_vec(),
            "In-on-terminator rangeCountable: path stops at the property-name \
             subtree (`[..., \"brand\"]`); outer Keys enumerate the In values"
        );
        let items = &path_query.query.query.items;
        assert_eq!(
            items.len(),
            3,
            "expected one outer Key per In value (acme, contoso, absent)"
        );
        for it in items {
            assert!(
                matches!(it, QueryItem::Key(_)),
                "outer items must all be Key(_) — got {:?}",
                it
            );
        }
        let subquery_branch = &path_query.query.query.default_subquery_branch;
        assert!(
            subquery_branch.subquery.is_none() && subquery_branch.subquery_path.is_none(),
            "In-on-rangeCountable-terminator must not set a subquery — the outer \
             Keys resolve directly to the value-tree CountTrees. \
             A regression that sets `Key([0])` as the subquery would silently \
             work (because grovedb would still find the CountTree under `[0]`) \
             but emits a bigger proof — exactly what this optimization aims \
             to avoid."
        );

        // End-to-end correctness.
        let no_proof = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("no-proof");
        // Per-In fan-out aggregates into a single summed entry on
        // the no-proof side.
        assert_eq!(no_proof.len(), 1);
        assert_eq!(no_proof[0].count, Some(3), "2 acme + 1 contoso = 3");

        let proof_bytes = query
            .execute_point_lookup_count_with_proof(&drive, None, platform_version)
            .expect("prove count");
        let (_root_hash, entries) = query
            .verify_point_lookup_count_proof(&proof_bytes, platform_version)
            .expect("verify");

        // Absent branches are omitted, so only the 2 present brands
        // surface — same omission semantics as the normal-countable
        // path (see `test_point_lookup_proof_omits_absent_in_branches_from_entries`).
        assert_eq!(entries.len(), 2);
        let summed: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
        assert_eq!(summed, 3);

        // Per-entry sanity: each entry's `key` is the serialized In
        // value (lifted from `grove_key` by the verifier).
        let key_acme = document_type
            .serialize_value_for_key("brand", &Value::Text("acme".to_string()), platform_version)
            .expect("serialize acme");
        let key_contoso = document_type
            .serialize_value_for_key(
                "brand",
                &Value::Text("contoso".to_string()),
                platform_version,
            )
            .expect("serialize contoso");
        let acme_entry = entries
            .iter()
            .find(|e| e.key == key_acme)
            .expect("acme entry present");
        assert_eq!(acme_entry.count, Some(2));
        let contoso_entry = entries
            .iter()
            .find(|e| e.key == key_contoso)
            .expect("contoso entry present");
        assert_eq!(contoso_entry.count, Some(1));
    }

    /// **Compound rangeCountable**: `brand IN [acme, contoso] AND
    /// color = "red"` against `byBrandColor` (rangeCountable
    /// terminator = `color`). The In is on a prefix and `color` is
    /// the trailing Equal; the optimization lifts the terminator
    /// value into the subquery's `Key(serialize("red"))` so the
    /// subquery_path ends at the terminator's property-name segment
    /// `["color"]` rather than `["color", serialize("red")]`.
    ///
    /// This shape is the one most likely to drift in a refactor —
    /// the trailing-Equal loop in `point_lookup_count_path_query`
    /// pushes `(name, value)` pairs into `subquery_path_extension`,
    /// and the optimization pops the last value out at the end. A
    /// regression that forgets to pop (or pops the wrong element)
    /// would silently produce a bigger proof or a wrong path query.
    #[test]
    fn compound_in_prefix_plus_trailing_equal_on_rangecountable_terminator() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_by_brand_color_range_countable_contract();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        // (brand, color):
        //   acme/red   ×3, acme/blue   ×1
        //   contoso/red ×2, contoso/green ×1
        //   stark/red   ×1 (excluded by In)
        // Expected for `brand IN [acme, contoso] AND color = red`:
        //   acme: 3, contoso: 2, total: 5.
        let docs = [
            ("acme", "red"),
            ("acme", "red"),
            ("acme", "red"),
            ("acme", "blue"),
            ("contoso", "red"),
            ("contoso", "red"),
            ("contoso", "green"),
            ("stark", "red"),
        ];
        for (i, (brand, color)) in docs.iter().enumerate() {
            insert_widget(
                &drive,
                &data_contract,
                [(i + 1) as u8; 32],
                brand,
                Some(color),
            );
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let brand_in = WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("acme".to_string()),
                Value::Text("contoso".to_string()),
            ]),
        };
        let color_eq = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("red".to_string()),
        };
        let clauses = vec![brand_in, color_eq];
        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &clauses,
            &[],
        )
        .expect("byBrandColor covers brand IN + color =");
        assert!(index.range_countable);
        assert_eq!(index.properties.len(), 2);
        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses: clauses,
        };

        let path_query = query
            .point_lookup_count_path_query(platform_version)
            .expect("path query builds");
        // base_path ends at `[..., "brand"]` (the In-bearing prop's
        // property-name subtree).
        assert_eq!(
            path_query.path.last().expect("non-empty path"),
            &b"brand".to_vec()
        );

        // Subquery shape: `set_subquery_path = ["color"]`,
        // `subquery.items = [Key(serialize("red"))]`. The legacy
        // shape would have had `set_subquery_path = ["color",
        // serialize("red")]` + `subquery.items = [Key([0])]`.
        let subquery_branch = &path_query.query.query.default_subquery_branch;
        let subquery_path = subquery_branch
            .subquery_path
            .as_ref()
            .expect("compound rangeCountable trailing Equal must set subquery_path");
        assert_eq!(
            subquery_path,
            &vec![b"color".to_vec()],
            "subquery_path must end at the terminator's property-name segment \
             (`color`), with the terminator's serialized value lifted into \
             the subquery's Key — a regression that left the value here would \
             re-introduce the `[0]` descent"
        );
        let subquery = subquery_branch
            .subquery
            .as_ref()
            .expect("compound rangeCountable must set subquery");
        let serialized_red = document_type
            .serialize_value_for_key("color", &Value::Text("red".to_string()), platform_version)
            .expect("serialize color key");
        assert_eq!(subquery.items.len(), 1);
        assert_eq!(
            subquery.items[0],
            QueryItem::Key(serialized_red),
            "subquery selector must be Key(serialize(terminator_value)) — \
             NOT Key([0])"
        );
        assert_ne!(subquery.items[0], QueryItem::Key(vec![0]));

        // Correctness end-to-end.
        let no_proof = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("no-proof");
        assert_eq!(no_proof.len(), 1);
        assert_eq!(no_proof[0].count, Some(5), "3 acme/red + 2 contoso/red");

        let proof_bytes = query
            .execute_point_lookup_count_with_proof(&drive, None, platform_version)
            .expect("prove count");
        let (_root_hash, entries) = query
            .verify_point_lookup_count_proof(&proof_bytes, platform_version)
            .expect("verify");
        let summed: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
        assert_eq!(summed, 5);
    }

    /// **Optimization is uniform across countability tiers** — pins
    /// that a plain `countable: true` index (NOT `rangeCountable`)
    /// also gets the compact value-tree-direct proof shape.
    ///
    /// This used to be the inverse pin (the legacy `Key([0])` shape
    /// is preserved for non-range_countable indexes), but the
    /// insertion side now makes the terminator value tree a
    /// `CountTree` for any countable index — not just rangeCountable
    /// ones — so the optimization activates uniformly. A regression
    /// to the old layout (`NormalTree` value trees + `[0]` descent
    /// for non-range_countable) would fail the shape assertion here
    /// AND silently break counts at runtime (`NormalTree`'s
    /// `count_value_or_default()` returns 1, not the doc count).
    ///
    /// `rangeCountable` is no longer needed for the smaller-proof
    /// win — it's now strictly an opt-in for `AggregateCountOnRange`
    /// (the property-name tree upgrade to `ProvableCountTree`).
    #[test]
    fn plain_countable_path_query_targets_value_tree_directly() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_by_category_normal_countable_contract();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        insert_gizmo(&drive, &data_contract, [1u8; 32], "tools");
        insert_gizmo(&drive, &data_contract, [2u8; 32], "tools");

        let document_type = data_contract
            .document_type_for_name("gizmo")
            .expect("gizmo");
        let category_eq = WhereClause {
            field: "category".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("tools".to_string()),
        };
        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            std::slice::from_ref(&category_eq),
            &[],
        )
        .expect("byCategory covers category=tools");
        assert!(
            index.countable.is_countable(),
            "fixture: byCategory must be countable (any tier) so the \
             value-tree-direct optimization activates"
        );
        assert!(
            !index.range_countable,
            "fixture: byCategory must NOT be `rangeCountable` so this test \
             actually exercises the plain-countable arm of the generalization"
        );
        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "gizmo".to_string(),
            index,
            where_clauses: vec![category_eq],
        };

        let path_query = query
            .point_lookup_count_path_query(platform_version)
            .expect("path query builds");
        let serialized_tools = document_type
            .serialize_value_for_key(
                "category",
                &Value::Text("tools".to_string()),
                platform_version,
            )
            .expect("serialize category");
        // Optimized shape: path ends at the property-name segment
        // (NOT at the serialized value), and the query item is
        // `Key(serialized_value)`. A regression that re-introduced
        // the `[0]` descent for plain countable would fire here.
        assert_eq!(
            path_query.path.last().expect("non-empty path"),
            &b"category".to_vec(),
            "plain `countable: true` Equal-only path must end at the \
             property-name subtree (matching the rangeCountable shape) — \
             the insertion side now stores the value tree as `CountTree` \
             regardless of `range_countable`, so the optimization applies \
             uniformly."
        );
        let items = &path_query.query.query.items;
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0],
            QueryItem::Key(serialized_tools),
            "selector must be `Key(serialize(value))` so the resolved \
             element is the terminator value-tree CountTree itself"
        );
        assert_ne!(
            items[0],
            QueryItem::Key(vec![0]),
            "`Key([0])` is the legacy descent and must NOT appear here — \
             the optimization is now active for every countable tier"
        );

        // Counts agree across no-proof and prove.
        let no_proof = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("no-proof");
        assert_eq!(no_proof[0].count, Some(2));
        let proof_bytes = query
            .execute_point_lookup_count_with_proof(&drive, None, platform_version)
            .expect("prove count");
        let (_root_hash, entries) = query
            .verify_point_lookup_count_proof(&proof_bytes, platform_version)
            .expect("verify");
        let summed: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
        assert_eq!(summed, 2);
    }
}

#[cfg(test)]
mod time_range_picker_tests {
    //! Coverage for the transform gate the count pickers apply before
    //! scoring a candidate — see
    //! [`crate::query::index_admissible_for_resolved_time_range`].
    //!
    //! A bucketed index stores one entry per bucket containing a document,
    //! keyed by bucket start. Counting over it is only meaningful when the
    //! query pins a single bucket, and the only thing that can pin one is an
    //! equality produced by `IN_TIME_RANGE` resolution. Both directions of
    //! the mismatch return a wrong count rather than an error, so the picker
    //! is where they have to be stopped.

    use super::*;
    use dpp::data_contract::document_type::{
        Index, IndexCountability, IndexProperty, TimeRangeTransform,
    };

    /// One hour as a transform declares a window (seconds) and as the clause
    /// values below are expressed (milliseconds, the unit of a bucket start).
    const HOUR_SECONDS: u64 = 3_600;
    const HOUR_MS: u64 = 3_600_000;
    const SOURCE: &str = "$createdAt";

    fn make_index(
        name: &str,
        properties: &[&str],
        time_range: Option<TimeRangeTransform>,
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
            countable: IndexCountability::Countable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_countable_at: vec![],
            ranked_summable: false,
            ranked_averageable: false,
            time_range,
            terminal: None,
            preallocated: false,
            skip_if_absent: false,
        }
    }

    /// `trending` buckets `$createdAt` into 6h windows every 2h; `byHashtag`
    /// covers the same two fields with raw timestamps and sorts first.
    fn indexes() -> std::collections::BTreeMap<String, Index> {
        [
            make_index("byHashtag", &["hashtag", SOURCE], None),
            make_index(
                "trending",
                &[SOURCE, "hashtag"],
                Some(TimeRangeTransform {
                    source: SOURCE.to_string(),
                    range_seconds: 6 * HOUR_SECONDS,
                    step_seconds: 2 * HOUR_SECONDS,
                    phase_seconds: 0,
                }),
            ),
        ]
        .into_iter()
        .map(|index| (index.name.clone(), index))
        .collect()
    }

    /// The provenance a real `IN_TIME_RANGE` resolution against the
    /// `trending` grid produces: the source field plus the exact transform.
    /// Constructed directly (not read from the candidate map) so tests that
    /// remove the trending index can still present the resolution.
    fn source_resolution() -> Vec<ResolvedTimeRange> {
        vec![ResolvedTimeRange {
            transform: TimeRangeTransform {
                source: SOURCE.to_string(),
                range_seconds: 6 * HOUR_SECONDS,
                step_seconds: 2 * HOUR_SECONDS,
                phase_seconds: 0,
            },
        }]
    }

    fn equal(field: &str, value: Value) -> WhereClause {
        WhereClause {
            field: field.to_string(),
            operator: WhereOperator::Equal,
            value,
        }
    }

    /// The resolved equality names the bucketed index's source, so that index
    /// — and only that index — may serve the count.
    #[test]
    fn resolved_source_equality_selects_the_bucketed_index() {
        let indexes = indexes();
        let where_clauses = vec![
            equal(SOURCE, Value::U64(6 * HOUR_MS)),
            equal("hashtag", Value::Text("ibiza".to_string())),
        ];
        let picked = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            &indexes,
            &where_clauses,
            &source_resolution(),
        )
        .expect("the bucketed index exactly covers the resolved clause set");
        assert_eq!(picked.name, "trending");
    }

    /// The same clause set without the provenance must not reach the bucketed
    /// index. `byHashtag` covers it and sorts first, so this also pins that
    /// the gate does not merely reorder candidates.
    #[test]
    fn raw_equality_on_the_source_never_selects_the_bucketed_index() {
        let indexes = indexes();
        let where_clauses = vec![
            equal(SOURCE, Value::U64(1_700_000_000_000)),
            equal("hashtag", Value::Text("ibiza".to_string())),
        ];
        let picked = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            &indexes,
            &where_clauses,
            &[],
        )
        .expect("the plain index covers a raw equality on both fields");
        assert_eq!(picked.name, "byHashtag");
    }

    /// An `In` over the source enumerates bucket starts, and a document
    /// appears under every bucket containing it, so any hit would be counted
    /// once per overlapping bucket. Resolution never produces an `In`, so the
    /// only way this shape reaches the picker is a client writing it by hand.
    #[test]
    fn raw_in_on_the_source_does_not_select_the_bucketed_index() {
        let mut indexes = indexes();
        // Drop the plain index so a `None` here can only mean the bucketed
        // one was refused, not that a raw index happened to win.
        indexes.remove("byHashtag");
        let where_clauses = vec![
            WhereClause {
                field: SOURCE.to_string(),
                operator: WhereOperator::In,
                value: Value::Array(vec![
                    Value::U64(2 * HOUR_MS),
                    Value::U64(4 * HOUR_MS),
                    Value::U64(6 * HOUR_MS),
                ]),
            },
            equal("hashtag", Value::Text("ibiza".to_string())),
        ];
        assert!(
            DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                &indexes,
                &where_clauses,
                &[],
            )
            .is_none()
        );
    }

    /// The converse gate: with a resolved field named, an index that stores
    /// raw timestamps is not a candidate even when it exactly covers the
    /// clause fields — matching a bucket start against raw values would
    /// return a proven-empty result.
    #[test]
    fn plain_index_is_not_selected_when_a_time_range_field_was_resolved() {
        let mut indexes = indexes();
        indexes.remove("trending");
        let where_clauses = vec![
            equal(SOURCE, Value::U64(6 * HOUR_MS)),
            equal("hashtag", Value::Text("ibiza".to_string())),
        ];
        assert!(
            DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                &indexes,
                &where_clauses,
                &source_resolution(),
            )
            .is_none()
        );
    }

    /// Provenance names its field through the transform itself
    /// ([`ResolvedTimeRange::field`] is derived from `transform.source`), so
    /// the fabricated field/transform mismatch this test used to construct is
    /// unrepresentable. What remains fabricatable is a resolution whose grid
    /// no index declares — it must admit nothing.
    #[test]
    fn provenance_with_a_grid_no_index_declares_admits_nothing() {
        let indexes = indexes();
        let where_clauses = vec![
            equal(SOURCE, Value::U64(6 * HOUR_MS)),
            equal("hashtag", Value::Text("ibiza".to_string())),
        ];
        let mut mismatched = source_resolution();
        mismatched[0].transform.step_seconds /= 2;
        assert!(
            DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                &indexes,
                &where_clauses,
                &mismatched,
            )
            .is_none(),
            "a resolution carrying a grid no index declares must not admit \
             the bucketed index (nor, being a resolution, the plain one)"
        );
    }
}

mod prefix_to_last {
    //! The prefix-to-last count form: `count WHERE hashtag == X` on a
    //! `rangeCountable` `[hashtag, postId]` index, with NO clause on the
    //! last property. The count is the terminal property-name tree's own
    //! element aggregate — the whole-prefix total — read (and proven) as
    //! one element at `…/hashtag/X/postId`. Runs against the `tally`
    //! fixture (the yappr-likes shape minus the ranked axis): grovedb's
    //! query dispatch refuses to return INDEXED tree elements, so a
    //! rankedCountable terminal stays unservable by this form — pinned
    //! below — while a prefix-level ranking (`rankedCountable: { at }`)
    //! keeps its terminal non-indexed and composes.

    use super::*;
    use crate::config::DriveConfig;
    use crate::query::drive_document_count_query::drive_dispatcher::{
        DocumentCountRequest, DocumentCountResponse,
    };
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::DocumentV0Setters;
    use dpp::tests::json_document::json_document_to_contract;

    fn setup_tally() -> (Drive, dpp::prelude::DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "tests/supporting_files/contract/tally/tally-contract.json",
            false,
            platform_version,
        )
        .expect("expected to parse the tally contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the tally contract");
        (drive, contract)
    }

    fn insert_like(
        drive: &Drive,
        contract: &dpp::prelude::DataContract,
        hashtag: &str,
        post: &str,
        seed: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let document_type = contract
            .document_type_for_name("like")
            .expect("like doctype exists");
        let mut doc: Document = document_type
            .random_document(Some(seed), platform_version)
            .expect("random document");
        let mut props = StdBTreeMap::new();
        props.insert("hashtag".to_string(), Value::Text(hashtag.to_string()));
        props.insert("postId".to_string(), Value::Text(post.to_string()));
        doc.set_properties(props);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert a like");
    }

    /// Three likes for `alpha` (across two posts) and one for `beta` —
    /// the totals the tests below assert.
    fn seed_likes(drive: &Drive, contract: &dpp::prelude::DataContract) {
        insert_like(drive, contract, "alpha", "p1", 1);
        insert_like(drive, contract, "alpha", "p1", 2);
        insert_like(drive, contract, "alpha", "p2", 3);
        insert_like(drive, contract, "beta", "p3", 4);
    }

    fn hashtag_equal(hashtag: &str) -> Vec<WhereClause> {
        vec![WhereClause {
            field: "hashtag".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text(hashtag.to_string()),
        }]
    }

    fn like_query<'a>(
        contract: &'a dpp::prelude::DataContract,
        where_clauses: Vec<WhereClause>,
    ) -> DriveDocumentCountQuery<'a> {
        let document_type = contract
            .document_type_for_name("like")
            .expect("like doctype exists");
        // Off the contract's document-type map rather than the local
        // `DocumentTypeRef`, so the borrow lives as long as `contract`.
        let indexes = contract
            .document_types()
            .get("like")
            .expect("like doctype exists")
            .indexes();
        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            indexes,
            &where_clauses,
            &[],
        )
        .expect("the prefix-to-last fallback must pick byHashtagPost");
        assert_eq!(
            index.properties.len(),
            2,
            "the picked index must be the compound byHashtagPost"
        );
        DriveDocumentCountQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "like".to_string(),
            index,
            where_clauses,
        }
    }

    /// The picker's coverage rules for the new form, over real contract
    /// index sets: the fallback fires only for an unranked
    /// `rangeCountable` index with exactly the leading properties
    /// covered; a ranked terminal (an indexed tree grovedb refuses to
    /// return) stays rejected, as does a countable-but-not-rangeCountable
    /// compound; and an exact match keeps winning where one exists.
    #[test]
    fn picker_accepts_leading_coverage_on_unranked_range_countable_only() {
        let platform_version = PlatformVersion::latest();

        // The tally fixture: {hashtag} covers byHashtagPost's leading
        // property → the fallback fires.
        let (_drive, tally) = setup_tally();
        let like_type_indexes = tally
            .document_types()
            .get("like")
            .expect("like doctype exists")
            .indexes();
        let picked = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            like_type_indexes,
            &hashtag_equal("alpha"),
            &[],
        )
        .expect("leading coverage of an unranked rangeCountable index must match");
        assert_eq!(picked.properties.len(), 2);
        assert_eq!(picked.properties[0].name, "hashtag");

        // yappr-likes: the same shape but rankedCountable — the terminal
        // property-name tree is a ProvableCountIndexedTree, which
        // grovedb's query dispatch refuses to return, so the fallback
        // must not select it.
        let yappr = json_document_to_contract(
            "tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json",
            false,
            platform_version,
        )
        .expect("yappr fixture parses");
        let yappr_like_indexes = yappr
            .document_types()
            .get("like")
            .expect("like doctype exists")
            .indexes();
        assert!(
            DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                yappr_like_indexes,
                &hashtag_equal("alpha"),
                &[],
            )
            .is_none(),
            "a ranked terminal must stay rejected — its element is an indexed tree"
        );

        // family's [firstName, middleName, lastName] is countable but NOT
        // rangeCountable: leading coverage must stay rejected.
        let family = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("family fixture parses");
        let person_indexes = family
            .document_types()
            .get("person")
            .expect("person doctype exists")
            .indexes();
        assert!(
            DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                person_indexes,
                &vec![
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
                ],
                &[],
            )
            .is_none(),
            "leading coverage without rangeCountable must stay rejected"
        );
    }

    /// The motivating query, end to end: the whole-prefix total is one
    /// element read on the no-proof path and one verified element on the
    /// prove path, reconstructing the live root hash.
    #[test]
    fn equal_prefix_count_reads_and_proves_the_whole_prefix_total() {
        let (drive, contract) = setup_tally();
        let platform_version = PlatformVersion::latest();
        seed_likes(&drive, &contract);

        let query = like_query(&contract, hashtag_equal("alpha"));

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("the prefix count must read");
        assert_eq!(
            results[0].count,
            Some(3),
            "alpha's total must count across its posts"
        );

        let proof = query
            .execute_point_lookup_count_with_proof(&drive, None, platform_version)
            .expect("the prefix count must prove");
        let (root_hash, entries) = query
            .verify_point_lookup_count_proof(&proof, platform_version)
            .expect("the proof must verify");
        let summed: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
        assert_eq!(summed, 3, "the verified total must match the read");
        assert_eq!(
            root_hash,
            drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("root hash must be readable"),
            "the proof must reconstruct the live grovedb root hash"
        );
    }

    /// An absent prefix counts zero — with a verifiable absence proof.
    #[test]
    fn absent_prefix_counts_zero_with_proof() {
        let (drive, contract) = setup_tally();
        let platform_version = PlatformVersion::latest();
        seed_likes(&drive, &contract);

        let query = like_query(&contract, hashtag_equal("missing"));

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("an absent prefix must read");
        assert_eq!(results[0].count, Some(0));

        let proof = query
            .execute_point_lookup_count_with_proof(&drive, None, platform_version)
            .expect("an absent prefix must prove");
        let (_root_hash, entries) = query
            .verify_point_lookup_count_proof(&proof, platform_version)
            .expect("the absence proof must verify");
        let summed: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
        assert_eq!(summed, 0, "the verified absence must count zero");
    }

    /// `hashtag IN […]` fans the same element read over the branches:
    /// the no-proof read sums them; the proof verifies per branch.
    #[test]
    fn in_prefix_counts_per_branch() {
        let (drive, contract) = setup_tally();
        let platform_version = PlatformVersion::latest();
        seed_likes(&drive, &contract);

        let query = like_query(
            &contract,
            vec![WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(vec![
                    Value::Text("alpha".to_string()),
                    Value::Text("beta".to_string()),
                ]),
            }],
        );

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("the branched prefix count must read");
        assert_eq!(results[0].count, Some(4), "alpha's 3 plus beta's 1");

        let proof = query
            .execute_point_lookup_count_with_proof(&drive, None, platform_version)
            .expect("the branched prefix count must prove");
        let (_root_hash, entries) = query
            .verify_point_lookup_count_proof(&proof, platform_version)
            .expect("the branched proof must verify");
        let summed: u64 = entries.iter().map(|e| e.count.unwrap_or(0)).sum();
        assert_eq!(summed, 4, "verified per-branch counts must sum to the read");
    }

    /// The public dispatcher serves the form end to end — the shape a
    /// DAPI `GetDocumentsCount` request actually takes.
    #[test]
    fn dispatcher_serves_the_prefix_form() {
        let (drive, contract) = setup_tally();
        let platform_version = PlatformVersion::latest();
        seed_likes(&drive, &contract);

        let drive_config = DriveConfig::default();
        let response = drive
            .execute_document_count_request(
                DocumentCountRequest {
                    contract: &contract,
                    document_type: contract
                        .document_type_for_name("like")
                        .expect("like doctype exists"),
                    where_clauses: hashtag_equal("alpha"),
                    resolved_time_ranges: vec![],
                    order_clauses: Vec::new(),
                    mode: CountMode::Aggregate,
                    limit: None,
                    prove: false,
                    drive_config: &drive_config,
                },
                None,
                platform_version,
            )
            .expect("the dispatcher must serve the prefix form");
        let DocumentCountResponse::Aggregate(total) = response else {
            panic!("expected an aggregate response, got a different variant");
        };
        assert_eq!(
            total, 3,
            "the aggregate total must be alpha's whole-prefix count"
        );
    }
}
