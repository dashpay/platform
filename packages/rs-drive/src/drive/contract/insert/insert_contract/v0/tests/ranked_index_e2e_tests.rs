//! End-to-end coverage for **ranked aggregate indexes** (meta schema v3 /
//! PV14, grovedb PR 657 indexed trees).
//!
//! Everything here runs against the `restaurants` fixture at
//! `tests/supporting_files/contract/restaurants/restaurants-contract.json`,
//! which exercises all three indexed element variants — one per document type,
//! because two indexes over the same property set on one doctype is a
//! `DuplicateIndexError`:
//!
//! | doctype  | index                | declares                                   | terminal property-name tree             |
//! |----------|----------------------|--------------------------------------------|-----------------------------------------|
//! | `review` | `byRestaurant`       | `averageable` + `rangeAverageable` + `rankedAverageable` | `ProvableCountProvableSumIndexedTree` axes `[Avg]` |
//! | `visit`  | `byRestaurantVisits` | `countable` + `rangeCountable` + `rankedCountable`        | `ProvableCountIndexedTree`              |
//! | `tip`    | `byRestaurantTips`   | `summable` + `rangeSummable` + `rankedSummable`           | `ProvableSumIndexedTree`                |
//! | `adjustment` | `byRestaurantAdjustments` | same as `review`                          | `ProvableCountProvableSumIndexedTree` axes `[Avg]` |
//!
//! `adjustment` duplicates `review`'s shape for one reason: its aggregated
//! property `delta` admits negative values, which `grade` (minimum 0) does
//! not, so it is the only doctype that can exercise signed sums and the
//! floor-toward-negative-infinity rounding of the Avg sort key.
//!
//! Two layers are checked. First the *shape*: contract registration must lay
//! down the right indexed element, with the right axes TLV. Then the
//! *behaviour*: documents inserted through the ordinary document-insert path
//! (real grovedb batches, real secondary maintenance) must be rankable through
//! grovedb's `indexed_{avg,count,sum}_top_k` reads, and stay correct across
//! updates and deletes.
//!
//! ## Grove path from the contract root to an indexed property-name tree
//!
//! Every ranked read (and, in the query phase, every ranked proof) is issued
//! against the path of the **terminal property-name tree**. For a
//! single-property index that is:
//!
//! ```text
//! [ RootTree::DataContractDocuments as u8 ]   // 0x01
//!   / <contract_id: 32 bytes>
//!   / [ 0x01 ]                                // "documents", not "contract"
//!   / <document_type_name: utf-8>             // e.g. b"review"
//!   / <last_index_property_name: utf-8>       // e.g. b"restaurantId"
//! ```
//!
//! and the children of that tree are the *groups*: one value tree per distinct
//! value of the last index property, keyed by the raw index-key bytes of that
//! value (for a `string` property, its UTF-8 bytes — e.g. `b"alpha"`). The
//! secondary entries `indexed_*_top_k` returns are keyed by those same group
//! keys. A compound index `[a, b]` inserts `<a> / <value_of_a>` between the
//! doctype and the terminal `<b>` level.
//!
//! [`batched_group_drain`] extends this suite to the one shape it does not
//! otherwise reach: several document operations applied in a *single* grovedb
//! batch. It reuses the fixture and the assertion helpers below, which is why
//! it is a child module rather than a sibling.

/// Declared with `#[path]` so it can sit beside the other test files while
/// still reaching this module's fixture and assertion helpers.
#[path = "batched_group_drain.rs"]
mod batched_group_drain;

use crate::drive::Drive;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::Value;
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use grovedb::element::indexed::AVG_FIXED_POINT_SCALE;
use grovedb::Element;

/// The one index property every doctype in the fixture ranks by.
const GROUP_PROPERTY: &str = "restaurantId";

fn platform_version() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

/// Load and apply the restaurants fixture.
fn setup_restaurants() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/restaurants/restaurants-contract.json",
        false,
        pv,
    )
    .expect("expected to parse the restaurants contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the restaurants contract");
    (drive, contract)
}

/// See the module docs: the path of the terminal property-name tree, which is
/// what every ranked read is issued against.
fn indexed_property_name_tree_path(
    contract: &DataContract,
    document_type_name: &str,
) -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        document_type_name.as_bytes().to_vec(),
        GROUP_PROPERTY.as_bytes().to_vec(),
    ]
}

/// `grove_get_raw` surfaces a missing key as `Err(PathKeyNotFound)`, so the
/// optional form is what the "group must be gone" assertions need.
fn read_grove_element(drive: &Drive, path: &[Vec<u8>], key: &[u8]) -> Option<Element> {
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    drive
        .grove_get_raw_optional(
            path_refs.as_slice().into(),
            key,
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &platform_version().drive,
        )
        .expect("grove_get_raw_optional should succeed")
}

/// Fetch the indexed property-name tree element itself (i.e. the element
/// living in the doctype tree under the property's key).
fn read_indexed_property_name_element(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
) -> Element {
    let path = indexed_property_name_tree_path(contract, document_type_name);
    let parent: Vec<Vec<u8>> = path[..path.len() - 1].to_vec();
    let key = path.last().expect("path is non-empty").clone();
    read_grove_element(drive, &parent, &key)
        .unwrap_or_else(|| panic!("indexed property-name tree for {document_type_name} must exist"))
}

/// grovedb's integrity sweep, including the per-axis primary↔secondary
/// content-consistency walk for indexed trees. Run at the end of every
/// behavioural test so a write path that keeps the reads *looking* right while
/// corrupting the secondaries still fails.
fn assert_grovedb_is_consistent(drive: &Drive) {
    let issues = drive
        .grove
        .verify_grovedb(None, true, false, &platform_version().drive.grove_version)
        .expect("verify_grovedb must run");
    assert!(
        issues.is_empty(),
        "grovedb integrity verification reported issues: {issues:?}"
    );
}

/// Build a document of `document_type_name` whose properties are exactly
/// `restaurantId` plus the one aggregated integer property.
fn build_doc(
    contract: &DataContract,
    document_type_name: &str,
    aggregated_property: &str,
    restaurant: &str,
    aggregated_value: i64,
    seed: u64,
) -> Document {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(document_type_name)
        .unwrap_or_else(|_| panic!("{document_type_name} doctype exists"));
    let mut doc = document_type
        .random_document(Some(seed), pv)
        .expect("random document");
    let mut props = std::collections::BTreeMap::new();
    props.insert(
        GROUP_PROPERTY.to_string(),
        Value::Text(restaurant.to_string()),
    );
    props.insert(
        aggregated_property.to_string(),
        Value::I64(aggregated_value),
    );
    doc.set_properties(props);
    doc
}

fn insert_doc(drive: &Drive, contract: &DataContract, document_type_name: &str, doc: &Document) {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(document_type_name)
        .unwrap_or_else(|_| panic!("{document_type_name} doctype exists"));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((doc, None)),
                    owner_id: None,
                },
                contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .unwrap_or_else(|e| panic!("expected to insert {document_type_name} document: {e}"));
}

/// `(restaurant, value)` pairs inserted as `document_type_name` documents,
/// returned in insertion order so tests can update / delete individual ones.
fn insert_docs(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    aggregated_property: &str,
    rows: &[(&str, i64)],
) -> Vec<Document> {
    rows.iter()
        .enumerate()
        .map(|(i, (restaurant, value))| {
            let doc = build_doc(
                contract,
                document_type_name,
                aggregated_property,
                restaurant,
                *value,
                i as u64 + 1,
            );
            insert_doc(drive, contract, document_type_name, &doc);
            doc
        })
        .collect()
}

/// The fixed-point average grovedb's Avg axis orders by:
/// `floor(sum * AVG_FIXED_POINT_SCALE / count)` with euclidean (toward
/// -inf) division. The scale is grovedb's constant — 10^19 as of the
/// merged PR #657 — and is deliberately not spelled as a literal here.
///
/// Mirrors `grovedb::element::indexed::compute_avg_fixed_point` operation
/// for operation. `div_euclid`, **not** Rust's `/`: the two agree on
/// non-negative sums and disagree by one unit on every negative sum that
/// does not divide evenly, and it is grovedb's choice that decides what is
/// on disk. `saturating_mul` matches for the same reason.
fn expected_avg_fixed_point(sum: i64, count: u64) -> i128 {
    if count == 0 {
        return 0;
    }
    (sum as i128)
        .saturating_mul(AVG_FIXED_POINT_SCALE)
        .div_euclid(count as i128)
}

fn group_keys<T>(entries: &[(T, Vec<u8>)]) -> Vec<String> {
    entries
        .iter()
        .map(|(_, key)| String::from_utf8(key.clone()).expect("group keys are utf-8 in this test"))
        .collect()
}

fn avg_top_k(drive: &Drive, path: &[Vec<u8>], k: u16, descending: bool) -> Vec<(i128, Vec<u8>)> {
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    drive
        .grove
        .indexed_avg_top_k(
            path_refs.as_slice(),
            k,
            descending,
            None,
            &platform_version().drive.grove_version,
        )
        .unwrap()
        .expect("indexed_avg_top_k must succeed")
}

fn count_top_k(drive: &Drive, path: &[Vec<u8>], k: u16, descending: bool) -> Vec<(u64, Vec<u8>)> {
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    drive
        .grove
        .indexed_count_top_k(
            path_refs.as_slice(),
            k,
            descending,
            None,
            &platform_version().drive.grove_version,
        )
        .unwrap()
        .expect("indexed_count_top_k must succeed")
}

fn sum_top_k(drive: &Drive, path: &[Vec<u8>], k: u16, descending: bool) -> Vec<(i64, Vec<u8>)> {
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    drive
        .grove
        .indexed_sum_top_k(
            path_refs.as_slice(),
            k,
            descending,
            None,
            &platform_version().drive.grove_version,
        )
        .unwrap()
        .expect("indexed_sum_top_k must succeed")
}

// ---------------------------------------------------------------------------
// Shape: contract registration lays down the declared indexed element
// ---------------------------------------------------------------------------

/// The `rankedAverageable` index's terminal property-name tree must be a
/// `ProvableCountProvableSumIndexedTree` carrying exactly the Avg axis. The
/// element is PCPS-shaped (not PCIT/PSIT) because `rangeAverageable` puts both
/// range axes in effect — the single-axis variants cannot mirror a
/// count+sum primary.
#[test]
fn ranked_averageable_index_creates_a_pcpsit_with_only_the_avg_axis() {
    let (drive, contract) = setup_restaurants();

    match read_indexed_property_name_element(&drive, &contract, "review") {
        Element::ProvableCountProvableSumIndexedTree(root_key, count, sum, axes, _) => {
            assert!(root_key.is_none(), "a fresh primary has no root key");
            assert_eq!(count, 0, "a fresh primary counts nothing");
            assert_eq!(sum, 0, "a fresh primary sums nothing");
            // Avg's on-disk tag is 2; the secondary starts empty.
            assert_eq!(
                axes,
                vec![(2u8, None)],
                "rankedAverageable alone must declare exactly the Avg axis — the ranking axes \
                 are independent opt-ins, so Count and Sum secondaries must NOT be created"
            );
        }
        other => panic!("expected ProvableCountProvableSumIndexedTree, got {other:?}"),
    }
}

/// `rankedCountable` on a count-only range layout gets the dedicated
/// single-axis PCIT element, which carries no axes TLV at all.
#[test]
fn ranked_countable_only_index_creates_a_pcit() {
    let (drive, contract) = setup_restaurants();

    match read_indexed_property_name_element(&drive, &contract, "visit") {
        Element::ProvableCountIndexedTree(primary_root_key, secondary_root_key, count, _) => {
            assert!(primary_root_key.is_none());
            assert!(secondary_root_key.is_none());
            assert_eq!(count, 0);
        }
        other => panic!("expected ProvableCountIndexedTree, got {other:?}"),
    }
}

/// `rankedSummable` on a sum-only range layout gets the dedicated single-axis
/// PSIT element.
#[test]
fn ranked_summable_only_index_creates_a_psit() {
    let (drive, contract) = setup_restaurants();

    match read_indexed_property_name_element(&drive, &contract, "tip") {
        Element::ProvableSumIndexedTree(primary_root_key, secondary_root_key, sum, _) => {
            assert!(primary_root_key.is_none());
            assert!(secondary_root_key.is_none());
            assert_eq!(sum, 0);
        }
        other => panic!("expected ProvableSumIndexedTree, got {other:?}"),
    }
}

/// The children of an indexed primary are the ordinary value trees its
/// non-indexed mirror would have — the indexed variant changes the parent, not
/// the group layout. Pinned because the write path deliberately does NOT
/// special-case value trees under an indexed parent.
#[test]
fn groups_under_an_indexed_primary_keep_their_mirror_value_tree_types() {
    let (drive, contract) = setup_restaurants();

    insert_docs(&drive, &contract, "review", "grade", &[("alpha", 50)]);
    insert_docs(&drive, &contract, "visit", "guests", &[("alpha", 2)]);
    insert_docs(&drive, &contract, "tip", "amount", &[("alpha", 7)]);

    // review: countable + summable + both range axes → PCPS value tree.
    let review_path = indexed_property_name_tree_path(&contract, "review");
    match read_grove_element(&drive, &review_path, b"alpha").expect("group must exist") {
        Element::ProvableCountProvableSumTree(_, count, sum, _) => {
            assert_eq!(count, 1);
            assert_eq!(sum, 50);
        }
        other => panic!("expected a PCPS group under the review PCPSIT, got {other:?}"),
    }

    // visit: countable + rangeCountable, no sum surface → plain CountTree.
    let visit_path = indexed_property_name_tree_path(&contract, "visit");
    match read_grove_element(&drive, &visit_path, b"alpha").expect("group must exist") {
        Element::CountTree(_, count, _) => assert_eq!(count, 1),
        other => panic!("expected a CountTree group under the visit PCIT, got {other:?}"),
    }

    // tip: summable + rangeSummable, no count surface → plain SumTree.
    let tip_path = indexed_property_name_tree_path(&contract, "tip");
    match read_grove_element(&drive, &tip_path, b"alpha").expect("group must exist") {
        Element::SumTree(_, sum, _) => assert_eq!(sum, 7),
        other => panic!("expected a SumTree group under the tip PSIT, got {other:?}"),
    }

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Behaviour: ranking across inserts / updates / deletes
// ---------------------------------------------------------------------------

/// Documents inserted through the ordinary document-insert path must leave the
/// Avg secondary ordered by each restaurant's average grade.
///
/// Averages: alpha (90+80)/2 = 85, beta (60+70+50)/3 = 60, gamma 95/1 = 95.
#[test]
fn avg_axis_ranks_groups_by_average_grade() {
    let (drive, contract) = setup_restaurants();

    insert_docs(
        &drive,
        &contract,
        "review",
        "grade",
        &[
            ("alpha", 90),
            ("alpha", 80),
            ("beta", 60),
            ("beta", 70),
            ("beta", 50),
            ("gamma", 95),
        ],
    );

    let path = indexed_property_name_tree_path(&contract, "review");

    let descending = avg_top_k(&drive, &path, 10, true);
    assert_eq!(
        group_keys(&descending),
        vec!["gamma", "alpha", "beta"],
        "descending Avg ranking must be gamma(95) > alpha(85) > beta(60)"
    );
    assert_eq!(descending[0].0, expected_avg_fixed_point(95, 1));
    assert_eq!(descending[1].0, expected_avg_fixed_point(170, 2));
    assert_eq!(descending[2].0, expected_avg_fixed_point(180, 3));

    let ascending = avg_top_k(&drive, &path, 10, false);
    assert_eq!(group_keys(&ascending), vec!["beta", "alpha", "gamma"]);

    // `k` truncates the ranking rather than the scan — top-2 is the prefix.
    let top_two = avg_top_k(&drive, &path, 2, true);
    assert_eq!(group_keys(&top_two), vec!["gamma", "alpha"]);

    assert_grovedb_is_consistent(&drive);
}

/// Averages can be negative — `summable` properties are signed — and the two
/// things that could quietly go wrong there are the fixed-point rounding and
/// the sign-aware sort-key encoding. Both are exercised here on the
/// `adjustment` doctype, whose `delta` is the one fixture property that admits
/// negative values.
///
/// Sums: alpha (-3-4-4) = -11 over 3, beta (3+4+4) = 11 over 3, gamma 0 over
/// 1. Neither ±11/3 divides evenly, so the rounding mode is observable:
/// grovedb floors toward -inf, which for the negative group is one unit below
/// what truncating division would give.
#[test]
fn avg_axis_ranks_negative_averages_and_floors_toward_negative_infinity() {
    let (drive, contract) = setup_restaurants();

    insert_docs(
        &drive,
        &contract,
        "adjustment",
        "delta",
        &[
            ("alpha", -3),
            ("alpha", -4),
            ("alpha", -4),
            ("beta", 3),
            ("beta", 4),
            ("beta", 4),
            ("gamma", 0),
        ],
    );

    let path = indexed_property_name_tree_path(&contract, "adjustment");

    let descending = avg_top_k(&drive, &path, 10, true);
    assert_eq!(
        group_keys(&descending),
        vec!["beta", "gamma", "alpha"],
        "the sort key must order signed averages: beta(+3.67) > gamma(0) > alpha(-3.67)"
    );
    assert_eq!(descending[0].0, expected_avg_fixed_point(11, 3));
    assert_eq!(descending[1].0, expected_avg_fixed_point(0, 1));
    assert_eq!(descending[2].0, expected_avg_fixed_point(-11, 3));

    // The rounding mode itself, stated rather than implied: on this group
    // euclidean division lands one unit *below* truncating division, so a
    // helper written with `/` would have agreed with the positive group and
    // silently disagreed with the negative one.
    let truncating = (-11i128 * AVG_FIXED_POINT_SCALE) / 3;
    assert_eq!(
        expected_avg_fixed_point(-11, 3),
        truncating - 1,
        "grovedb floors toward -inf; truncation would round the wrong way"
    );

    assert_grovedb_is_consistent(&drive);
}

/// Updating a document's aggregated property has to move its group in the
/// ranking. This is the delete-then-reinsert secondary transition, driven
/// through the real `update_document_for_contract` path.
#[test]
fn updating_a_grade_reorders_the_avg_ranking() {
    let (drive, contract) = setup_restaurants();
    let pv = platform_version();

    let docs = insert_docs(
        &drive,
        &contract,
        "review",
        "grade",
        &[("alpha", 90), ("beta", 60), ("gamma", 30)],
    );

    let path = indexed_property_name_tree_path(&contract, "review");
    assert_eq!(
        group_keys(&avg_top_k(&drive, &path, 10, true)),
        vec!["alpha", "beta", "gamma"],
        "baseline ranking before the update"
    );

    // Push gamma from 30 to 100, which must make it the new leader.
    let document_type = contract
        .document_type_for_name("review")
        .expect("review doctype exists");
    let mut updated = docs[2].clone();
    let mut props = updated.properties().clone();
    props.insert("grade".to_string(), Value::I64(100));
    updated.set_properties(props);
    updated.set_revision(Some(2));

    drive
        .update_document_for_contract(
            &updated,
            &contract,
            document_type,
            Some(updated.owner_id().to_buffer()),
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
            None,
        )
        .expect("expected to update the review document");

    assert_eq!(
        group_keys(&avg_top_k(&drive, &path, 10, true)),
        vec!["gamma", "alpha", "beta"],
        "gamma's average moved 30 → 100 and must lead the ranking"
    );
    assert_eq!(
        avg_top_k(&drive, &path, 1, true)[0].0,
        expected_avg_fixed_point(100, 1)
    );

    assert_grovedb_is_consistent(&drive);
}

/// Deleting every document of a group must remove that group from the axis
/// entirely — not leave a stale zero-valued secondary entry behind.
#[test]
fn draining_a_group_removes_it_from_the_avg_axis() {
    let (drive, contract) = setup_restaurants();
    let pv = platform_version();

    let docs = insert_docs(
        &drive,
        &contract,
        "review",
        "grade",
        &[("alpha", 90), ("beta", 60), ("beta", 40)],
    );

    let path = indexed_property_name_tree_path(&contract, "review");
    assert_eq!(
        group_keys(&avg_top_k(&drive, &path, 10, true)),
        vec!["alpha", "beta"]
    );

    // Delete one of beta's two reviews: the group survives, its average moves.
    drive
        .delete_document_for_contract(
            docs[2].id(),
            &contract,
            "review",
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete the second beta review");

    let after_one = avg_top_k(&drive, &path, 10, true);
    assert_eq!(group_keys(&after_one), vec!["alpha", "beta"]);
    assert_eq!(
        after_one[1].0,
        expected_avg_fixed_point(60, 1),
        "beta's average must be recomputed from the surviving review alone"
    );

    // Delete beta's last review: the group drains and must disappear.
    drive
        .delete_document_for_contract(
            docs[1].id(),
            &contract,
            "review",
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete the first beta review");

    assert_eq!(
        group_keys(&avg_top_k(&drive, &path, 10, true)),
        vec!["alpha"],
        "a drained group must leave the axis, not linger with a zero average"
    );
    assert!(
        read_grove_element(&drive, &path, b"beta").is_none(),
        "the drained group's value tree must be gone from the primary too"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The PCIT arm: a count-only ranked index ranks its groups by document count.
#[test]
fn count_axis_ranks_groups_by_document_count() {
    let (drive, contract) = setup_restaurants();

    insert_docs(
        &drive,
        &contract,
        "visit",
        "guests",
        &[
            ("alpha", 2),
            ("beta", 4),
            ("beta", 2),
            ("beta", 6),
            ("gamma", 3),
            ("gamma", 1),
        ],
    );

    let path = indexed_property_name_tree_path(&contract, "visit");

    let descending = count_top_k(&drive, &path, 10, true);
    assert_eq!(
        group_keys(&descending),
        vec!["beta", "gamma", "alpha"],
        "descending Count ranking must be beta(3) > gamma(2) > alpha(1)"
    );
    assert_eq!(
        descending.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        vec![3, 2, 1]
    );

    assert_eq!(
        group_keys(&count_top_k(&drive, &path, 1, false)),
        vec!["alpha"],
        "ascending top-1 is the smallest group"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Compound ranked indexes — the terminal level is created lazily by the
// document index walker, not at contract registration
// ---------------------------------------------------------------------------

/// Protocol version whose `document_type_schema` is 3, i.e. the first one whose
/// index grammar knows the `ranked*` keywords.
const PROTOCOL_VERSION_V14: u32 = 14;

/// A `dish` doctype with a compound `[restaurantId, course]` ranked index.
/// `standalone_prefix_index` additionally declares a countable index
/// terminating at `[restaurantId]`, which turns the prefix's value trees into
/// aggregating `CountTree`s — the shape grovedb cannot host an indexed tree
/// inside.
fn try_build_dish_contract(
    standalone_prefix_index: bool,
) -> Result<DataContract, dpp::ProtocolError> {
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::platform_value;
    use dpp::tests::utils::generate_random_identifier_struct;

    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V14).expect("expected to create factory");

    let mut indices = vec![platform_value!({
        "name": "byRestaurantCourse",
        "properties": [{"restaurantId": "asc"}, {"course": "asc"}],
        "countable": "countable",
        "summable": "price",
        "averageable": "price",
        "rangeCountable": true,
        "rangeSummable": true,
        "rangeAverageable": true,
        "rankedAverageable": true,
    })];
    if standalone_prefix_index {
        indices.push(platform_value!({
            "name": "byRestaurant",
            "properties": [{"restaurantId": "asc"}],
            "countable": "countable",
        }));
    }

    let document_schema = platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "properties": {
            "restaurantId": {"type": "string", "position": 0, "maxLength": 32},
            "course": {"type": "string", "position": 1, "maxLength": 32},
            "price": {"type": "integer", "minimum": 0, "maximum": 100000, "position": 2},
        },
        "required": ["restaurantId", "course", "price"],
        "indices": Value::Array(indices),
        "additionalProperties": false,
    });

    let schemas = platform_value!({ "dish": document_schema });

    factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .map(|created| created.data_contract_owned())
}

/// The write path's terminal-level resolver has to pick the indexed variant
/// for a **compound** ranked index too — that level lives one step below the
/// doctype tree and is materialized lazily, per prefix value, by the
/// document index walker rather than at contract registration. This pins
/// the resolver's half of that contract; the end-to-end half (documents
/// inserted, per-prefix secondaries read and proved) lives in the query
/// suites' `pinned_prefix` modules.
#[test]
fn compound_ranked_index_resolves_its_terminal_level_to_an_indexed_tree() {
    use crate::drive::document::ranked_index_tree_type::property_name_tree_type_and_ranked_axes;
    use dpp::data_contract::document_type::{Index, IndexCountability, IndexLevel, IndexProperty};
    use grovedb::element::IndexAxis;
    use grovedb::TreeType;

    let compound_ranked_index = Index {
        name: "byRestaurantCourse".to_string(),
        properties: vec![
            IndexProperty {
                name: GROUP_PROPERTY.to_string(),
                ascending: true,
            },
            IndexProperty {
                name: "course".to_string(),
                ascending: true,
            },
        ],
        unique: false,
        null_searchable: true,
        contested_index: None,
        countable: IndexCountability::Countable,
        range_countable: true,
        summable: Some("price".to_string()),
        range_summable: true,
        ranked_countable: false,
        ranked_summable: false,
        ranked_averageable: true,
    };
    let index_structure =
        IndexLevel::try_from_indices([&compound_ranked_index], "dish", platform_version())
            .expect("index level must build from a compound ranked index");

    // Prefix level `restaurantId`: no index terminates there, so it stays a
    // plain tree with no ranking axes.
    let prefix_level = index_structure
        .sub_levels()
        .get(GROUP_PROPERTY)
        .expect("prefix level exists");
    let (prefix_tree_type, prefix_axes) =
        property_name_tree_type_and_ranked_axes(prefix_level.has_index_with_type())
            .expect("prefix resolution must succeed");
    assert_eq!(prefix_tree_type, TreeType::NormalTree);
    assert!(prefix_axes.is_empty());

    // Terminal level `course`: the ranked upgrade lands here.
    let terminal_level = prefix_level
        .sub_levels()
        .get("course")
        .expect("terminal level exists");
    let (terminal_tree_type, terminal_axes) =
        property_name_tree_type_and_ranked_axes(terminal_level.has_index_with_type())
            .expect("terminal resolution must succeed");
    assert_eq!(
        terminal_tree_type,
        TreeType::ProvableCountProvableSumIndexedTree
    );
    assert_eq!(terminal_axes, vec![IndexAxis::Avg]);
}

/// A compound ranked index parses on its own (per-prefix semantics —
/// the terminal level's indexed tree is materialized lazily per prefix
/// by the document walker; grovedb's PR #657 supports creating and
/// populating an indexed tree in one batch). What stays rejected, at
/// contract-parse time, is the one structurally impossible pairing: a
/// countable/summable index terminating at the compound's full leading
/// prefix, whose aggregating value trees would demand the
/// NonCounted/NotSummed wrapper grovedb structurally rejects for
/// indexed trees.
///
/// Storage-level backstop behind that parse-time gate: both wrapper
/// dispatchers in `fees/op.rs` fail closed (`DriveError::NotSupported`)
/// for a ranked terminal level inside an aggregating value tree — the
/// frozen v0 diagonal and the v14 zero-contribution matrix alike. The
/// latter matters since the v14 shared-prefix fix: its unwrapped
/// fallback for non-sum children of sum-only parents would otherwise
/// have accepted an indexed continuation, quietly creating a ranked
/// tree the picker never resolves.
///
/// Note that a ranked index *sharing* its property with a compound index
/// — `[a]` ranked next to `[a, b]` — is a different (and, since v14,
/// fully supported) shape: there the ranked level is still the terminal
/// one, and the continuation hangs *below* it. See
/// `ranked_index_ranks_correctly_next_to_a_compound_index_sharing_its_property`.
#[test]
fn compound_ranked_index_contract_parses_unless_its_prefix_aggregates() {
    try_build_dish_contract(false)
        .expect("a compound ranked index with no aggregating prefix index must parse");

    let error = try_build_dish_contract(true).expect_err(
        "a countable index terminating at the ranked compound's prefix must be rejected",
    );
    let message = error.to_string();
    assert!(
        message.contains("byRestaurantCourse")
            && message.contains("byRestaurant")
            && message.contains("NonCounted"),
        "expected the prefix-overlap rejection naming both indexes and the structural \
         conflict, got: {message}"
    );
}

/// Build a `visit` contract whose single-property `rankedCountable` index
/// spells `nullSearchable` out as `null_searchable`, so the rejection can be
/// exercised from the contract path rather than from the index parser alone.
fn try_build_null_searchable_ranked_contract(
    null_searchable: Option<bool>,
) -> Result<DataContract, dpp::ProtocolError> {
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::platform_value;
    use dpp::tests::utils::generate_random_identifier_struct;

    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V14).expect("expected to create factory");

    let mut index_entry: Vec<(Value, Value)> = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("byRestaurantVisits".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![Value::Map(vec![(
                Value::Text(GROUP_PROPERTY.to_string()),
                Value::Text("asc".to_string()),
            )])]),
        ),
        (
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        ),
        (Value::Text("rangeCountable".to_string()), Value::Bool(true)),
        (
            Value::Text("rankedCountable".to_string()),
            Value::Bool(true),
        ),
    ];
    if let Some(null_searchable) = null_searchable {
        index_entry.push((
            Value::Text("nullSearchable".to_string()),
            Value::Bool(null_searchable),
        ));
    }

    let schemas = platform_value!({
        "visit": {
            "type": "object",
            "documentsMutable": true,
            "canBeDeleted": true,
            "properties": {
                "restaurantId": {"type": "string", "position": 0, "maxLength": 32},
                "guests": {"type": "integer", "minimum": 1, "maximum": 100, "position": 1},
            },
            "required": ["restaurantId", "guests"],
            "indices": Value::Array(vec![Value::Map(index_entry)]),
            "additionalProperties": false,
        }
    });

    factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .map(|created| created.data_contract_owned())
}

/// `nullSearchable: false` on a ranked index is refused at contract-parse
/// time (rs-dpp), and that parse gate is the *enforcement* — nothing below it
/// re-checks the combination.
///
/// What the gate prevents: with `nullSearchable: false`, a document that
/// leaves the indexed property out still gets its null group's value tree
/// created by the index walker
/// (`add_indices_for_top_index_level_for_contract_operations_v2` inserts it
/// unconditionally, before recursing), and only then does the terminal
/// handler decline to write the reference
/// (`add_reference_for_index_level_for_contract_operations_v0`'s
/// `all_fields_null && !should_insert_with_all_null` early return). Under a
/// ranked index that value tree is an entry of a grovedb indexed primary, so
/// the secondary mirror records it as a group with zero aggregates — an
/// authenticated TOP/BOTTOM answer would contain a group the index is
/// supposed to exclude, with no document behind it.
///
/// The same shape is what a ranked `[a]` sharing its prefix with `[a, b]`
/// would hit: the null group's value tree additionally hosts the compound
/// index's `b` continuation, which keeps the phantom group alive even after
/// every real document is gone.
#[test]
fn null_searchable_false_on_a_ranked_index_is_rejected_at_parse_time() {
    let error = try_build_null_searchable_ranked_contract(Some(false))
        .expect_err("nullSearchable: false on a ranked index must be rejected at parse time");
    let message = error.to_string();
    assert!(
        message.contains("nullSearchable") && message.contains("phantom"),
        "expected the phantom-group rationale, got: {message}"
    );

    // Both accepted spellings of the default.
    try_build_null_searchable_ranked_contract(None)
        .expect("a ranked index with no nullSearchable key must register");
    try_build_null_searchable_ranked_contract(Some(true))
        .expect("a ranked index with an explicit nullSearchable: true must register");
}

/// The shape the parse gate above rules out, pinned as a struct literal —
/// the parser is the only thing standing between this combination and the
/// phantom group, so it is worth stating exactly what makes it one.
///
/// Both halves have to be true at once for a phantom to exist, and they are:
///
/// * `should_insert_with_all_null: false` — the terminal handler suppresses
///   the null group's reference, so the group has no document behind it;
/// * the level still resolves to an *indexed* tree, so grovedb mirrors every
///   one of its children — including that document-less null group — into the
///   ranked secondary.
///
/// With `null_searchable: true` the first half flips: the null documents get
/// a real reference and the group becomes a legitimate rankable one. That is
/// the only suppression path into this level — the terminal handler's early
/// return is the sole place a reference is skipped, and for a single-property
/// index `all_fields_null` is exactly "this property is null" — which is why
/// the parse gate closes the hole completely rather than narrowing it.
#[test]
fn a_null_unsearchable_ranked_level_is_what_makes_a_phantom_group_possible() {
    use crate::drive::document::ranked_index_tree_type::property_name_tree_type_and_ranked_axes;
    use dpp::data_contract::document_type::{Index, IndexCountability, IndexLevel, IndexProperty};
    use grovedb::element::IndexAxis;
    use grovedb::TreeType;

    let ranked_index = |null_searchable: bool| Index {
        name: "byRestaurantVisits".to_string(),
        properties: vec![IndexProperty {
            name: GROUP_PROPERTY.to_string(),
            ascending: true,
        }],
        unique: false,
        null_searchable,
        contested_index: None,
        countable: IndexCountability::Countable,
        range_countable: true,
        summable: None,
        range_summable: false,
        ranked_countable: true,
        ranked_summable: false,
        ranked_averageable: false,
    };

    for null_searchable in [false, true] {
        let index = ranked_index(null_searchable);
        let index_structure = IndexLevel::try_from_indices([&index], "visit", platform_version())
            .expect("index level must build from a single-property ranked index");
        let terminal_level = index_structure
            .sub_levels()
            .get(GROUP_PROPERTY)
            .expect("terminal level exists");
        let type_info = terminal_level
            .has_index_with_type()
            .expect("the ranked index terminates here");

        // The level is indexed either way — that is the ranking axis doing
        // its job, and it is why every child of this level reaches the
        // secondary mirror.
        let (tree_type, axes) = property_name_tree_type_and_ranked_axes(Some(type_info))
            .expect("terminal resolution must succeed");
        assert_eq!(tree_type, TreeType::ProvableCountIndexedTree);
        assert_eq!(axes, vec![IndexAxis::Count]);

        // ...and this is the half the parse gate removes: without it the
        // null group's value tree exists with no reference inside it.
        assert_eq!(
            type_info.should_insert_with_all_null, null_searchable,
            "the terminal handler suppresses the null group's reference exactly when \
             null_searchable is false"
        );
    }
}

// ---------------------------------------------------------------------------
// Contract update
// ---------------------------------------------------------------------------

/// Build a `cafe` contract owned by `owner_id`. With `with_ranked_doctype` a
/// second doctype carrying a single-property `rankedCountable` index is added,
/// so applying the two versions in sequence exercises
/// `update_contract_v0`'s new-doctype branch.
fn build_cafe_contract(
    owner_id: dpp::identifier::Identifier,
    with_ranked_doctype: bool,
) -> DataContract {
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::platform_value;

    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V14).expect("expected to create factory");

    let mut schemas = vec![(
        Value::Text("note".to_string()),
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "canBeDeleted": true,
            "properties": {
                "body": {"type": "string", "position": 0, "maxLength": 63},
            },
            "required": ["body"],
            "additionalProperties": false,
        }),
    )];
    if with_ranked_doctype {
        schemas.push((
            Value::Text("order".to_string()),
            platform_value!({
                "type": "object",
                "documentsMutable": true,
                "canBeDeleted": true,
                "properties": {
                    "restaurantId": {"type": "string", "position": 0, "maxLength": 32},
                    "guests": {"type": "integer", "minimum": 1, "maximum": 100, "position": 1},
                },
                "required": ["restaurantId", "guests"],
                "indices": [{
                    "name": "byRestaurantOrders",
                    "properties": [{"restaurantId": "asc"}],
                    "countable": "countable",
                    "rangeCountable": true,
                    "rankedCountable": true,
                }],
                "additionalProperties": false,
            }),
        ));
    }

    factory
        .create_with_value_config(owner_id, 0, Value::Map(schemas), None, None)
        .expect("expected to create the cafe data contract")
        .data_contract_owned()
}

/// A contract update that introduces a doctype with a ranked index must
/// materialize the indexed element, exactly as a fresh registration would.
/// Without the ranked arms in `update_contract_v0` the new index would come
/// back as a plain `ProvableCountTree` — readable by range-count queries but
/// invisible to every ranked one, and diverging from what a fresh insert of the
/// same contract lays down.
#[test]
fn contract_update_adding_a_ranked_doctype_creates_the_indexed_element() {
    use dpp::data_contract::accessors::v0::DataContractV0Setters;

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let owner_id = dpp::tests::utils::generate_random_identifier_struct();

    let v1 = build_cafe_contract(owner_id, false);
    drive
        .apply_contract(
            &v1,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply cafe v1");

    let mut v2 = build_cafe_contract(owner_id, true);
    assert_eq!(v2.id(), v1.id(), "both versions must be the same contract");
    v2.set_version(2);
    drive
        .apply_contract(
            &v2,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply cafe v2");

    let doctype_path: Vec<Vec<u8>> = vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        v2.id().as_bytes().to_vec(),
        vec![1],
        b"order".to_vec(),
    ];
    match read_grove_element(&drive, &doctype_path, GROUP_PROPERTY.as_bytes())
        .expect("the new doctype's index tree must exist after the update")
    {
        Element::ProvableCountIndexedTree(_, _, count, _) => assert_eq!(count, 0),
        other => panic!("expected a ProvableCountIndexedTree after the update, got {other:?}"),
    }

    // And it works end to end: documents inserted after the update rank.
    for (restaurant, guests, seed) in [("alpha", 2, 1u64), ("beta", 4, 2), ("beta", 3, 3)] {
        let document_type = v2
            .document_type_for_name("order")
            .expect("order doctype exists");
        let mut doc = document_type
            .random_document(Some(seed), pv)
            .expect("random document");
        let mut props = std::collections::BTreeMap::new();
        props.insert(
            GROUP_PROPERTY.to_string(),
            Value::Text(restaurant.to_string()),
        );
        props.insert("guests".to_string(), Value::I64(guests));
        doc.set_properties(props);
        insert_doc(&drive, &v2, "order", &doc);
    }

    let mut ranked_path = doctype_path.clone();
    ranked_path.push(GROUP_PROPERTY.as_bytes().to_vec());
    assert_eq!(
        group_keys(&count_top_k(&drive, &ranked_path, 10, true)),
        vec!["beta", "alpha"]
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Estimated (dry-run) costs
// ---------------------------------------------------------------------------

/// Run one document insert twice — once in estimated mode (`apply: false`,
/// which populates the layer-info map and takes every stateless cost path) and
/// once applied — and return `(estimated, actual)` fees.
fn estimated_and_actual_insert_fees(
    document_type_name: &str,
    aggregated_property: &str,
) -> (
    dpp::fee::fee_result::FeeResult,
    dpp::fee::fee_result::FeeResult,
) {
    use std::borrow::Cow;

    let (drive, contract) = setup_restaurants();
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(document_type_name)
        .unwrap_or_else(|_| panic!("{document_type_name} doctype exists"));
    let doc = build_doc(
        &contract,
        document_type_name,
        aggregated_property,
        "alpha",
        42,
        7,
    );
    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

    let run = |apply: bool| {
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, storage_flags.clone())),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                apply,
                None,
                pv,
                None,
            )
            .unwrap_or_else(|e| {
                panic!("expected the {document_type_name} insert (apply={apply}) to succeed: {e}")
            })
    };

    // Estimate first, against the same empty state the applied run will see.
    let estimated = run(false);
    let actual = run(true);
    (estimated, actual)
}

/// Estimated-mode document insertion has to work end to end against a ranked
/// contract: grovedb's batch estimator understands the indexed `TreeType`s and
/// emits multi-axis `ReplaceAggregateIndexedTreeRootKeys` costs for them, so
/// the dry run must produce fees rather than erroring on an unknown layer.
///
/// The dry run must also not *under*-charge relative to the applied write —
/// worst-case estimation is what the fee system bills against. Note the known
/// gap documented on `estimated_sum_trees_for_value_tree_type`: an indexed
/// element is ~`1 + 33*axes` bytes larger than its non-indexed mirror and no
/// `EstimatedSumTrees` weight slot captures that, so the comparison here is
/// the guard that the remaining headroom still covers it.
#[test]
fn estimated_mode_insert_on_ranked_indexes_produces_fees_and_does_not_undercharge() {
    for (document_type_name, aggregated_property) in [
        ("review", "grade"), // PCPSIT, Avg axis
        ("visit", "guests"), // PCIT, Count axis
        ("tip", "amount"),   // PSIT, Sum axis
    ] {
        let (estimated, actual) =
            estimated_and_actual_insert_fees(document_type_name, aggregated_property);

        assert!(
            estimated.storage_fee > 0,
            "{document_type_name}: estimated storage fee must be non-zero"
        );
        assert!(
            estimated.processing_fee > 0,
            "{document_type_name}: estimated processing fee must be non-zero"
        );

        assert!(
            estimated.storage_fee >= actual.storage_fee,
            "{document_type_name}: estimated storage fee {} is BELOW the applied storage fee {} \
             (short by {}) — the ranked layers under-charge",
            estimated.storage_fee,
            actual.storage_fee,
            actual.storage_fee.saturating_sub(estimated.storage_fee),
        );
        assert!(
            estimated.processing_fee >= actual.processing_fee,
            "{document_type_name}: estimated processing fee {} is BELOW the applied processing \
             fee {} (short by {}) — the ranked layers under-charge",
            estimated.processing_fee,
            actual.processing_fee,
            actual
                .processing_fee
                .saturating_sub(estimated.processing_fee),
        );
    }
}

/// The PSIT arm: a sum-only ranked index ranks its groups by running sum.
#[test]
fn sum_axis_ranks_groups_by_running_sum() {
    let (drive, contract) = setup_restaurants();

    insert_docs(
        &drive,
        &contract,
        "tip",
        "amount",
        &[
            ("alpha", 10),
            ("alpha", 15),
            ("beta", 100),
            ("gamma", 7),
            ("gamma", 8),
            ("gamma", 9),
        ],
    );

    let path = indexed_property_name_tree_path(&contract, "tip");

    let descending = sum_top_k(&drive, &path, 10, true);
    assert_eq!(
        group_keys(&descending),
        vec!["beta", "alpha", "gamma"],
        "descending Sum ranking must be beta(100) > alpha(25) > gamma(24)"
    );
    assert_eq!(
        descending.iter().map(|(s, _)| *s).collect::<Vec<_>>(),
        vec![100, 25, 24]
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Offset pagination through the query surface
// ---------------------------------------------------------------------------
//
// Everything above reads the secondaries straight through grovedb, which is
// the right altitude for a *write-path* suite: it asserts what the insert
// path put on disk without a query grammar in between. The three tests below
// are the exception, and deliberately so — offset pagination is the one
// ranked behaviour whose answer is not fully visible in the entries. "Which
// groups came back" is observable from a raw read; "which *rank* they are"
// only exists as the proof's attested skip count, and that is produced by the
// query layer. So these run the documents through the same write path as
// everything else here and then ask the public ranked query for a page,
// end to end.

/// `SELECT AVG(grade) GROUP BY restaurantId ORDER BY grade DESC LIMIT n
/// OFFSET m` against the fixture, unproven. Returns the page.
fn ranked_avg_page(
    drive: &Drive,
    contract: &DataContract,
    limit: u32,
    offset: u32,
) -> crate::query::RankedPage {
    use crate::query::{DocumentRankedRequest, DocumentRankedResponse};

    let response = drive
        .execute_document_ranked_request(
            DocumentRankedRequest {
                contract,
                document_type: contract
                    .document_type_for_name("review")
                    .expect("review doctype exists"),
                group_by: &[GROUP_PROPERTY.to_string()],
                select: crate::query::SelectProjection::avg("grade"),
                having: &[],
                order_by: &[crate::query::OrderClause {
                    field: "grade".to_string(),
                    ascending: false,
                }],
                where_clauses: &[],
                limit: Some(limit),
                offset: Some(offset),
                has_start_at: false,
                prove: false,
            },
            None,
            platform_version(),
        )
        .expect("the ranked read must succeed");
    match response {
        DocumentRankedResponse::Entries(page) => page,
        DocumentRankedResponse::Proof(_) => unreachable!("prove = false"),
    }
}

/// Prove the same page and verify it, returning the **attested** page —
/// `skipped` here is re-derived by the verifier from the proof's counted
/// subtree commitments, not echoed from the request.
fn verified_ranked_avg_page(
    drive: &Drive,
    contract: &DataContract,
    limit: u32,
    offset: u32,
) -> crate::query::RankedPage {
    use crate::query::drive_document_ranked_query::index_picker::find_ranked_index_for_axis;
    use crate::query::{
        DocumentRankedRequest, DocumentRankedResponse, DriveDocumentRankedQuery, RankedAxis,
    };
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

    let document_type = contract
        .document_type_for_name("review")
        .expect("review doctype exists");
    let response = drive
        .execute_document_ranked_request(
            DocumentRankedRequest {
                contract,
                document_type,
                group_by: &[GROUP_PROPERTY.to_string()],
                select: crate::query::SelectProjection::avg("grade"),
                having: &[],
                order_by: &[crate::query::OrderClause {
                    field: "grade".to_string(),
                    ascending: false,
                }],
                where_clauses: &[],
                limit: Some(limit),
                offset: Some(offset),
                has_start_at: false,
                prove: true,
            },
            None,
            platform_version(),
        )
        .expect("the ranked prove must succeed");
    let proof = match response {
        DocumentRankedResponse::Proof(proof) => proof,
        DocumentRankedResponse::Entries(_) => unreachable!("prove = true"),
    };

    // Rebuild the query the way a client would — off the contract alone.
    let indexes = contract
        .document_types()
        .get("review")
        .expect("review doctype exists")
        .indexes();
    let query = DriveDocumentRankedQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "review".to_string(),
        index: find_ranked_index_for_axis(indexes, GROUP_PROPERTY, &[], RankedAxis::Avg, "grade")
            .expect("the fixture declares rankedAverageable on grade"),
        equality_prefix_values: vec![],
        axis: RankedAxis::Avg,
        descending: true,
        k: limit as u16,
        offset,
    };
    let (root_hash, page) = query
        .verify_ranked_top_k_proof(&proof, platform_version())
        .expect("the proof must verify");
    assert_eq!(
        root_hash,
        drive
            .grove
            .root_hash(None, &platform_version().drive.grove_version)
            .unwrap()
            .expect("root hash must be readable"),
        "the proof must reconstruct the live grovedb root hash"
    );
    page
}

/// Insert the five-restaurant Avg fixture used by the offset tests:
/// `gamma(95) > alpha(85) > beta(60) > delta(30) > epsilon(10)`.
fn insert_five_graded_restaurants(drive: &Drive, contract: &DataContract) {
    insert_docs(
        drive,
        contract,
        "review",
        "grade",
        &[
            ("gamma", 95),
            ("alpha", 90),
            ("alpha", 80),
            ("beta", 60),
            ("delta", 30),
            ("epsilon", 10),
        ],
    );
}

/// **"The 5th best grade"** — `LIMIT 1 OFFSET 4`.
///
/// The entry on its own carries no rank: `epsilon` looks identical whether
/// it was returned as the 5th-best group or as the only group in the index.
/// What makes the answer meaningful is the attested skip, which the proof
/// re-derives from the counted subtree commitments rather than echoing from
/// the request — so a server cannot answer "the 5th best" with a proof of
/// "the best".
#[test]
fn offset_four_limit_one_returns_the_fifth_best_group_with_its_attested_rank() {
    let (drive, contract) = setup_restaurants();
    insert_five_graded_restaurants(&drive, &contract);

    let page = ranked_avg_page(&drive, &contract, 1, 4);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| String::from_utf8(entry.key.clone()).expect("utf-8 group keys"))
            .collect::<Vec<_>>(),
        vec!["epsilon"],
        "gamma > alpha > beta > delta > epsilon — rank 4 (0-based) is epsilon"
    );
    assert_eq!(
        page.entries[0].value,
        crate::query::RankedEntryValue::AvgFixedPoint(expected_avg_fixed_point(10, 1))
    );

    let verified = verified_ranked_avg_page(&drive, &contract, 1, 4);
    assert_eq!(
        verified.entries, page.entries,
        "the proved page must equal the unproven one"
    );
    assert_eq!(
        verified.skipped, 4,
        "the proof attests that exactly four groups outrank this one"
    );

    assert_grovedb_is_consistent(&drive);
}

/// A window that runs off the end of the ranking returns the tail, short —
/// the same contract an unpaginated `LIMIT` larger than the group count has.
/// `skipped` is still the requested offset, because the *skip* succeeded;
/// only the take came up short.
#[test]
fn an_offset_window_spanning_the_end_returns_the_short_tail() {
    let (drive, contract) = setup_restaurants();
    insert_five_graded_restaurants(&drive, &contract);

    let page = ranked_avg_page(&drive, &contract, 4, 3);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| String::from_utf8(entry.key.clone()).expect("utf-8 group keys"))
            .collect::<Vec<_>>(),
        vec!["delta", "epsilon"],
        "four were asked for from rank 3, and only two exist"
    );

    let verified = verified_ranked_avg_page(&drive, &contract, 4, 3);
    assert_eq!(verified.entries, page.entries);
    assert_eq!(
        verified.skipped, 3,
        "three groups really were skipped, so the attested skip is the requested offset"
    );

    assert_grovedb_is_consistent(&drive);
}

/// **Paging past the end is a provable answer, not an error.**
///
/// The page comes back empty and `skipped` collapses below the requested
/// offset — and *that shape* is the proof that the ranking holds exactly
/// `skipped` groups in total, because the counted commitments cover the whole
/// walk. It is the only way this surface reports a population, and both paths
/// now report it: grovedb's counted descent tracks how far the skip got and
/// returns it on the page, so the unproven read no longer has to echo the
/// request back. What proving still adds is that the number is attested.
#[test]
fn an_offset_past_the_end_returns_an_empty_page_whose_skip_attests_the_population() {
    let (drive, contract) = setup_restaurants();
    insert_five_graded_restaurants(&drive, &contract);

    let page = ranked_avg_page(&drive, &contract, 3, 12);
    assert!(
        page.entries.is_empty(),
        "there is no rank 12 in a five-group ranking"
    );
    assert_eq!(
        page.skipped, 5,
        "the unproven read reports the five groups the ranking holds, not the requested \
         offset of 12"
    );

    let verified = verified_ranked_avg_page(&drive, &contract, 3, 12);
    assert!(verified.entries.is_empty());
    assert_eq!(
        verified.skipped, 5,
        "skipped < offset with an empty page is a proof that the ranking holds exactly \
         five groups"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// A ranked index sharing its property with a compound index
// ---------------------------------------------------------------------------

/// A `dish` doctype whose ranked single-property index `[restaurantId]`
/// shares its property with a plain compound index `[restaurantId, chefId]`.
///
/// This is the shape both v14 changes meet on. The compound index hangs a
/// `chefId` continuation property-name tree inside the very value trees whose
/// (count, sum) the ranked Avg secondary orders by, so:
///
/// * the **shared-prefix fix** decides the value trees: with a continuation
///   present they demote from `ProvableCountProvableSumTree` to
///   `CountSumTree`, and the continuation goes in `Element::NonCounted` so it
///   contributes zero to the group's count and sum;
/// * the **ranked upgrade** decides the property-name tree one level up: it
///   stays the `ProvableCountProvableSumIndexedTree` carrying the Avg axis.
///
/// Before v14 this contract registered but rejected every document insert
/// (the diagonal-only wrapper matrix had no legal element for a plain
/// continuation under a count+sum value tree), which is why the ranked
/// multi-index fixture in `query::drive_document_ranked_query::tests` had to
/// hang its compound sibling off a count-only index instead.
fn build_shared_prefix_ranked_dish_contract() -> DataContract {
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::platform_value;
    use dpp::tests::utils::generate_random_identifier_struct;

    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V14).expect("expected to create factory");

    let schemas = platform_value!({
        "dish": {
            "type": "object",
            "documentsMutable": true,
            "canBeDeleted": true,
            "properties": {
                "restaurantId": {"type": "string", "position": 0, "maxLength": 32},
                "chefId": {"type": "string", "position": 1, "maxLength": 32},
                "grade": {"type": "integer", "minimum": 0, "maximum": 100, "position": 2},
            },
            "required": ["restaurantId", "chefId", "grade"],
            "indices": [
                {
                    "name": "byRestaurant",
                    "properties": [{"restaurantId": "asc"}],
                    "countable": "countable",
                    "summable": "grade",
                    "averageable": "grade",
                    "rangeCountable": true,
                    "rangeSummable": true,
                    "rangeAverageable": true,
                    "rankedAverageable": true,
                },
                {
                    "name": "byRestaurantChef",
                    "properties": [{"restaurantId": "asc"}, {"chefId": "asc"}],
                },
            ],
            "additionalProperties": false,
        }
    });

    factory
        .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
        .expect("expected to build the shared-prefix ranked contract")
        .data_contract_owned()
}

/// Build a `dish` document with the three properties the shared-prefix
/// contract declares.
fn build_dish(
    contract: &DataContract,
    restaurant: &str,
    chef: &str,
    grade: i64,
    seed: u64,
) -> Document {
    let document_type = contract
        .document_type_for_name("dish")
        .expect("dish doctype exists");
    let mut doc = document_type
        .random_document(Some(seed), platform_version())
        .expect("random document");
    let mut props = std::collections::BTreeMap::new();
    props.insert(
        GROUP_PROPERTY.to_string(),
        Value::Text(restaurant.to_string()),
    );
    props.insert("chefId".to_string(), Value::Text(chef.to_string()));
    props.insert("grade".to_string(), Value::I64(grade));
    doc.set_properties(props);
    doc
}

/// The two v14 fixes on the same doctype: ranking must stay exact while a
/// compound index hangs continuations inside the ranked groups.
///
/// The assertions that would break if either fix regressed:
/// - inserts succeed at all (pre-v14 they did not for this shape);
/// - the Avg ranking matches the averages computed from the documents alone,
///   i.e. the `chefId` continuations contribute nothing to any group;
/// - the group's value tree is the demoted `CountSumTree` carrying exactly
///   the documents' (count, sum), with the continuation `NonCounted`-wrapped;
/// - the property-name tree above it is still the indexed element;
/// - grovedb's integrity sweep — which walks primary against secondary for
///   every axis — reports nothing.
#[test]
fn ranked_index_ranks_correctly_next_to_a_compound_index_sharing_its_property() {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = build_shared_prefix_ranked_dish_contract();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version(),
        )
        .expect("expected to apply the shared-prefix ranked contract");

    // The ranked upgrade survives the presence of the compound sibling.
    let element = read_indexed_property_name_element(&drive, &contract, "dish");
    match &element {
        Element::ProvableCountProvableSumIndexedTree(_, _, _, axes, _) => {
            assert_eq!(
                axes,
                &vec![(2u8, None)],
                "the terminal property-name tree must carry exactly the Avg axis"
            );
        }
        other => panic!("expected a ProvableCountProvableSumIndexedTree, got {other:?}"),
    }

    // alpha (90 + 80)/2 = 85, beta (60 + 70 + 50)/3 = 60, gamma 95/1 = 95.
    // Two different chefs per restaurant, so every group really does host a
    // populated `chefId` continuation.
    let rows = [
        ("alpha", "chefA", 90),
        ("alpha", "chefB", 80),
        ("beta", "chefA", 60),
        ("beta", "chefB", 70),
        ("beta", "chefC", 50),
        ("gamma", "chefC", 95),
    ];
    let docs: Vec<Document> = rows
        .iter()
        .enumerate()
        .map(|(i, (restaurant, chef, grade))| {
            let doc = build_dish(&contract, restaurant, chef, *grade, i as u64 + 1);
            insert_doc(&drive, &contract, "dish", &doc);
            doc
        })
        .collect();

    let path = indexed_property_name_tree_path(&contract, "dish");
    let ranking = avg_top_k(&drive, &path, 10, true);
    assert_eq!(
        group_keys(&ranking),
        vec!["gamma", "alpha", "beta"],
        "Avg ranking must be gamma(95) > alpha(85) > beta(60)"
    );
    assert_eq!(
        ranking.iter().map(|(avg, _)| *avg).collect::<Vec<_>>(),
        vec![
            expected_avg_fixed_point(95, 1),
            expected_avg_fixed_point(170, 2),
            expected_avg_fixed_point(180, 3),
        ],
        "the continuations must contribute nothing to any group's (count, sum)"
    );

    // The group value tree itself: demoted to CountSumTree, carrying only the
    // documents' aggregates.
    let alpha = read_grove_element(&drive, &path, b"alpha").expect("alpha's group must exist");
    match &alpha {
        Element::CountSumTree(_, count, sum, _) => {
            assert_eq!(*count, 2, "alpha's count must be its two dishes");
            assert_eq!(*sum, 170, "alpha's sum must be 90 + 80");
        }
        other => panic!("expected alpha's value tree to be a demoted CountSumTree, got {other:?}"),
    }

    // ...and the compound index's continuation lives inside it, wrapped so it
    // contributes zero to both axes.
    let mut alpha_path = path.clone();
    alpha_path.push(b"alpha".to_vec());
    let continuation = read_grove_element(&drive, &alpha_path, b"chefId")
        .expect("the compound index's continuation must exist under alpha");
    assert!(
        matches!(&continuation, Element::NonCounted(inner) if matches!(inner.as_ref(), Element::Tree(..))),
        "the continuation must be NonCounted-wrapped, got {continuation:?}"
    );

    // A key-changing update runs the v1 update walker over the same shape.
    let document_type = contract
        .document_type_for_name("dish")
        .expect("dish doctype exists");
    let mut moved = docs[5].clone();
    let mut props = moved.properties().clone();
    props.insert("chefId".to_string(), Value::Text("chefD".to_string()));
    props.insert("grade".to_string(), Value::I64(20));
    moved.set_properties(props);
    moved.set_revision(Some(2));
    drive
        .update_document_for_contract(
            &moved,
            &contract,
            document_type,
            Some(moved.owner_id().to_buffer()),
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version(),
            None,
        )
        .expect("expected to update the dish document across the compound index");

    assert_eq!(
        group_keys(&avg_top_k(&drive, &path, 10, true)),
        vec!["alpha", "beta", "gamma"],
        "gamma's only dish dropped to 20 and must fall to last"
    );

    // Deleting alpha's dishes drains the group without leaving its
    // continuation behind.
    for doc in docs.iter().take(2) {
        drive
            .delete_document_for_contract(
                doc.id(),
                &contract,
                "dish",
                BlockInfo::default(),
                true,
                None,
                platform_version(),
                None,
            )
            .expect("expected to delete an alpha dish");
    }
    assert_eq!(
        group_keys(&avg_top_k(&drive, &path, 10, true)),
        vec!["beta", "gamma"],
        "the drained group must leave the axis"
    );
    assert!(
        read_grove_element(&drive, &path, b"alpha").is_none(),
        "the drained group's value tree must be gone from the primary too"
    );

    assert_grovedb_is_consistent(&drive);
}
